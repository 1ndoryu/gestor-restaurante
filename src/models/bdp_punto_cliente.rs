/* [198A-1/D9] Ledger local de puntos de fidelización. El saldo BDP es la
 * fuente remota (GetPoints); este ledger permite operar (sumar/restar) y
 * consultar el saldo local sin BDP, y encola el push AddPoints. */

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct BdpPuntoCliente {
    pub id: Uuid,
    pub user_id: Uuid,
    pub cliente_id: Uuid,
    pub bdp_customer_code: i32,
    pub points_added: Decimal,
    pub reason: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct SumarPuntosRequest {
    /// Positivo suma, negativo resta. No puede ser cero.
    pub points_added: Decimal,
    #[validate(length(min = 1, max = 255, message = "El motivo es obligatorio (max 255)"))]
    pub reason: String,
}
