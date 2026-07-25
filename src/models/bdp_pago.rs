/* [247A-9] Modelo del ledger local de pagos parciales BDP.
 * Cada fila representa un pago (total o parcial) sobre una venta.
 * El saldo pendiente se calcula desde la venta y los pagos exitosos. */

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BdpPago {
    pub id: Uuid,
    pub venta_id: Uuid,
    pub amount: Decimal,
    pub tender_id: i32,
    pub idempotency_key: String,
    pub bdp_order_id: Option<i64>,
    pub bdp_payment_id: Option<String>,
    pub resultado: String,
    pub datos_respuesta: Option<serde_json::Value>,
    pub error_mensaje: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
