// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [065A-5] Servicio de sincronización Glory → BDP WebLink REST API.
 * Crea comandas reales en el TPV cuando se registra una venta en Glory.
 * Usa exclusión local/distribuida y una única escritura con reconciliación;
 * nunca reintenta CreateOrder a ciegas.
 *
 * Flujo: VentaService::create/update → spawn_bdp_sync → BdpSyncService::sync_venta
 *   1. Login a BDP WebLink
 *   2. Construir Order (Type=0 Barra, OrderEndType=1 pendiente)
 *   3. CreateOrder (OperationType=0 escritura real)
 *   4. Actualizar bdp_synced / bdp_order_id en la venta
 *
 * Mapeo Glory → BDP: usa bdp_default_article_code configurado.
 * Glory ventas son monolíticas (1 descripción + total), BDP requiere líneas de artículos.
 * Por defecto, toda venta se envía como 1 artículo genérico configurable.
 *
 * Gotchas documentados:
 * - Type=0 (Barra) es el único que pasa validación en POS 31. Type=1 falla 300008, Type=2 falla 300009.
 * - OrderEndType=1 crea comanda pendiente (no facturada, no impresa). El TPV la muestra en autocomanda.
 * - MarketplaceOrderId max 15 chars (error 301011).
 * - AlreadyInvoiced e Invoice son campos REQUERIDOS dentro de Order.
 * - CancelOrder devuelve "Subscripción no activada" — no se puede cancelar vía API.
 * - Serie 00031TI (IVA incluido) configurada en POS 31 desde 2026-06-07. */

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex as StdMutex};

use tokio::sync::Mutex as TokioMutex;
use uuid::Uuid;

use crate::models::ConfiguracionRestaurante;
use crate::services::bdp_weblink::BdpWeblinkClient;

pub(crate) const BDP_SYNC_MARKET_ID: i32 = 9_900;
/* [F3.1] Contexto resuelto para construir el pedido BDP.
 * Se resuelve en sync_venta() y se pasa a build_order() para no hacer
 * lookups dentro de la función de construcción del payload. */
pub(crate) struct OrderContext {
    pub(crate) tender_id: Option<i32>,
    pub(crate) order_type: i32,
    pub(crate) customer_code: Option<String>,
    pub(crate) customer_name: Option<String>,
    pub(crate) customer_phone: Option<String>,
}

pub(crate) static SYNC_LOCKS: LazyLock<StdMutex<HashMap<uuid::Uuid, Arc<TokioMutex<()>>>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));

pub struct BdpSyncService;

/* [R14] Guard RAII que limpia el lock de SYNC_LOCKS al salir del scope.
 * Evita llamadas manuales a cleanup_lock en cada camino de retorno. */
pub(crate) struct SyncLockGuard {
    venta_id: Uuid,
}

impl SyncLockGuard {
    pub(crate) fn new(venta_id: Uuid) -> Self {
        Self { venta_id }
    }
}

impl Drop for SyncLockGuard {
    fn drop(&mut self) {
        BdpSyncService::cleanup_lock(self.venta_id);
    }
}

impl BdpSyncService {
    /* ===== Infra BDP compartida (pago/factura/catalogo) ===== */

    /* [F3.2] Login BDP compartido por pago y factura. */
    pub(crate) async fn conectar_bdp(
        config: &ConfiguracionRestaurante,
    ) -> Result<BdpWeblinkClient<'_>, String> {
        let client = BdpWeblinkClient::new(config);
        client
            .login()
            .await
            .map_err(|e| format!("Error login BDP: {e}"))?;
        Ok(client)
    }
}

/// Artículo BDP resuelto para el mapeo.
pub(crate) struct ResolvedArticle {
    pub(crate) id: i64,
    pub(crate) name: String,
    #[allow(dead_code)]
    pub(crate) price: f64,
    #[allow(dead_code)]
    pub(crate) vat_pct: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    /* [F4-split] El split del 12-09 movió Venta/VentaLinea, el repositorio de
     * artículos y chrono::Utc a los submódulos, así que `use super::*` ya no
     * los trae: se importan aquí para que los tests sigan compilando. */
    use crate::models::{CrearBdpArticleMapRequest, Venta, VentaLinea};
    use crate::repositories::BdpArticleMapRepository;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn test_config() -> ConfiguracionRestaurante {
        ConfiguracionRestaurante {
            id: uuid::Uuid::new_v4(),
            user_id: uuid::Uuid::new_v4(),
            bdp_base_url: "http://localhost:8068".into(),
            bdp_login: "admin".into(),
            bdp_password: "pass".into(),
            bdp_integrator_code: "TEST1234".into(),
            bdp_sync_enabled: true,
            bdp_pos_id: 31,
            bdp_employee_id: 1,
            bdp_items_profile_id: 1,
            bdp_catalog_price_type: 1,
            bdp_purchase_notes_profile_id: None,
            bdp_default_article_code: "1001".into(),
            bdp_default_article_name: "CAFE BOMBON".into(),
            bdp_tender_map: serde_json::json!({"efectivo": "1", "tarjeta": "2"}),
            bdp_order_type_map: serde_json::json!({"comedor": "0", "barra": "0"}),
            bdp_default_customer_code: "GENERIC".into(),
            ..Default::default()
        }
    }

