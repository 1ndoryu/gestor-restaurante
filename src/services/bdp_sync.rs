/* [065A-5] Servicio de sincronización Glory → BDP WebLink REST API.
 * Crea comandas reales en el TPV cuando se registra una venta en Glory.
 * Patrón: idéntico a HaddockService — background spawn, mutex por venta, retry con backoff.
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

use chrono::Utc;
use serde_json::{json, Value};
use sqlx::PgPool;
use tokio::sync::Mutex as TokioMutex;
use tracing::{info, warn};

use crate::models::{ConfiguracionRestaurante, Venta};
use crate::repositories::VentaRepository;
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{BdpCreateOrderRequest, BdpGetPosArticlesRequest};

const MAX_RETRIES: u32 = 3;
const BDP_SYNC_MARKET_ID: i32 = 9_900;

static SYNC_LOCKS: LazyLock<StdMutex<HashMap<uuid::Uuid, Arc<TokioMutex<()>>>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));

pub struct BdpSyncService;

impl BdpSyncService {
    /// Orquesta el flujo completo Glory → BDP para una venta.
    pub async fn sync_venta(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
        is_update: bool,
    ) {
        if !config.bdp_sync_enabled || !crate::services::bdp_sync_preflight::bdp_configurado(config)
        {
            return;
        }

        let lock = {
            let mut map = SYNC_LOCKS.lock().expect("SYNC_LOCKS poisoned");
            map.entry(venta.id)
                .or_insert_with(|| Arc::new(TokioMutex::new(())))
                .clone()
        };
        let Ok(_guard) = lock.try_lock() else {
            info!(
                "[065A-5] BDP sync ya en progreso para venta {}, saltando",
                venta.id
            );
            return;
        };

        /* Guard: si ya sincronizada y es create (no update), saltar */
        if !is_update {
            match VentaRepository::find_by_id(pool, venta.id, venta.user_id).await {
                Ok(Some(fresh)) if fresh.bdp_synced => {
                    info!(
                        "[065A-5] Venta {} ya sincronizada con BDP, saltando",
                        venta.id
                    );
                    Self::cleanup_lock(venta.id);
                    return;
                }
                Ok(None) => {
                    warn!("[065A-5] Venta {} no encontrada en BD", venta.id);
                    Self::cleanup_lock(venta.id);
                    return;
                }
                Err(e) => {
                    warn!("[065A-5] Error leyendo venta {} para guard: {e}", venta.id);
                }
                _ => {}
            }
        }

        let client = BdpWeblinkClient::new(config);
        let article = Self::resolve_article(&client, config).await;

        let result = Self::retry_send_order(&client, config, venta, &article).await;

        match result {
            Ok(order_id) => {
                info!(
                    "[065A-5] Venta {} sincronizada con BDP → OrderId={order_id}",
                    venta.id
                );
                if let Err(e) =
                    VentaRepository::update_bdp_status(pool, venta.id, true, None, Some(order_id))
                        .await
                {
                    warn!(
                        "[065A-5] Error actualizando bdp_synced de venta {}: {e}",
                        venta.id
                    );
                }
            }
            Err((permanent, msg)) => {
                if permanent {
                    warn!(
                        "[065A-5] Error auth BDP para venta {}: {msg} — no se reintenta",
                        venta.id
                    );
                } else {
                    warn!(
                        "[065A-5] Fallo definitivo BDP sync venta {}: {msg}",
                        venta.id
                    );
                }
                let safe_msg = Self::sanitize_error(&msg);
                if let Err(e) =
                    VentaRepository::update_bdp_status(pool, venta.id, false, Some(&safe_msg), None)
                        .await
                {
                    warn!(
                        "[065A-5] Error guardando error BDP de venta {}: {e}",
                        venta.id
                    );
                }
            }
        }
        Self::cleanup_lock(venta.id);
    }

    /// Intenta enviar la comanda con reintentos. Devuelve `Ok(order_id)` o `Err((is_permanent, msg))`.
    async fn retry_send_order(
        client: &BdpWeblinkClient<'_>,
        config: &ConfiguracionRestaurante,
        venta: &Venta,
        article: &ResolvedArticle,
    ) -> Result<i64, (bool, String)> {
        let mut last_error = String::new();
        for attempt in 0..MAX_RETRIES {
            match Self::send_order(client, config, venta, article).await {
                Ok(order_id) => return Ok(order_id),
                Err(BdpSyncError::Auth(msg)) => return Err((true, msg)),
                Err(BdpSyncError::Api(msg) | BdpSyncError::Network(msg)) => {
                    last_error = msg;
                    warn!(
                        "[065A-5] Error BDP venta {} (intento {}): {last_error}",
                        venta.id,
                        attempt + 1
                    );
                }
            }
            if attempt < MAX_RETRIES - 1 {
                tokio::time::sleep(std::time::Duration::from_secs(1 << attempt)).await;
            }
        }
        Err((false, last_error))
    }

    /// Construye y envía una comanda a BDP para la venta dada.
    async fn send_order(
        client: &BdpWeblinkClient<'_>,
        config: &ConfiguracionRestaurante,
        venta: &Venta,
        article: &ResolvedArticle,
    ) -> Result<i64, BdpSyncError> {
        let order = Self::build_order(config, venta, article);
        let response = client
            .create_order(&order)
            .await
            .map_err(|e| BdpSyncError::Network(format!("{e}")))?;

        /* Extraer OrderId y ErrorMessage de la respuesta */
        let order_id = response.get("OrderId").and_then(Value::as_i64).unwrap_or(0);
        let error_msg = response
            .get("ErrorMessage")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();

        if error_msg.is_empty() && order_id > 0 {
            Ok(order_id)
        } else if error_msg.is_empty() {
            Err(BdpSyncError::Api(
                "BDP devolvió OrderId=0 sin error".to_string(),
            ))
        } else {
            Err(BdpSyncError::Api(format!("BDP: {error_msg}")))
        }
    }

    /// Construye el payload BDP `CreateOrder` desde una venta Glory.
    fn build_order(
        config: &ConfiguracionRestaurante,
        venta: &Venta,
        article: &ResolvedArticle,
    ) -> BdpCreateOrderRequest {
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        /* MarketplaceOrderId: max 15 chars. Prefijo "G" + timestamp corto. */
        let marketplace_order_id = format!(
            "G{:014}",
            Utc::now().timestamp_millis() % 100_000_000_000_000
        );

        let total =
            Self::decimal_to_f64(&venta.importe_base) + Self::decimal_to_f64(&venta.importe_iva);
        let description = if venta.descripcion.is_empty() {
            article.name.clone()
        } else {
            venta.descripcion.clone()
        };

        BdpCreateOrderRequest {
            employee_id: config.bdp_employee_id,
            items_profile_id: config.bdp_items_profile_id,
            order_end_type: 1, /* Pendiente de validación — no factura, no imprime ticket */
            order_operation_type: 0, /* Escritura real (0=CheckAndCreate) */
            invoice: Some(false),
            order: json!({
                "MarketplaceOrderId": &marketplace_order_id[..15.min(marketplace_order_id.len())],
                "MarketId": BDP_SYNC_MARKET_ID,
                "MarketName": "Glory",
                "PreparationTime": now,
                "OrderId": 0,
                "PosId": config.bdp_pos_id,
                "Type": 0,
                "RoomNumber": 0,
                "TableNumber": 0,
                "Items": [{
                    "Lin": 1,
                    "Id": article.id,
                    "Name": description,
                    "Units": 1.0,
                    "Price": total,
                    "Supplement": 0.0,
                    "Discount": 0.0,
                    "DiscountPct": false,
                    "Total": total,
                    "VatPct": Self::decimal_to_f64(&venta.iva_porcentaje),
                    "Comments": [],
                    "Supplements": [],
                    "OrderItemType": 0,
                    "OrderItemTypeMetaInfo": "",
                    "TyC_D1": 0,
                    "TyC_D2": 0,
                    "TyC_D3": 0,
                    "OnSale": false
                }],
                "Discount": 0.0,
                "DiscountPct": false,
                "Total": total,
                "ExecutionTime": now,
                "Status": 0,
                "AlreadyInvoiced": false,
                "Comments": format!("Glory venta {}", venta.id)
            }),
        }
    }

    /// Resuelve qué artículo BDP usar. Intenta:
    /// 1. Si `bdp_default_article_code` es numérico, usarlo directamente
    /// 2. Si no, buscar el primer artículo del perfil
    /// 3. Fallback: artículo genérico con código 0
    async fn resolve_article(
        client: &BdpWeblinkClient<'_>,
        config: &ConfiguracionRestaurante,
    ) -> ResolvedArticle {
        /* Intento 1: código configurado como número */
        if let Ok(code) = config.bdp_default_article_code.trim().parse::<i64>() {
            if code > 0 {
                return ResolvedArticle {
                    id: code,
                    name: config.bdp_default_article_name.clone(),
                    price: 0.0, /* El precio real viene de la venta */
                    vat_pct: 10.0,
                };
            }
        }

        /* Intento 2: buscar primer artículo del perfil */
        match client
            .get_pos_articles(&BdpGetPosArticlesRequest::first_page(
                config.bdp_items_profile_id,
                1,
            ))
            .await
        {
            Ok(value) => {
                if let Some(article) = Self::extract_first_article(&value) {
                    return article;
                }
            }
            Err(e) => {
                warn!("[065A-5] Error buscando artículo BDP: {e}");
            }
        }

        /* Fallback: genérico */
        ResolvedArticle {
            id: 0,
            name: config.bdp_default_article_name.clone(),
            price: 0.0,
            vat_pct: 10.0,
        }
    }

    fn extract_first_article(value: &Value) -> Option<ResolvedArticle> {
        let items = value
            .get("ArticlesListData")
            .or_else(|| value.get("ArticleListData"))
            .or_else(|| value.get("Articles"))
            .and_then(|v| v.as_array())?;

        let item = items.first()?;
        #[allow(clippy::cast_possible_truncation)]
        let id = item
            .get("ArtCode")
            .or_else(|| item.get("Id"))
            .or_else(|| item.get("Code"))
            .and_then(Value::as_i64)
            .or_else(|| {
                item.get("ArtCode")
                    .or_else(|| item.get("Id"))
                    .and_then(Value::as_f64)
                    .map(|f| f as i64)
            })?;

        let name = item
            .get("ArtDescription")
            .or_else(|| item.get("Description"))
            .or_else(|| item.get("Name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Artículo BDP")
            .to_string();

        #[allow(clippy::cast_precision_loss)]
        let price = item
            .get("Price1")
            .or_else(|| item.get("Price"))
            .and_then(Value::as_f64)
            .or_else(|| item.get("Price1").and_then(Value::as_i64).map(|i| i as f64))
            .unwrap_or(0.0);

        let vat_pct = item
            .get("TAVPer")
            .or_else(|| item.get("VatPct"))
            .and_then(Value::as_f64)
            .unwrap_or(10.0);

        (id > 0).then_some(ResolvedArticle {
            id,
            name,
            price,
            vat_pct,
        })
    }

    fn decimal_to_f64(d: &rust_decimal::Decimal) -> f64 {
        use std::str::FromStr;
        match f64::from_str(&d.to_string()) {
            Ok(v) => v,
            Err(e) => {
                warn!("[065A-5] Error convirtiendo Decimal '{d}' a f64: {e}");
                0.0
            }
        }
    }

    fn sanitize_error(raw: &str) -> String {
        if raw.contains("401") || raw.contains("403") {
            "Error de autenticación con BDP (401/403)".to_string()
        } else if raw.contains("300035") {
            "BDP: serie no válida (300035)".to_string()
        } else if raw.contains("300008") {
            "BDP: salón/mesa no válidos (300008)".to_string()
        } else if raw.contains("300009") {
            "BDP: delivery no soportado en este POS (300009)".to_string()
        } else if raw.contains("301011") {
            "BDP: MarketplaceOrderId demasiado largo (301011)".to_string()
        } else if raw.contains("301400") {
            "BDP: caja cerrada (301400)".to_string()
        } else {
            let truncated: String = raw.chars().take(200).collect();
            format!("Error BDP: {truncated}")
        }
    }

    fn cleanup_lock(venta_id: uuid::Uuid) {
        let mut map = SYNC_LOCKS.lock().expect("SYNC_LOCKS poisoned");
        if let Some(entry) = map.get(&venta_id) {
            if Arc::strong_count(entry) <= 2 {
                map.remove(&venta_id);
            }
        }
    }
}

