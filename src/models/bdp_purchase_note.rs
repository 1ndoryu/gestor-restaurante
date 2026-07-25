/* [247A-11] Modelo de albarán de compra BDP (solo lectura).
 * Cache local de la respuesta de ExportPurchaseNotes. */

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Registro de albarán de compra importado desde BDP.
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct BdpPurchaseNote {
    pub id: Uuid,
    pub user_id: Uuid,
    pub serie: String,
    pub numero: String,
    pub fecha: Option<chrono::NaiveDate>,
    pub codigo_proveedor: Option<String>,
    pub nombre_proveedor: Option<String>,
    pub total: Option<Decimal>,
    pub datos_bdp: serde_json::Value,
    pub ultima_sync_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Parámetros de consulta para listar albaranes.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BdpPurchaseNoteListParams {
    #[serde(default)]
    pub proveedor: Option<String>,
    #[serde(default)]
    pub fecha_desde: Option<String>,
    #[serde(default)]
    pub fecha_hasta: Option<String>,
}

/// Request para sincronizar albaranes desde BDP.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BdpPurchaseNoteSyncRequest {
    pub export_profile_code: i32,
    #[serde(default)]
    pub fecha_desde: Option<String>,
    #[serde(default)]
    pub fecha_hasta: Option<String>,
    #[serde(default)]
    pub proveedor_desde: Option<i64>,
    #[serde(default)]
    pub proveedor_hasta: Option<i64>,
}

/// Resumen del resultado de sincronización.
#[derive(Debug, Serialize, ToSchema)]
pub struct BdpPurchaseNoteSyncResult {
    pub procesados: usize,
    pub total_bdp: usize,
}