    fn test_order_ctx() -> OrderContext {
        OrderContext {
            tender_id: Some(1),
            order_type: 0,
            customer_code: None,
            customer_name: None,
            customer_phone: None,
        }
    }

    fn test_venta() -> Venta {
        Venta {
            id: uuid::Uuid::new_v4(),
            user_id: uuid::Uuid::new_v4(),
            fecha: chrono::NaiveDate::from_ymd_opt(2026, 6, 7).unwrap(),
            comensales: Some(2),
            descripcion: "Cena para 2".into(),
            iva_porcentaje: Decimal::from_str("10.0").unwrap(),
            turno: "cena".into(),
            canal: "comedor".into(),
            metodo_pago: "efectivo".into(),
            importe_base: Decimal::from_str("25.00").unwrap(),
            importe_iva: Decimal::from_str("2.50").unwrap(),
            reserva_id: None,
            cliente_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            haddock_synced: false,
            haddock_synced_at: None,
            haddock_sync_error: None,
            bdp_synced: false,
            bdp_synced_at: None,
            bdp_sync_error: None,
            bdp_order_id: None,
            bdp_order_status: None,
            bdp_invoiced: false,
            anulada: false,
            anulada_at: None,
            anulacion_motivo: None,
            anulacion_usuario: None,
            facturada_local: false,
            factura_numero: None,
            factura_fecha: None,
            propina: Decimal::ZERO,
        }
    }

    #[test]
    fn build_order_uses_venta_total() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE BOMBON".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let order =
            BdpSyncService::build_order(&config, &venta, &article, None, None, &test_order_ctx());
        let order_json = &order.order;

        /* El total debe ser importe_base + importe_iva = 27.50 */
        let total = order_json
            .get("Total")
            .and_then(serde_json::Value::as_f64)
            .unwrap();
        assert!((total - 27.5).abs() < 0.01, "Expected ~27.5, got {total}");