/// Artículo BDP resuelto para el mapeo.
struct ResolvedArticle {
    id: i64,
    name: String,
    #[allow(dead_code)]
    price: f64,
    #[allow(dead_code)]
    vat_pct: f64,
}

/// Errores clasificados para decidir si reintentar.
enum BdpSyncError {
    /// Error de autenticación — no reintentar.
    #[allow(dead_code)]
    Auth(String),
    /// Error de negocio de BDP — reintentar puede ayudar.
    Api(String),
    /// Error de red — reintentar.
    Network(String),
}

#[cfg(test)]
mod tests {
    use super::*;
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
            bdp_default_article_code: "1001".into(),
            bdp_default_article_name: "CAFE BOMBON".into(),
            ..Default::default()
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

        let order = BdpSyncService::build_order(&config, &venta, &article);
        let order_json = &order.order;

        /* El total debe ser importe_base + importe_iva = 27.50 */
        let total = order_json.get("Total").and_then(|v| v.as_f64()).unwrap();
        assert!((total - 27.5).abs() < 0.01, "Expected ~27.5, got {total}");

        /* Item[0].Price debe ser el total de la venta */
        let item_price = order_json
            .get("Items")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|i| i.get("Price"))
            .and_then(|v| v.as_f64())
            .unwrap();
        assert!((item_price - 27.5).abs() < 0.01);

        /* Type=0 (Barra) */
        let tipo = order_json.get("Type").and_then(|v| v.as_i64()).unwrap();
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

        let order = BdpSyncService::build_order(&config, &venta, &article);
        assert_eq!(order.employee_id, 1);
        assert_eq!(order.items_profile_id, 1);

        let pos_id = order.order.get("PosId").and_then(|v| v.as_i64()).unwrap();
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

        let order = BdpSyncService::build_order(&config, &venta, &article);
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

        let article = BdpSyncService::extract_first_article(&json).unwrap();
        assert_eq!(article.id, 1001);
        assert_eq!(article.name, "CAFE BOMBON");
        assert!((article.price - 5.0).abs() < 0.01);
    }

    #[test]
    fn sanitize_error_classifies_known_codes() {
        assert!(BdpSyncService::sanitize_error("300035 series").contains("serie"));
        assert!(BdpSyncService::sanitize_error("300008 salón").contains("salón"));
        assert!(BdpSyncService::sanitize_error("301011 too long").contains("MarketplaceOrderId"));
        assert!(BdpSyncService::sanitize_error("301400 caja").contains("caja"));
        assert!(BdpSyncService::sanitize_error("401 Unauthorized").contains("autenticación"));
    }
}
