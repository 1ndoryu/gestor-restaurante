use chrono::Utc;
use serde_json::{json, Value};

use super::campos::{number_f64, number_i64, text_field, value_array};
use super::{BdpDryRunArticle, BDP_DRY_RUN_MARKET_ID};
use crate::models::ConfiguracionRestaurante;
use crate::services::bdp_weblink_catalog::BdpCreateOrderRequest;

pub(super) fn build_only_check_order(
    config: &ConfiguracionRestaurante,
    article: &BdpDryRunArticle,
) -> BdpCreateOrderRequest {
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let marketplace_order_id = format!("GDRY{:010}", Utc::now().timestamp() % 10_000_000_000);
    BdpCreateOrderRequest {
        employee_id: config.bdp_employee_id,
        items_profile_id: config.bdp_items_profile_id,
        /* [169A-2] Mismo OrderEndType que el flujo real (configurable). En OnlyCheck
         * nada se crea de todos modos; esto alinea el dry-run con lo que se enviará. */
        order_end_type: config.bdp_order_end_type,
        order_operation_type: 1,
        invoice: Some(false),
        order: json!({
            "MarketplaceOrderId": marketplace_order_id,
            "MarketId": BDP_DRY_RUN_MARKET_ID,
            "MarketName": "Glory Dry Run",
            "PreparationTime": now,
            "OrderId": 0,
            "PosId": config.bdp_pos_id,
            "Type": 0, /* Barra/Ticket aparcado — el único tipo que pasa validación en POS 31.
                           Type=1 (Mesa) falla 300008 (salón incorrecto) y Type=2 (Delivery) falla 300009.
                           POS 31 usa serie 00031TI (IVA incluido) desde 2026-06-07.
                           Ver bdp-300035-resumen-completo-2026-06-01.md. */
            "RoomNumber": 0,
            "TableNumber": 0,
            "Items": [{
                "Lin": 1,
                "Id": article.id,
                "Name": article.name,
                "Units": 1.0,
                "Price": article.price,
                "Supplement": 0.0,
                "Discount": 0.0,
                "DiscountPct": false,
                "Total": article.price,
                "VatPct": article.vat_pct,
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
            "Total": article.price,
            "ExecutionTime": now,
            "Status": 0,
            "AlreadyInvoiced": false,
            "Comments": "GLORY DRY RUN - NO CREAR"
        }),
    }
}

/* [169A-2] Variante OnlyCheck que replica el payload de pago-en-creación
 * (P1 silencioso de la prueba nocturna): EndType=1, Payments con el total,
 * PaymentId = MarketplaceOrderId (≤15), Invoice=true. OperationType=1, así que
 * BDP solo valida sin crear comanda, pago ni factura. Sin tender no hay pago:
 * usar `build_only_check_order` en ese caso. */
pub(super) fn build_only_check_order_con_pago(
    config: &ConfiguracionRestaurante,
    article: &BdpDryRunArticle,
    tender_id: i32,
) -> BdpCreateOrderRequest {
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let marketplace_order_id = format!("GDRY{:010}", Utc::now().timestamp() % 10_000_000_000);
    /* PaymentId = MarketplaceOrderId: BDP vincula el pago a la comanda por este
     * id estable; va clonado porque `json!` consume los valores. */
    let payment_id = marketplace_order_id.clone();
    BdpCreateOrderRequest {
        employee_id: config.bdp_employee_id,
        items_profile_id: config.bdp_items_profile_id,
        order_end_type: config.bdp_order_end_type,
        order_operation_type: 1,
        invoice: Some(true),
        order: json!({
            "MarketplaceOrderId": marketplace_order_id,
            "MarketId": BDP_DRY_RUN_MARKET_ID,
            "MarketName": "Glory Dry Run",
            "PreparationTime": now,
            "OrderId": 0,
            "PosId": config.bdp_pos_id,
            "Type": 0,
            "RoomNumber": 0,
            "TableNumber": 0,
            "Items": [{
                "Lin": 1,
                "Id": article.id,
                "Name": article.name,
                "Units": 1.0,
                "Price": article.price,
                "Supplement": 0.0,
                "Discount": 0.0,
                "DiscountPct": false,
                "Total": article.price,
                "VatPct": article.vat_pct,
                "Comments": [],
                "Supplements": [],
                "OrderItemType": 0,
                "OrderItemTypeMetaInfo": "",
                "TyC_D1": 0,
                "TyC_D2": 0,
                "TyC_D3": 0,
                "OnSale": false
            }],
            "TenderId": tender_id,
            "Payments": [{
                "TenderId": tender_id,
                "Amount": article.price,
                "PaymentId": payment_id,
            }],
            "Discount": 0.0,
            "DiscountPct": false,
            "Total": article.price,
            "ExecutionTime": now,
            "Status": 0,
            "AlreadyInvoiced": false,
            "Comments": "GLORY DRY RUN CON PAGO - NO CREAR"
        }),
    }
}

pub(super) fn first_article(value: &Value, default_iva_pct: f64) -> Option<BdpDryRunArticle> {
    value_array(
        value,
        &[
            "ArticlesListData",
            "ArticleListData",
            "Articles",
            "ArticleList",
        ],
    )?
    .iter()
    .find_map(|item| {
        let id = number_i64(item, &["ArtCode", "Id", "Code"])?;
        let name = text_field(item, &["ArtDescription", "Description", "Name"])?;
        let price = number_f64(item, &["Price1", "Price", "Total"])?;
        /* [R12] Usar iva_por_defecto de config como fallback en vez de 10.0 hardcodeado */
        let vat_pct = number_f64(item, &["TAVPer", "VatPct"]).unwrap_or(default_iva_pct);
        (id > 0 && price > 0.0).then_some(BdpDryRunArticle {
            id,
            name,
            price,
            vat_pct,
        })
    })
}
