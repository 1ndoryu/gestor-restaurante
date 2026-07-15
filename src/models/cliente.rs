/* 263A-1: Modelo de cliente para el CRM.
Basado en Data II (Video 9): listado paginado, búsqueda, campos completos.
El restaurante maneja ~43k clientes, la paginación es crítica. */

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

/// Cliente del restaurante
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
#[allow(clippy::struct_excessive_bools)]
pub struct Cliente {
    pub id: Uuid,
    pub user_id: Uuid,
    pub nombre: String,
    pub apellidos: String,
    pub telefono: String,
    pub prefijo_telefono: String,
    pub email: String,
    pub empresa: String,
    pub notas: String,
    pub foto_url: String,
    pub consentimiento_comercial_email: bool,
    pub consentimiento_comercial_sms: bool,
    pub enviar_encuestas: bool,
    pub alergias: String,
    pub preferencias_bebida: String,
    pub preferencias_ubicacion: String,
    /* [094A-5] Fecha de última visita — se actualiza automáticamente al completar reserva */
    pub ultima_visita: Option<DateTime<Utc>>,
    /* [Fase 7.3] Campos BDP para sync bidireccional */
    pub bdp_customer_code: Option<i32>,
    pub bdp_synced: bool,
    pub bdp_synced_at: Option<DateTime<Utc>>,
    pub bdp_sync_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request para crear un cliente
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CrearClienteRequest {
    #[validate(length(
        min = 1,
        max = 255,
        message = "El nombre es obligatorio y no debe exceder 255 caracteres"
    ))]
    pub nombre: String,
    #[validate(length(max = 255))]
    pub apellidos: Option<String>,
    #[validate(length(max = 30))]
    pub telefono: Option<String>,
    #[validate(length(max = 10))]
    pub prefijo_telefono: Option<String>,
    #[validate(length(max = 255), email)]
    pub email: Option<String>,
    #[validate(length(max = 255))]
    pub empresa: Option<String>,
    pub notas: Option<String>,
    pub foto_url: Option<String>,
    pub consentimiento_comercial_email: Option<bool>,
    pub consentimiento_comercial_sms: Option<bool>,
    pub enviar_encuestas: Option<bool>,
    pub alergias: Option<String>,
    pub preferencias_bebida: Option<String>,
    pub preferencias_ubicacion: Option<String>,
}

/// Request para actualizar un cliente
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ActualizarClienteRequest {
    #[validate(length(min = 1, max = 255))]
    pub nombre: Option<String>,
    #[validate(length(max = 255))]
    pub apellidos: Option<String>,
    #[validate(length(max = 30))]
    pub telefono: Option<String>,
    #[validate(length(max = 10))]
    pub prefijo_telefono: Option<String>,
    #[validate(length(max = 255))]
    pub email: Option<String>,
    #[validate(length(max = 255))]
    pub empresa: Option<String>,
    pub notas: Option<String>,
    pub foto_url: Option<String>,
    pub consentimiento_comercial_email: Option<bool>,
    pub consentimiento_comercial_sms: Option<bool>,
    pub enviar_encuestas: Option<bool>,
    pub alergias: Option<String>,
    pub preferencias_bebida: Option<String>,
    pub preferencias_ubicacion: Option<String>,
}

/// Response paginada de clientes
#[derive(Debug, Serialize, ToSchema)]
pub struct ClientesPaginados {
    pub items: Vec<Cliente>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

/// Query para listar/buscar clientes
#[derive(Debug, Deserialize, IntoParams)]
pub struct ClientesQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
    /// Búsqueda por nombre, apellidos, teléfono o email
    pub busqueda: Option<String>,
    /// [044A-8] Columna por la que ordenar: `nombre`, `apellidos`, `telefono`, `email`, `empresa`
    pub sort_by: Option<String>,
    /// [044A-8] Dirección del orden: `asc` o `desc`
    pub sort_order: Option<String>,
}

fn default_page() -> i64 {
    1
}
fn default_per_page() -> i64 {
    20
}

/* [263A-26] Request para merge de clientes duplicados.
 * origen_id = cliente que se absorbe (se elimina al final).
 * destino_id = cliente que sobrevive y hereda relaciones + campos vacíos. */
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct MergeClientesRequest {
    /// ID del cliente que se absorbe (se eliminará)
    pub origen_id: Uuid,
    /// ID del cliente que sobrevive
    pub destino_id: Uuid,
}

/// Resultado de la operación de merge
#[derive(Debug, Serialize, ToSchema)]
pub struct MergeClientesResponse {
    pub cliente: Cliente,
    pub reservas_migradas: i64,
    pub etiquetas_migradas: i64,
    pub campanas_migradas: i64,
}