        /* Item[0].Price debe ser el total de la venta */
        let item_price = order_json
            .get("Items")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|i| i.get("Price"))
            .and_then(serde_json::Value::as_f64)
            .unwrap();
        assert!((item_price - 27.5).abs() < 0.01);

        /* Type=0 (Barra) */
        let tipo = order_json
            .get("Type")
            .and_then(serde_json::Value::as_i64)
            .unwrap();
        assert_eq!(tipo, 0);

        /* OrderEndType=1 (pendiente) */
        assert_eq!(order.order_end_type, 1);

        /* OrderOperationType=0 (escritura real) */
        assert_eq!(order.order_operation_type, 0);

        /* MarketplaceOrderId <= 15 chars */
        let mid = order_json
            .get("MarketplaceOrderId")
            .and_then(|v| v.as_str())
            .unwrap();
        assert!(mid.len() <= 15, "MarketplaceOrderId too long: {mid}");
    }

    #[test]
    fn build_order_uses_configured_employee_and_pos() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE BOMBON".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let order =
            BdpSyncService::build_order(&config, &venta, &article, None, None, &test_order_ctx());
        assert_eq!(order.employee_id, 1);
        assert_eq!(order.items_profile_id, 1);

        let pos_id = order
            .order
            .get("PosId")
            .and_then(serde_json::Value::as_i64)
            .unwrap();
        assert_eq!(pos_id, 31);
    }

    #[test]
    fn build_order_uses_venta_description_as_item_name() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE BOMBON".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let order =
            BdpSyncService::build_order(&config, &venta, &article, None, None, &test_order_ctx());
        let item_name = order
            .order
            .get("Items")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|i| i.get("Name"))
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(item_name, "Cena para 2");
    }

    #[test]
    fn build_order_multi_item_with_lineas() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE BOMBON".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let lineas = vec![
            VentaLinea {
                id: uuid::Uuid::new_v4(),
                venta_id: venta.id,
                articulo_codigo: "1001".into(),
                descripcion: "Café bombón".into(),
                cantidad: Decimal::from_str("2").unwrap(),
                precio_unitario: Decimal::from_str("5.00").unwrap(),
                iva_pct: Decimal::from_str("10.0").unwrap(),
                descuento: Decimal::from_str("0.00").unwrap(),
                created_at: Utc::now(),
            },
            VentaLinea {
                id: uuid::Uuid::new_v4(),
                venta_id: venta.id,
                articulo_codigo: "2002".into(),
                descripcion: "Tostada".into(),
                cantidad: Decimal::from_str("1").unwrap(),
                precio_unitario: Decimal::from_str("3.50").unwrap(),
                iva_pct: Decimal::from_str("10.0").unwrap(),
                descuento: Decimal::from_str("0.50").unwrap(),
                created_at: Utc::now(),
            },
        ];

        let order = BdpSyncService::build_order(
            &config,
            &venta,
            &article,
            Some(&lineas),
            Some(&[1001, 2002]),
            &test_order_ctx(),
        );
        let items = order.order.get("Items").and_then(|v| v.as_array()).unwrap();

        /* Debe tener 2 items */
        assert_eq!(items.len(), 2, "Expected 2 items, got {}", items.len());

        /* Primer item: Café bombón x2 */
        assert_eq!(items[0].get("Lin").unwrap(), 1);
        assert_eq!(items[0].get("Id").unwrap().as_i64().unwrap(), 1001);
        assert_eq!(
            items[0].get("Name").unwrap().as_str().unwrap(),
            "Café bombón"
        );
        assert!((items[0].get("Units").unwrap().as_f64().unwrap() - 2.0).abs() < 0.01);
        assert!((items[0].get("Price").unwrap().as_f64().unwrap() - 5.0).abs() < 0.01);
        assert!((items[0].get("Total").unwrap().as_f64().unwrap() - 10.0).abs() < 0.01);

        /* Segundo item: Tostada x1, descuento 0.50 → total = 3.50 - 0.50 = 3.00 */
        assert_eq!(items[1].get("Lin").unwrap(), 2);
        assert_eq!(items[1].get("Id").unwrap().as_i64().unwrap(), 2002);
        assert_eq!(items[1].get("Name").unwrap().as_str().unwrap(), "Tostada");
        assert!((items[1].get("Units").unwrap().as_f64().unwrap() - 1.0).abs() < 0.01);
        assert!((items[1].get("Discount").unwrap().as_f64().unwrap() - 0.50).abs() < 0.01);
        assert!((items[1].get("Total").unwrap().as_f64().unwrap() - 3.00).abs() < 0.01);

        /* VatPct por línea */
        assert!((items[0].get("VatPct").unwrap().as_f64().unwrap() - 10.0).abs() < 0.01);
        assert!((items[1].get("VatPct").unwrap().as_f64().unwrap() - 10.0).abs() < 0.01);

        /* MarketplaceOrderId <= 15 chars */
        let mid = order
            .order
            .get("MarketplaceOrderId")
            .and_then(|v| v.as_str())
            .unwrap();
        assert!(mid.len() <= 15, "MarketplaceOrderId too long: {mid}");
    }

    #[test]
    fn extract_first_article_parses_bdp_response() {
        let json = serde_json::json!({
            "ArticlesListData": [{
                "ArtCode": 1001,
                "ArtDescription": "CAFE BOMBON",
                "Price1": 5.0,
                "TAVPer": 10.0
            }],
            "ErrorMessage": ""
        });

        let article = BdpSyncService::extract_first_article(&json, 10.0).unwrap();
        assert_eq!(article.id, 1001);
        assert_eq!(article.name, "CAFE BOMBON");
        assert!((article.price - 5.0).abs() < 0.01);
    }

    /* [R12] Test: extract_first_article usa default_iva_pct cuando BDP no devuelve TAVPer */
    #[test]
    fn extract_first_article_uses_default_iva_when_missing() {
        let json = serde_json::json!({
            "ArticlesListData": [{
                "ArtCode": 1002,
                "ArtDescription": "SIN IVA",
                "Price1": 3.0
            }]
        });

        let article = BdpSyncService::extract_first_article(&json, 21.0).unwrap();
        assert!(
            (article.vat_pct - 21.0).abs() < f64::EPSILON,
            "Debe usar default_iva_pct pasado como parámetro"
        );
    }

    #[test]
    fn sanitize_error_classifies_known_codes() {
        assert!(BdpSyncService::sanitize_error("300035 series").contains("serie"));
        assert!(BdpSyncService::sanitize_error("300008 salón").contains("salón"));
        assert!(BdpSyncService::sanitize_error("301011 too long").contains("MarketplaceOrderId"));
        assert!(BdpSyncService::sanitize_error("301400 caja").contains("caja"));
        assert!(BdpSyncService::sanitize_error("401 Unauthorized").contains("autenticación"));
    }

    /* [F3.2] Test: TenderId se mapea desde metodo_pago via bdp_tender_map */
    #[test]
    fn build_order_maps_tender_id_from_metodo_pago() {
        let config = test_config();
        let venta = test_venta(); /* metodo_pago = "efectivo" */
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let ctx = OrderContext {
            tender_id: Some(1), /* efectivo → 1 */
            order_type: 0,
            customer_code: None,
            customer_name: None,
            customer_phone: None,
        };
        let order = BdpSyncService::build_order(&config, &venta, &article, None, None, &ctx);
        let tender = order
            .order
            .get("TenderId")
            .and_then(serde_json::Value::as_i64)
            .unwrap();
        assert_eq!(tender, 1, "TenderId should be 1 for efectivo");
    }

    /* [F3.2] Test: TenderId no se incluye si es None */
    #[test]
    fn build_order_no_tender_when_none() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let ctx = OrderContext {
            tender_id: None,
            order_type: 0,
            customer_code: None,
            customer_name: None,
            customer_phone: None,
        };
        let order = BdpSyncService::build_order(&config, &venta, &article, None, None, &ctx);
        assert!(
            order.order.get("TenderId").is_none(),
            "TenderId should not be present"
        );
    }

    /* [169A-2] Test: con tender y total positivo, Payments va dentro del
     * CreateOrder y la petición pide factura (pago total). */
    #[test]
    fn build_order_con_tender_incluye_payments_e_invoice_true() {
        let config = test_config();
        let venta = test_venta(); /* base 25 + iva 2.5 = total 27.5 */
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let ctx = OrderContext {
            tender_id: Some(2),
            order_type: 0,
            customer_code: None,
            customer_name: None,
            customer_phone: None,
        };
        let order = BdpSyncService::build_order(&config, &venta, &article, None, None, &ctx);
        let payments = order
            .order
            .get("Payments")
            .and_then(serde_json::Value::as_array)
            .expect("Payments should be present");
        assert_eq!(payments.len(), 1, "un solo pago total");
        assert_eq!(
            payments[0]
                .get("TenderId")
                .and_then(serde_json::Value::as_i64),
            Some(2)
        );
        assert_eq!(
            payments[0]
                .get("Amount")
                .and_then(serde_json::Value::as_f64),
            Some(27.5)
        );
        assert!(
            payments[0]
                .get("PaymentId")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| !id.is_empty()),
            "PaymentId identificador no vacío"
        );
        assert_eq!(order.invoice, Some(true), "pago total => pide factura");
    }

    /* [169A-2] Test: sin tender no hay Payments y no se pide factura
     * (comportamiento anterior intacto para comandas no cobradas). */
    #[test]
    fn build_order_sin_tender_sin_payments_e_invoice_false() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let ctx = OrderContext {
            tender_id: None,
            order_type: 0,
            customer_code: None,
            customer_name: None,
            customer_phone: None,
        };
        let order = BdpSyncService::build_order(&config, &venta, &article, None, None, &ctx);
        assert!(
            order.order.get("Payments").is_none(),
            "Payments should not be present"
        );
        assert_eq!(order.invoice, Some(false));
    }

    /* [169A-2] Test: propina positiva va en Tip; cero no pinta la clave. */
    #[test]
    fn build_order_con_propina_incluye_tip() {
        let config = test_config();
        let mut venta = test_venta();
        venta.propina = Decimal::from_str("1.50").unwrap();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let order =
            BdpSyncService::build_order(&config, &venta, &article, None, None, &test_order_ctx());
        assert_eq!(
            order.order.get("Tip").and_then(serde_json::Value::as_f64),
            Some(1.5)
        );

        let venta_sin = test_venta(); /* propina ZERO */
        let order_sin = BdpSyncService::build_order(
            &config,
            &venta_sin,
            &article,
            None,
            None,
            &test_order_ctx(),
        );
        assert!(
            order_sin.order.get("Tip").is_none(),
            "Tip should not be present without propina"
        );
    }

    /* [F3.3] Test: Order type se mapea desde canal */
    #[test]
    fn build_order_uses_order_type_from_canal() {
        let config = test_config();
        let venta = test_venta(); /* canal = "comedor" */
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let ctx = OrderContext {
            tender_id: None,
            order_type: 0, /* comedor → 0 */
            customer_code: None,
            customer_name: None,
            customer_phone: None,
        };
        let order = BdpSyncService::build_order(&config, &venta, &article, None, None, &ctx);
        let tipo = order
            .order
            .get("Type")
            .and_then(serde_json::Value::as_i64)
            .unwrap();
        assert_eq!(tipo, 0);
    }

    /* [F3.1] Test: Customer se incluye cuando hay nombre */
    #[test]
    fn build_order_includes_customer_when_present() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let ctx = OrderContext {
            tender_id: Some(1),
            order_type: 0,
            customer_code: Some("GENERIC".into()),
            customer_name: Some("Juan Pérez".into()),
            customer_phone: Some("600123456".into()),
        };
        let order = BdpSyncService::build_order(&config, &venta, &article, None, None, &ctx);
        let customer = order.order.get("Customer").unwrap();
        assert_eq!(
            customer.get("Name").unwrap().as_str().unwrap(),
            "Juan Pérez"
        );
        assert_eq!(
            customer.get("Phone").unwrap().as_str().unwrap(),
            "600123456"
        );
        assert_eq!(customer.get("Code").unwrap().as_str().unwrap(), "GENERIC");
    }

    /* [F3.1] Test: Customer NO se incluye cuando nombre es None */
    #[test]
    fn build_order_no_customer_when_name_none() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let ctx = OrderContext {
            tender_id: Some(1),
            order_type: 0,
            customer_code: None,
            customer_name: None,
            customer_phone: None,
        };
        let order = BdpSyncService::build_order(&config, &venta, &article, None, None, &ctx);
        assert!(
            order.order.get("Customer").is_none(),
            "Customer should not be present"
        );
    }

    /* [F3.2-3.3] Test: resolve_tender_id y resolve_order_type helpers */
    #[test]
    fn resolve_tender_id_maps_metodo_pago() {
        let config = test_config();
        let mut venta = test_venta();

        venta.metodo_pago = "efectivo".into();
        assert_eq!(BdpSyncService::resolve_tender_id(&venta, &config), Some(1));

        venta.metodo_pago = "tarjeta".into();
        assert_eq!(BdpSyncService::resolve_tender_id(&venta, &config), Some(2));

        venta.metodo_pago = "bizum".into(); /* no está en el map */
        assert_eq!(BdpSyncService::resolve_tender_id(&venta, &config), None);
    }

    #[test]
    fn resolve_order_type_maps_canal() {
        let config = test_config();
        let mut venta = test_venta();

        venta.canal = "comedor".into();
        assert_eq!(BdpSyncService::resolve_order_type(&venta, &config), 0);

        venta.canal = "barra".into();
        assert_eq!(BdpSyncService::resolve_order_type(&venta, &config), 0);

        venta.canal = "delivery".into(); /* no configurado, default 0 */
        assert_eq!(BdpSyncService::resolve_order_type(&venta, &config), 0);
    }

    /* [BDP-TEST-A] Tests adicionales: edge cases de build_order */

    /* build_order con Some(&[]) (vec vacío) debe producir Items vacío,
     * NO caer al fallback legacy. Esto valida que el codegen distingue
     * Some(vec![]) de None correctamente. */
    #[test]
    fn build_order_con_0_lineas_produce_items_vacio() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE BOMBON".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let lineas_vacias: Vec<VentaLinea> = vec![];
        let order = BdpSyncService::build_order(
            &config,
            &venta,
            &article,
            Some(&lineas_vacias),
            Some(&[]),
            &test_order_ctx(),
        );
        let items = order.order.get("Items").and_then(|v| v.as_array()).unwrap();
        assert_eq!(
            items.len(),
            0,
            "Some(vec![]) should produce empty Items array"
        );
    }

    /* build_order con None cae al fallback legacy: 1 item genérico */
    #[test]
    fn build_order_con_none_produce_fallback_legacy() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE BOMBON".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let order =
            BdpSyncService::build_order(&config, &venta, &article, None, None, &test_order_ctx());
        let items = order.order.get("Items").and_then(|v| v.as_array()).unwrap();
        assert_eq!(items.len(), 1, "None should produce 1 fallback item");
        assert_eq!(
            items[0].get("Name").unwrap().as_str().unwrap(),
            "Cena para 2",
            "Fallback item Name should be venta.descripcion"
        );
    }

    /* build_order con 1 línea explícita: un solo item custom */
    #[test]
    fn build_order_con_1_linea_explicita() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE BOMBON".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let lineas = vec![VentaLinea {
            id: uuid::Uuid::new_v4(),
            venta_id: venta.id,
            articulo_codigo: "5001".into(),
            descripcion: "Ensalada César".into(),
            cantidad: Decimal::from_str("1").unwrap(),
            precio_unitario: Decimal::from_str("8.50").unwrap(),
            iva_pct: Decimal::from_str("10.0").unwrap(),
            descuento: Decimal::from_str("0.00").unwrap(),
            created_at: Utc::now(),
        }];

        let order = BdpSyncService::build_order(
            &config,
            &venta,
            &article,
            Some(&lineas),
            Some(&[5001]),
            &test_order_ctx(),
        );
        let items = order.order.get("Items").and_then(|v| v.as_array()).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].get("Id").unwrap().as_i64().unwrap(), 5001);
        assert_eq!(
            items[0].get("Name").unwrap().as_str().unwrap(),
            "Ensalada César"
        );
        assert!((items[0].get("Total").unwrap().as_f64().unwrap() - 8.50).abs() < 0.01);
    }

    /* build_order con 3 líneas: valida Lin secuencial y totales individuales */
    #[test]
    fn build_order_con_3_lineas() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE BOMBON".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let lineas = vec![
            VentaLinea {
                id: uuid::Uuid::new_v4(),
                venta_id: venta.id,
                articulo_codigo: "1001".into(),
                descripcion: "Café bombón".into(),
                cantidad: Decimal::from_str("2").unwrap(),
                precio_unitario: Decimal::from_str("5.00").unwrap(),
                iva_pct: Decimal::from_str("10.0").unwrap(),
                descuento: Decimal::from_str("0.00").unwrap(),
                created_at: Utc::now(),
            },
            VentaLinea {
                id: uuid::Uuid::new_v4(),
                venta_id: venta.id,
                articulo_codigo: "2002".into(),
                descripcion: "Tostada".into(),
                cantidad: Decimal::from_str("1").unwrap(),
                precio_unitario: Decimal::from_str("3.50").unwrap(),
                iva_pct: Decimal::from_str("10.0").unwrap(),
                descuento: Decimal::from_str("0.50").unwrap(),
                created_at: Utc::now(),
            },
            VentaLinea {
                id: uuid::Uuid::new_v4(),
                venta_id: venta.id,
                articulo_codigo: "3003".into(),
                descripcion: "Zumo naranja".into(),
                cantidad: Decimal::from_str("3").unwrap(),
                precio_unitario: Decimal::from_str("2.00").unwrap(),
                iva_pct: Decimal::from_str("10.0").unwrap(),
                descuento: Decimal::from_str("0.00").unwrap(),
                created_at: Utc::now(),
            },
        ];

        let order = BdpSyncService::build_order(
            &config,
            &venta,
            &article,
            Some(&lineas),
            Some(&[1001, 2002, 3003]),
            &test_order_ctx(),
        );
        let items = order.order.get("Items").and_then(|v| v.as_array()).unwrap();

        assert_eq!(items.len(), 3, "Expected 3 items");

        /* Lin secuencial: 1, 2, 3 */
        assert_eq!(items[0].get("Lin").unwrap(), 1);
        assert_eq!(items[1].get("Lin").unwrap(), 2);
        assert_eq!(items[2].get("Lin").unwrap(), 3);

        /* Tercer item: Zumo x3 = 6.00 */
        assert_eq!(items[2].get("Id").unwrap().as_i64().unwrap(), 3003);
        assert_eq!(
            items[2].get("Name").unwrap().as_str().unwrap(),
            "Zumo naranja"
        );
        assert!((items[2].get("Units").unwrap().as_f64().unwrap() - 3.0).abs() < 0.01);
        assert!((items[2].get("Total").unwrap().as_f64().unwrap() - 6.00).abs() < 0.01);
    }

    /* build_order con descuento parcial: descuento se refleja en Total */
    #[test]
    fn build_order_linea_con_descuento_parcial() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE BOMBON".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let lineas = vec![VentaLinea {
            id: uuid::Uuid::new_v4(),
            venta_id: venta.id,
            articulo_codigo: "9999".into(),
            descripcion: "Menú del día".into(),
            cantidad: Decimal::from_str("2").unwrap(),
            precio_unitario: Decimal::from_str("12.00").unwrap(),
            iva_pct: Decimal::from_str("10.0").unwrap(),
            descuento: Decimal::from_str("4.00").unwrap(),
            created_at: Utc::now(),
        }];

        let order = BdpSyncService::build_order(
            &config,
            &venta,
            &article,
            Some(&lineas),
            Some(&[9999]),
            &test_order_ctx(),
        );
        let items = order.order.get("Items").and_then(|v| v.as_array()).unwrap();

        /* Total = (12.00 * 2) - 4.00 = 20.00 */
        assert!((items[0].get("Total").unwrap().as_f64().unwrap() - 20.00).abs() < 0.01);
        assert!((items[0].get("Discount").unwrap().as_f64().unwrap() - 4.00).abs() < 0.01);
    }

    /* build_order sin line_article_ids usa article.id como fallback */
    #[test]
    fn build_order_linea_sin_article_ids_usa_fallback() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 7777,
            name: "DEFAULT".into(),
            price: 1.0,
            vat_pct: 10.0,
        };

        let lineas = vec![VentaLinea {
            id: uuid::Uuid::new_v4(),
            venta_id: venta.id,
            articulo_codigo: "XXXX".into(),
            descripcion: "Sin mapeo".into(),
            cantidad: Decimal::from_str("1").unwrap(),
            precio_unitario: Decimal::from_str("3.00").unwrap(),
            iva_pct: Decimal::from_str("10.0").unwrap(),
            descuento: Decimal::from_str("0.00").unwrap(),
            created_at: Utc::now(),
        }];

        /* line_article_ids = None → usa article.id (7777) */
        let order = BdpSyncService::build_order(
            &config,
            &venta,
            &article,
            Some(&lineas),
            None,
            &test_order_ctx(),
        );
        let items = order.order.get("Items").and_then(|v| v.as_array()).unwrap();
        assert_eq!(
            items[0].get("Id").unwrap().as_i64().unwrap(),
            7777,
            "Should fallback to article.id when line_article_ids is None"
        );
    }

    #[test]
    fn validate_order_lines_rejects_non_positive_quantity() {
        let venta_id = uuid::Uuid::new_v4();
        let lineas = vec![VentaLinea {
            id: uuid::Uuid::new_v4(),
            venta_id,
            articulo_codigo: "A".into(),
            descripcion: "Cantidad inválida".into(),
            cantidad: Decimal::ZERO,
            precio_unitario: Decimal::from_str("2.00").unwrap(),
            iva_pct: Decimal::from_str("10").unwrap(),
            descuento: Decimal::ZERO,
            created_at: Utc::now(),
        }];
        assert!(BdpSyncService::validate_order_lines(&lineas).is_err());
    }

    #[test]
    fn validate_order_lines_rejects_negative_price_or_discount() {
        let venta_id = uuid::Uuid::new_v4();
        for (price, discount) in [("-1.00", "0"), ("1.00", "-0.01")] {
            let lineas = vec![VentaLinea {
                id: uuid::Uuid::new_v4(),
                venta_id,
                articulo_codigo: "A".into(),
                descripcion: "Importe inválido".into(),
                cantidad: Decimal::ONE,
                precio_unitario: Decimal::from_str(price).unwrap(),
                iva_pct: Decimal::from_str("10").unwrap(),
                descuento: Decimal::from_str(discount).unwrap(),
                created_at: Utc::now(),
            }];
            assert!(BdpSyncService::validate_order_lines(&lineas).is_err());
        }
    }

    #[test]
    fn parse_remote_money_accepts_number_and_string_but_rejects_invalid_values() {
        assert_eq!(
            BdpSyncService::parse_remote_money(&serde_json::json!("12.50"), "Total").unwrap(),
            Decimal::from_str("12.50").unwrap()
        );
        assert_eq!(
            BdpSyncService::parse_remote_money(&serde_json::json!(0), "Total").unwrap(),
            Decimal::ZERO
        );
        assert!(BdpSyncService::parse_remote_money(&serde_json::Value::Null, "Total").is_err());
        assert!(BdpSyncService::parse_remote_money(&serde_json::json!("NaN"), "Total").is_err());
        assert!(BdpSyncService::parse_remote_money(&serde_json::json!(-1), "Total").is_err());
    }

    #[test]
    fn remote_payment_id_accepts_string_or_number_but_rejects_empty_values() {
        assert_eq!(
            BdpSyncService::remote_payment_id(Some(&serde_json::json!("PAY-1"))),
            Some("PAY-1".to_string())
        );
        assert_eq!(
            BdpSyncService::remote_payment_id(Some(&serde_json::json!(42))),
            Some("42".to_string())
        );
        assert_eq!(
            BdpSyncService::remote_payment_id(Some(&serde_json::json!("  "))),
            None
        );
        assert_eq!(
            BdpSyncService::remote_payment_id(Some(&serde_json::Value::Null)),
            None
        );
    }

    #[test]
    fn parse_remote_payments_deduplicates_same_id_and_amount() {
        let payments = vec![
            serde_json::json!({"PaymentId": "PAY-1", "Amount": "4.50"}),
            serde_json::json!({"PaymentId": "PAY-1", "Amount": 4.50}),
            serde_json::json!({"PaymentId": 42, "Amount": "2.00"}),
            serde_json::json!({"Amount": "1.00"}),
        ];
        let (amounts, total) = BdpSyncService::parse_remote_payments(&payments).unwrap();
        assert_eq!(amounts.len(), 2);
        assert_eq!(
            amounts.get("PAY-1"),
            Some(&Decimal::from_str("4.50").unwrap())
        );
        assert_eq!(amounts.get("42"), Some(&Decimal::from_str("2.00").unwrap()));
        assert_eq!(total, Decimal::from_str("7.50").unwrap());
    }

    #[test]
    fn parse_remote_payments_rejects_same_id_with_different_amount() {
        let payments = vec![
            serde_json::json!({"PaymentId": "PAY-1", "Amount": "4.50"}),
            serde_json::json!({"PaymentId": "PAY-1", "Amount": "5.00"}),
        ];
        let error = BdpSyncService::parse_remote_payments(&payments).unwrap_err();
        assert!(error.contains("importes contradictorios"));
    }

    #[test]
    fn local_payment_union_is_conservative_and_rejects_mismatch() {
        let remote = HashMap::from([("PAY-1".to_string(), Decimal::from_str("4.50").unwrap())]);
        assert_eq!(
            BdpSyncService::local_payment_contribution(
                &remote,
                Some("PAY-1"),
                Decimal::from_str("4.50").unwrap()
            )
            .unwrap(),
            Decimal::ZERO
        );
        assert_eq!(
            BdpSyncService::local_payment_contribution(
                &remote,
                Some("LOCAL-2"),
                Decimal::from_str("2.00").unwrap()
            )
            .unwrap(),
            Decimal::from_str("2.00").unwrap()
        );
        assert_eq!(
            BdpSyncService::local_payment_contribution(
                &remote,
                None,
                Decimal::from_str("1.00").unwrap()
            )
            .unwrap(),
            Decimal::from_str("1.00").unwrap()
        );
        let error = BdpSyncService::local_payment_contribution(
            &remote,
            Some("PAY-1"),
            Decimal::from_str("5.00").unwrap(),
        )
        .unwrap_err();
        assert!(error.contains("importe local y remoto distinto"));
    }

    #[test]
    fn validate_order_lines_rejects_discount_above_gross_amount() {
        let venta_id = uuid::Uuid::new_v4();
        let lineas = vec![VentaLinea {
            id: uuid::Uuid::new_v4(),
            venta_id,
            articulo_codigo: "A".into(),
            descripcion: "Descuento imposible".into(),
            cantidad: Decimal::ONE,
            precio_unitario: Decimal::from_str("2.00").unwrap(),
            iva_pct: Decimal::from_str("10").unwrap(),
            descuento: Decimal::from_str("2.01").unwrap(),
            created_at: Utc::now(),
        }];
        assert!(BdpSyncService::validate_order_lines(&lineas).is_err());
    }

    /* [128A-1/F2-4] `resolve_article_local` no exige que el código Glory
     * configurado sea numérico: busca el mapeo local por el string exacto y,
     * si el código configurado no es numérico, el id BDP sale del
     * `articulo_bdp_codigo` del mapeo. Vacío o "GLORY" → fallback genérico. */
    #[sqlx::test(migrations = "./migrations")]
    async fn test_resolve_article_local_sin_codigo_numerico(pool: sqlx::PgPool) {
        let user_id = Uuid::new_v4();
        let email = format!("bdp-sync-{user_id}@example.com");
        sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(&email)
            .bind("argon2_hash_placeholder")
            .execute(&pool)
            .await
            .expect("create_test_user failed");

        BdpArticleMapRepository::crear(
            &pool,
            user_id,
            &CrearBdpArticleMapRequest {
                articulo_glory_codigo: "CAFE001".into(),
                articulo_bdp_codigo: Some("1001".into()),
                articulo_bdp_nombre: Some("CAFE BOMBON".into()),
                descripcion: Some("Cafe bombon grande".into()),
                precio_tarifa1: Some(Decimal::from_str("5.00").unwrap()),
                iva_pct: Some(Decimal::from_str("10").unwrap()),
                departamento: None,
                familia: None,
                subfamilia: None,
                activo: Some(true),
                barcode: None,
            },
        )
        .await
        .expect("crear mapeo local");

        let mut config = test_config();
        config.user_id = user_id;
        config.bdp_default_article_code = "CAFE001".into();

        let resolved = BdpSyncService::resolve_article_local(&pool, user_id, &config)
            .await
            .expect("debe resolver por glory code alfanumerico");
        assert_eq!(resolved.id, 1001, "id BDP debe salir del mapeo local");
        assert_eq!(resolved.name, "Cafe bombon grande");

        config.bdp_default_article_code = String::new();
        assert!(
            BdpSyncService::resolve_article_local(&pool, user_id, &config)
                .await
                .is_none(),
            "codigo vacio -> fallback generico"
        );

        config.bdp_default_article_code = "GLORY".into();
        assert!(
            BdpSyncService::resolve_article_local(&pool, user_id, &config)
                .await
                .is_none(),
            "codigo GLORY -> fallback generico"
        );
    }
}
