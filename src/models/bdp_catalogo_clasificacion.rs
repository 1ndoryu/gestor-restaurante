/* [198A-1/D7] Clasificación local del catálogo (departamento/familia) con su
 * código numérico BDP. El código se asigna secuencialmente por (user_id, tipo)
 * porque BDP lo exige como entero y el nombre local es texto libre. */

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

pub const TIPO_DEPARTAMENTO: &str = "departamento";
pub const TIPO_FAMILIA: &str = "familia";

#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct BdpCatalogoClasificacion {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tipo: String,
    pub code: i32,
    pub nombre: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CrearBdpClasificacionRequest {
    /// `departamento` | `familia`
    #[validate(length(min = 1, max = 20))]
    pub tipo: String,
    #[validate(length(min = 1, max = 255, message = "El nombre es obligatorio (max 255)"))]
    pub nombre: String,
}
