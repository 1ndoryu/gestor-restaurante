/* [247A-11] Modelo de albarán de compra BDP (solo lectura).
 * Cache local de la respuesta de ExportPurchaseNotes. */

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Estados del ciclo de vida local de un albarán de compra BDP.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "VARCHAR")]
pub enum BdpPurchaseNoteEstado {
    #[sqlx(rename = "pendiente")]
    Pendiente,
    #[sqlx(rename = "borrador")]
    Borrador,
    #[sqlx(rename = "conciliado")]
    Conciliado,
}

impl BdpPurchaseNoteEstado {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            BdpPurchaseNoteEstado::Pendiente => "pendiente",
            BdpPurchaseNoteEstado::Borrador => "borrador",
            BdpPurchaseNoteEstado::Conciliado => "conciliado",
        }
    }
}

impl From<String> for BdpPurchaseNoteEstado {
    fn from(value: String) -> Self {
        match value.as_str() {
            "borrador" => BdpPurchaseNoteEstado::Borrador,
            "conciliado" => BdpPurchaseNoteEstado::Conciliado,
            "pendiente" => BdpPurchaseNoteEstado::Pendiente,
            other => {
                tracing::warn!(
                    "[BdpPurchaseNoteEstado] valor desconocido '{}', defaulteando a Pendiente",
                    other
                );
                BdpPurchaseNoteEstado::Pendiente
            }
        }
    }
}

impl From<&str> for BdpPurchaseNoteEstado {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}

/// Registro de albarán de compra (importado de BDP o creado localmente).
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
    /* [128A-1/F5] Procedencia del albarán: 'bdp' (importado) | 'local'. */
    pub origen: String,
    pub estado: BdpPurchaseNoteEstado,
    pub gasto_id: Option<Uuid>,
    pub ultima_sync_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Línea de un albarán local (IVA por línea — A10).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BdpPurchaseNoteLineaLocal {
    pub descripcion: String,
    pub cantidad: Decimal,
    pub precio_unitario: Decimal,
    pub iva_pct: Decimal,
}

/// Request para crear un albarán de compra local (F5, M18).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CrearBdpPurchaseNoteRequest {
    /// Serie local. Si se omite, se usa la serie reservada `L` (M18).
    pub serie: Option<String>,
    /// Número local. Si se omite, se asigna el siguiente secuencial de la serie.
    pub numero: Option<String>,
    /// Fecha del albarán (YYYY-MM-DD).
    pub fecha: Option<String>,
    pub codigo_proveedor: Option<String>,
    pub nombre_proveedor: Option<String>,
    /// Total del documento. Si se omite y hay líneas, se calcula (base + IVA).
    pub total: Option<Decimal>,
    /// Líneas del albarán local (guardadas en `datos_bdp.lineas`).
    pub lineas: Option<Vec<BdpPurchaseNoteLineaLocal>>,
}

/// Request para actualizar un albarán de compra local (F5).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ActualizarBdpPurchaseNoteRequest {
    pub numero: Option<String>,
    pub fecha: Option<String>,
    pub codigo_proveedor: Option<String>,
    pub nombre_proveedor: Option<String>,
    pub total: Option<Decimal>,
    pub lineas: Option<Vec<BdpPurchaseNoteLineaLocal>>,
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
    /* [287A-5] Opcional en cada petición: si se omite se usa el perfil
     * persistido en configuración. */
    pub export_profile_code: Option<i32>,
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

/// Request para marcar un albarán como borrador (Fase 2).
/// Cuerpo vacío; se mantiene como placeholder por compatibilidad `OpenAPI`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BdpPurchaseNoteDraftRequest {}

/// Request para conciliar un albarán con un gasto existente o nuevo (Fase 3).
#[derive(Debug, Deserialize, ToSchema)]
pub struct BdpPurchaseNoteReconcileRequest {
    /// ID del gasto existente a vincular. Si es `None`, se crea un gasto nuevo.
    pub gasto_existente_id: Option<Uuid>,
    /// Categoría contable para el gasto nuevo.
    pub categoria_id: Option<Uuid>,
}

/// Resultado de la conciliación.
#[derive(Debug, Serialize, ToSchema)]
pub struct BdpPurchaseNoteReconcileResult {
    pub albaran_id: Uuid,
    pub gasto_id: Uuid,
    pub accion: String,
}
