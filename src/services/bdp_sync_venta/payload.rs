/* [267A-9/F4.1] Payload CreateOrder de sync_venta: MarketplaceOrderId, items,
 * total y campos opcionales (TenderId, Payments, Tip, Customer). */

use chrono::Utc;
use serde_json::{json, Value};
use tracing::warn;
use uuid::Uuid;

use crate::models::{ConfiguracionRestaurante, Venta, VentaLinea};
use crate::services::bdp_sync::{
    BdpSyncService, OrderContext, ResolvedArticle, BDP_SYNC_MARKET_ID,
};
use crate::services::bdp_weblink_catalog::BdpCreateOrderRequest;

impl BdpSyncService {
    #[must_use]
    pub fn marketplace_order_id(venta_id: Uuid) -> String {
        let venta_hex = venta_id.simple().to_string();
        format!("G{}", &venta_hex[..14])
    }

    /// Construye el payload BDP `CreateOrder` desde una venta Glory.
    /// [F2.7] Si hay líneas, genera un pedido multi-item. Si no, usa fallback legacy (1 artículo).
    /// `line_article_ids`: paralelo a `lineas`, con el ID BDP de cada artículo resuelto.
    /// [F3.1] `order_ctx`: tender, order type y customer resueltos.
    pub(crate) fn build_order(
        config: &ConfiguracionRestaurante,
        venta: &Venta,
        article: &ResolvedArticle,
        lineas: Option<&[VentaLinea]>,
        line_article_ids: Option<&[i64]>,
        order_ctx: &OrderContext,
    ) -> BdpCreateOrderRequest {
        let total = Self::total_pedido(venta, lineas);
        /* [F2.7] Items multi-item si hay líneas; si no, el artículo genérico legacy. */
        let items = match lineas {
            Some(lineas) => Self::items_de_lineas(lineas, line_article_ids, article),
            None => Self::item_legacy(venta, article, total),
        };

        /* [169A-2] Pago dentro del CreateOrder: la gratuita solo bloquea agregar
         * pago/propina/factura a una comanda EXISTENTE; Payments/Tip/Invoice en la
         * llamada inicial sí están permitidos (manual: "Comandos quitados" vs
         * CreateOrder/Payments/Tip). Solo con tender (venta cobrada) y total
         * positivo: el pago es total, así que Invoice=true. Sin tender: como antes. */
        let pago_incluido = order_ctx.tender_id.is_some() && total > 0.0;

        BdpCreateOrderRequest {
            employee_id: config.bdp_employee_id,
            items_profile_id: config.bdp_items_profile_id,
            order_end_type: 1, /* Pendiente de validación — no factura, no imprime ticket */
            order_operation_type: 0, /* Escritura real (0=CheckAndCreate) */
            invoice: Some(pago_incluido),
            order: Self::json_pedido(config, venta, order_ctx, &items, total, pago_incluido),
        }
    }

    /* [048A-10] El Total del Order debe coincidir con la suma de los Item.Total
     * (bruto por línea), que es el "teórico" que BDP valida — error 300033 cuando
     * no coincide. El IVA se calcula desde VatPct de cada item, no se suma aquí.
     * Evidencia: dry-run real (2026-06-01) pasó validación de totales con
     * Item.Total=Price y Order.Total=Price (sin IVA). En fallback legacy, el item
     * usa base+iva como precio y el total coincide con ese mismo valor. */
    fn total_pedido(venta: &Venta, lineas: Option<&[VentaLinea]>) -> f64 {
        match lineas {
            Some(ls) => ls
                .iter()
                .map(|linea| {
                    Self::decimal_to_f64(&linea.precio_unitario)
                        * Self::decimal_to_f64(&linea.cantidad)
                        - Self::decimal_to_f64(&linea.descuento)
                })
                .sum(),
            None => {
                Self::decimal_to_f64(&venta.importe_base) + Self::decimal_to_f64(&venta.importe_iva)
            }
        }
    }

