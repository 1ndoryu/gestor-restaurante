/* [F1.3] Modelo de mapeo artículos Glory → BDP.
 * [128A-1/F2] Semántica (M5): la tabla es "artículos del catálogo + mapeo
 * Glory↔BDP". `origen` distingue artículos creados/editados localmente
 * ('local') de los importados de BDP ('bdp'); `local_dirty` marca ediciones
 * locales que el import BDP no debe sobrescribir (M6/M7).
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
    /* [128A-1/F2] Procedencia del registro: 'local' | 'bdp' */
    pub origen: String,
    /* [128A-1/F2] Edición local pendiente de reconciliación con BDP */
    pub local_dirty: bool,
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
    /* [128A-1/F2] Catálogo local: si vienen campos locales, el código BDP
     * puede quedar vacío (artículo local puro). Si no hay campos locales,
     * sigue siendo obligatorio (mapeo BDP clásico). */
    #[validate(length(max = 100, message = "Código BDP requerido (max 100)"))]
    pub articulo_bdp_codigo: Option<String>,
    #[validate(length(max = 255))]
    pub articulo_bdp_nombre: Option<String>,
    /* [128A-1/F2] Campos del catálogo local (opcionales en alta) */
    #[validate(length(max = 255))]
    pub descripcion: Option<String>,
    pub precio_tarifa1: Option<Decimal>,
    pub iva_pct: Option<Decimal>,
    #[validate(range(min = 0, max = 999_999))]
    pub departamento: Option<i32>,
    #[validate(range(min = 0, max = 999_999))]
    pub familia: Option<i32>,
    #[validate(range(min = 0, max = 999_999))]
    pub subfamilia: Option<i32>,
    pub activo: Option<bool>,
    #[validate(length(max = 100))]
    pub barcode: Option<String>,
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

/* [128A-1/F3] Request para ajustar stock local de un artículo (entrada/salida).
 * Fuente de verdad del stock local: `bdp_article_stock`. `stock_actual` de
 * `bdp_article_map` es el snapshot BDP (solo lectura) y nunca se pisa.
 * `delta` puede ser negativo (salida). La idempotency_key opcional deduplica
 * reintentos del mismo ajuste (patrón C1: ON CONFLICT en bdp_audit_log). */
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AjustarBdpArticleStockRequest {
    #[validate(length(min = 1, max = 100, message = "Código Glory requerido (max 100)"))]
    pub articulo_glory_codigo: String,
    pub delta: Decimal,
    #[validate(length(min = 1, max = 255, message = "Motivo requerido (max 255)"))]
    pub motivo: String,
    #[validate(length(max = 50, message = "warehouse_id max 50"))]
    pub warehouse_id: Option<String>,
    #[validate(length(max = 100, message = "idempotency_key max 100"))]
    pub idempotency_key: Option<String>,
}

/// Request para actualizar un mapeo de artículo (PATCH parcial)
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ActualizarBdpArticleMapRequest {
    /* [128A-1/F2] El código BDP es opcional también en PATCH (artículo local). */
    #[validate(length(max = 100))]
    pub articulo_bdp_codigo: Option<String>,
    #[validate(length(max = 255))]
    pub articulo_bdp_nombre: Option<String>,
    /* [128A-1/F2] Campos del catálogo local editables (PATCH parcial) */
    #[validate(length(max = 255))]
    pub descripcion: Option<String>,
    pub precio_tarifa1: Option<Decimal>,
    pub iva_pct: Option<Decimal>,
    #[validate(range(min = 0, max = 999_999))]
    pub departamento: Option<i32>,
    #[validate(range(min = 0, max = 999_999))]
    pub familia: Option<i32>,
    #[validate(range(min = 0, max = 999_999))]
    pub subfamilia: Option<i32>,
    pub activo: Option<bool>,
    #[validate(length(max = 100))]
    pub barcode: Option<String>,
}
