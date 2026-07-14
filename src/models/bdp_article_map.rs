/* [F1.3] Modelo de mapeo artículos Glory → BDP.
 * Permite al usuario vincular códigos de artículo del POS BDP con conceptos Glory.
 * Usado por bdp_sync::resolve_article() para encontrar el código BDP correcto. */

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Registro de mapeo artículo Glory → BDP
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct BdpArticleMap {
    pub id: Uuid,
    pub user_id: Uuid,
    pub articulo_glory_codigo: String,
    pub articulo_bdp_codigo: String,
    pub articulo_bdp_nombre: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request para crear un mapeo de artículo
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CrearBdpArticleMapRequest {
    #[validate(length(min = 1, max = 100, message = "Código Glory requerido (max 100)"))]
    pub articulo_glory_codigo: String,
    #[validate(length(min = 1, max = 100, message = "Código BDP requerido (max 100)"))]
    pub articulo_bdp_codigo: String,
    #[validate(length(max = 255))]
    pub articulo_bdp_nombre: Option<String>,
}

/// Request para actualizar un mapeo de artículo (PATCH parcial)
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ActualizarBdpArticleMapRequest {
    #[validate(length(min = 1, max = 100))]
    pub articulo_bdp_codigo: Option<String>,
    #[validate(length(max = 255))]
    pub articulo_bdp_nombre: Option<String>,
}