    /* [F2.7] Un item por línea. [F2.8] El ID BDP sale del mapeo por línea; sin
     * mapeo se usa el artículo por defecto. El cast `Lin` es seguro: nº de líneas
     * de una venta muy por debajo de i32::MAX. */
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    fn items_de_lineas(
        lineas: &[VentaLinea],
        line_article_ids: Option<&[i64]>,
        article: &ResolvedArticle,
    ) -> Vec<Value> {
        lineas
            .iter()
            .enumerate()
            .map(|(i, linea)| {
                let precio = Self::decimal_to_f64(&linea.precio_unitario);
                let cantidad = Self::decimal_to_f64(&linea.cantidad);
                let descuento = Self::decimal_to_f64(&linea.descuento);
                /* [AUDIT-2.3] Validar que precio y cantidad sean positivos. */
                if precio < 0.0 || cantidad <= 0.0 {
                    warn!(
                        "[BDP-SAFE] Línea '{}' tiene precio={} o cantidad={} inválidos; se envía a BDP tal cual",
                        linea.descripcion, precio, cantidad
                    );
                }
                let linea_total = (precio * cantidad) - descuento;
                /* [F2.8] Usar artículo BDP mapeado por línea, o fallback al default */
                let line_article_id = line_article_ids
                    .and_then(|ids| ids.get(i))
                    .copied()
                    .unwrap_or(article.id);
                json!({
                    "Lin": (i + 1) as i32,
                    "Id": line_article_id,
                    "Name": linea.descripcion,
                    "Units": cantidad,
                    "Price": precio,
                    "Supplement": 0.0,
                    "Discount": descuento,
                    "DiscountPct": false,
                    "Total": linea_total,
                    "VatPct": Self::decimal_to_f64(&linea.iva_pct),
                    "Comments": [],
                    "Supplements": [],
                    "OrderItemType": 0,
                    "OrderItemTypeMetaInfo": "",
                    "TyC_D1": 0,
                    "TyC_D2": 0,
                    "TyC_D3": 0,
                    "OnSale": false
                })
            })
            .collect()
    }

    /* Fallback legacy: 1 artículo genérico con el total de la venta. */
    fn item_legacy(venta: &Venta, article: &ResolvedArticle, total: f64) -> Vec<Value> {
        let description = if venta.descripcion.is_empty() {
            article.name.clone()
        } else {
            venta.descripcion.clone()
        };
        vec![json!({
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
        })]
    }

    /* Objeto `Order` de CreateOrder con los campos opcionales ya resueltos. */
    fn json_pedido(
        config: &ConfiguracionRestaurante,
        venta: &Venta,
        order_ctx: &OrderContext,
        items: &[Value],
        total: f64,
        pago_incluido: bool,
    ) -> Value {
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        /* MarketplaceOrderId: estable por venta y max 15 chars.
         * Debe mantenerse idéntico entre reintentos para que BDP pueda deduplicar
         * una escritura que sí se aplicó pero cuya respuesta se perdió. */
        let marketplace_order_id = Self::marketplace_order_id(venta.id);

        /* [F3.1-3.3] Construir order JSON con campos opcionales */
        let mut order = json!({
            "MarketplaceOrderId": &marketplace_order_id[..15.min(marketplace_order_id.len())],
            "MarketId": BDP_SYNC_MARKET_ID,
            "MarketName": "Glory",
            "PreparationTime": now,
            "OrderId": 0,
            "PosId": config.bdp_pos_id,
            "Type": order_ctx.order_type,
            "RoomNumber": 0,
            "TableNumber": 0,
            "Items": items,
            "Discount": 0.0,
            "DiscountPct": false,
            "Total": total,
            "ExecutionTime": now,
            "Status": 0,
            "AlreadyInvoiced": false,
            "Comments": format!("Glory venta {}", venta.id)
        });
        /* [F3.2] TenderId — mapeo de método de pago */
        if let Some(tender_id) = order_ctx.tender_id {
            order["TenderId"] = json!(tender_id);
        }
        /* [169A-2] Payments — pago total dentro de la creación.
         * PaymentId: identificador del pago en nuestro sistema
         * (MarketplaceOrderId estable, max 15 chars). */
        if pago_incluido {
            order["Payments"] = json!([{
                "TenderId": order_ctx.tender_id,
                "Amount": total,
                "PaymentId": &marketplace_order_id[..15.min(marketplace_order_id.len())],
            }]);
        }
        /* [169A-2] Tip — propina de la venta, solo si positiva. */
        let propina = Self::decimal_to_f64(&venta.propina);
        if propina > 0.0 {
            order["Tip"] = json!(propina);
        }
        /* [F3.1] Customer — datos del cliente si existe */
        if let Some(ref name) = order_ctx.customer_name {
            let mut customer = json!({ "Name": name });
            if let Some(ref phone) = order_ctx.customer_phone {
                if !phone.is_empty() {
                    customer["Phone"] = json!(phone);
                }
            }
            if let Some(ref code) = order_ctx.customer_code {
                if !code.is_empty() {
                    customer["Code"] = json!(code);
                }
            }
            order["Customer"] = customer;
        }
        order
    }
}
