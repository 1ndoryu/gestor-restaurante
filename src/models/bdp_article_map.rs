/* [F1.3] Modelo de mapeo artículos Glory → BDP.
 * Permite al usuario vincular códigos de artículo del POS BDP con conceptos Glory.
 * Usado por bdp_sync::resolve_article() para encontrar el código BDP correcto.
 * [157A-7] F9.1: campos enriquecidos para sync completa de catálogo. */

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Registro de mapeo artículo Glory → BDP.
/// Los campos `descripcion`, `precio_tarifa1`, `iva_pct`, `departamento`, `familia`,
/// `subfamilia`, `activo`, `barcode` y `ultima_sync_at` se rellenan por F9.1 sync-catalog.
/// `stock_actual` se rellena si `ExportArticles` devuelve `CurrentStock` en la respuesta.
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct BdpArticleMap {
    pub id: Uuid,
    pub user_id: Uuid,
    pub articulo_glory_codigo: String,
    pub articulo_bdp_codigo: String,
    pub articulo_bdp_nombre: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /* [157A-7] Campos enriquecidos — F9.1 sync-catalog */
    pub descripcion: String,
    pub precio_tarifa1: Decimal,
    pub iva_pct: Decimal,
    pub departamento: i32,
    pub familia: i32,
    pub subfamilia: i32,
    pub activo: bool,
    pub barcode: String,
    pub ultima_sync_at: Option<DateTime<Utc>>,
    /* [237A-4] Stock actual del artículo en BDP (solo lectura) */
    pub stock_actual: Decimal,
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

/// Registro de stock de un artículo por almacén.
/// [247A-10/S2] BDP actualmente no expone almacenes por artículo; se guarda
/// un único almacén por defecto "General" para preparar la evolución futura.
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct BdpArticleStock {
    pub id: Uuid,
    pub user_id: Uuid,
    pub articulo_glory_codigo: String,
    pub warehouse_id: String,
    pub warehouse_name: String,
    pub stock: Decimal,
    pub ultima_sync_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request para actualizar un mapeo de artículo (PATCH parcial)
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ActualizarBdpArticleMapRequest {
    #[validate(length(min = 1, max = 100))]
    pub articulo_bdp_codigo: Option<String>,
    #[validate(length(max = 255))]
    pub articulo_bdp_nombre: Option<String>,
}
