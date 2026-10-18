/* [283A-20] Modelo de notificaciones en tiempo real.
 * Se almacenan en BD para persistencia y se emiten via SSE al frontend. */

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Notificacion {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tipo: String,
    pub titulo: String,
    pub mensaje: String,
    pub leida: bool,
    pub created_at: DateTime<Utc>,
}

