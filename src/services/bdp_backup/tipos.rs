use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Snapshot almacenado en la base de datos.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema, sqlx::FromRow)]
pub struct BdpSnapshot {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tipo: String,
    pub direccion: String,
    pub trigger_tipo: String,
    pub datos: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
    pub target_base_url: Option<String>,
    pub connection_fingerprint: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub notas: Option<String>,
}

/// Entrada del audit log.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema, sqlx::FromRow)]
pub struct BdpAuditEntry {
    pub id: Uuid,
    pub user_id: Uuid,
    pub operacion: String,
    pub direccion: String,
    pub snapshot_pre_id: Option<Uuid>,
    pub datos_enviados: Option<serde_json::Value>,
    pub resultado: String,
    pub datos_respuesta: Option<serde_json::Value>,
    pub error_mensaje: Option<String>,
    pub target_base_url: Option<String>,
    pub target_entity_type: Option<String>,
    pub target_entity_id: Option<Uuid>,
    pub authorization_reason: Option<String>,
    /* [128A-1/F6] Origen de la operación: 'local' (operaciones locales puras)
     * o 'bdp' (default — implican al BDP). Permite que Historial muestre
     * anulaciones, ajustes de stock, pagos y facturas locales sin BDP. */
    pub origen_operacion: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Resultado de una restauración.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct RestoreResult {
    pub snapshot_id: Uuid,
    pub tipo: String,
    pub registros_restaurados: u32,
    pub errores: u32,
    pub detalles: String,
}

/// Datos de un snapshot nuevo antes de persistirlo. Agrupa los argumentos de
/// `insert_snapshot` para que la firma no supere el límite de parámetros.
pub(super) struct NuevoSnapshot<'a> {
    pub(super) tipo: &'a str,
    pub(super) direccion: &'a str,
    pub(super) trigger_tipo: &'a str,
    pub(super) datos: serde_json::Value,
    pub(super) metadata: Option<serde_json::Value>,
    pub(super) target_base_url: Option<String>,
    pub(super) connection_fingerprint: Option<String>,
    pub(super) expires_at: Option<DateTime<Utc>>,
    pub(super) notas: Option<String>,
}
