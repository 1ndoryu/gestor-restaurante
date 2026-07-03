/* [263A-14] Modelos del plano de sala: zonas, mesas, combinaciones.
 * El dueño construye el plano arrastrando mesas en un canvas por zona. */

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

/* ========== Zona de sala ========== */

#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct ZonaSala {
    pub id: Uuid,
    pub user_id: Uuid,
    pub nombre: String,
    pub orden: i32,
    pub ancho: i32,
    pub alto: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CrearZonaRequest {
    #[validate(length(
        min = 1,
        max = 100,
        message = "El nombre de la zona es obligatorio (máx 100)"
    ))]
    pub nombre: String,
    pub orden: Option<i32>,
    #[validate(range(min = 200, max = 3000, message = "Ancho entre 200 y 3000 px"))]
    pub ancho: Option<i32>,
    #[validate(range(min = 200, max = 3000, message = "Alto entre 200 y 3000 px"))]
    pub alto: Option<i32>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ActualizarZonaRequest {
    #[validate(length(min = 1, max = 100))]
    pub nombre: Option<String>,
    pub orden: Option<i32>,
    #[validate(range(min = 200, max = 3000))]
    pub ancho: Option<i32>,
    #[validate(range(min = 200, max = 3000))]
    pub alto: Option<i32>,
}

/* ========== Mesa ========== */

#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct Mesa {
    pub id: Uuid,
    pub zona_id: Uuid,
    pub numero: i32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub ancho: i32,
    pub alto: i32,
    pub forma: String,
    pub min_personas: i32,
    pub max_personas: i32,
    pub activa: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CrearMesaRequest {
    pub zona_id: Uuid,
    #[validate(range(min = 1, message = "El número de mesa debe ser >= 1"))]
    pub numero: i32,
    pub pos_x: Option<i32>,
    pub pos_y: Option<i32>,
    #[validate(range(min = 30, max = 400))]
    pub ancho: Option<i32>,
    #[validate(range(min = 30, max = 400))]
    pub alto: Option<i32>,
    #[validate(length(min = 1, max = 20))]
    pub forma: Option<String>,
    #[validate(range(min = 1, max = 50))]
    pub min_personas: Option<i32>,
    #[validate(range(min = 1, max = 50))]
    pub max_personas: Option<i32>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ActualizarMesaRequest {
    #[validate(range(min = 1))]
    pub numero: Option<i32>,
    pub pos_x: Option<i32>,
    pub pos_y: Option<i32>,
    #[validate(range(min = 30, max = 400))]
    pub ancho: Option<i32>,
    #[validate(range(min = 30, max = 400))]
    pub alto: Option<i32>,
    #[validate(length(min = 1, max = 20))]
    pub forma: Option<String>,
    #[validate(range(min = 1, max = 50))]
    pub min_personas: Option<i32>,
    #[validate(range(min = 1, max = 50))]
    pub max_personas: Option<i32>,
    pub activa: Option<bool>,
}

/* Batch update de posiciones — para guardar drag-and-drop */
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ActualizarPosicionesRequest {
    pub posiciones: Vec<PosicionMesa>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct PosicionMesa {
    pub id: Uuid,
    pub pos_x: i32,
    pub pos_y: i32,
}

/* ========== Combinación de mesas ========== */

#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct CombinacionMesas {
    pub id: Uuid,
    pub user_id: Uuid,
    pub nombre: String,
    pub min_personas: i32,
    pub max_personas: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CrearCombinacionRequest {
    #[validate(length(min = 1, max = 100, message = "El nombre es obligatorio (máx 100)"))]
    pub nombre: String,
    #[validate(range(min = 1, max = 50))]
    pub min_personas: Option<i32>,
    #[validate(range(min = 1, max = 100))]
    pub max_personas: i32,
    #[validate(length(min = 2, message = "Una combinación necesita al menos 2 mesas"))]
    pub mesa_ids: Vec<Uuid>,
}

/* ========== Response compuesto: plano completo ========== */

#[derive(Debug, Serialize, ToSchema)]
pub struct PlanoSala {
    pub zonas: Vec<ZonaConMesas>,
    pub combinaciones: Vec<CombinacionConMesas>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ZonaConMesas {
    #[serde(flatten)]
    pub zona: ZonaSala,
    pub mesas: Vec<Mesa>,
    /* [094A-7] Paredes de la zona */
    pub paredes: Vec<ParedSala>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CombinacionConMesas {
    #[serde(flatten)]
    pub combinacion: CombinacionMesas,
    pub mesas: Vec<Mesa>,
}

/* ========== Export/Import ========== */

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PlanoExport {
    pub version: String,
    pub zonas: Vec<ZonaExport>,
    pub combinaciones: Vec<CombinacionExport>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ZonaExport {
    pub nombre: String,
    pub orden: i32,
    pub ancho: i32,
    pub alto: i32,
    pub mesas: Vec<MesaExport>,
    /* [027B-1] Paredes incluidas en export para no perderlas al restaurar */
    #[serde(default)]
    pub paredes: Vec<ParedExport>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MesaExport {
    pub numero: i32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub ancho: i32,
    pub alto: i32,
    pub forma: String,
    pub min_personas: i32,
    pub max_personas: i32,
    pub activa: bool,
}

/* [027B-1] Pared exportable — solo campos de diseño, sin ids ni timestamps */
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ParedExport {
    pub pos_x: i32,
    pub pos_y: i32,
    pub ancho: i32,
    pub alto: i32,
    pub rotacion: i32,
    pub color: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CombinacionExport {
    pub nombre: String,
    pub min_personas: i32,
    pub max_personas: i32,
    /* Números de mesa referenciados (zona_nombre:numero) para remap al importar */
    pub mesas_ref: Vec<String>,
}

/* ========== Ocupación de mesas (vista día) — 263A-16 ========== */

/* ========== Pared de sala — 094A-7 ========== */

#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct ParedSala {
    pub id: Uuid,
    pub zona_id: Uuid,
    pub pos_x: i32,
    pub pos_y: i32,
    pub ancho: i32,
    pub alto: i32,
    pub rotacion: i32,
    pub color: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CrearParedRequest {
    pub zona_id: Uuid,
    pub pos_x: Option<i32>,
    pub pos_y: Option<i32>,
    #[validate(range(min = 10, max = 3000, message = "Ancho entre 10 y 3000 px"))]
    pub ancho: Option<i32>,
    #[validate(range(min = 5, max = 3000, message = "Alto entre 5 y 3000 px"))]
    pub alto: Option<i32>,
    #[validate(range(min = 0, max = 359))]
    pub rotacion: Option<i32>,
    #[validate(length(min = 4, max = 20))]
    pub color: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ActualizarParedRequest {
    pub pos_x: Option<i32>,
    pub pos_y: Option<i32>,
    #[validate(range(min = 10, max = 3000))]
    pub ancho: Option<i32>,
    #[validate(range(min = 5, max = 3000))]
    pub alto: Option<i32>,
    #[validate(range(min = 0, max = 359))]
    pub rotacion: Option<i32>,
    #[validate(length(min = 4, max = 20))]
    pub color: Option<String>,
}

/* Batch update de posiciones de paredes — para drag-and-drop */
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ActualizarPosicionesParedesRequest {
    pub posiciones: Vec<PosicionPared>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct PosicionPared {
    pub id: Uuid,
    pub pos_x: i32,
    pub pos_y: i32,
}

/* ========== Ocupación de mesas (vista día) — 263A-16 ========== */

/// Reserva asociada a una mesa para la vista de ocupación
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReservaMesa {
    pub reserva_id: Uuid,
    pub hora: chrono::NaiveTime,
    pub nombre_cliente: String,
    pub apellidos_cliente: String,
    pub num_personas: i32,
    pub estado: String,
    pub telefono: String,
}

/// Mesa con sus reservas del día para la vista de ocupación
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MesaOcupacion {
    #[serde(flatten)]
    pub mesa: Mesa,
    pub reservas: Vec<ReservaMesa>,
}

/// Zona con mesas y su estado de ocupación
#[derive(Debug, Serialize, ToSchema)]
pub struct ZonaOcupacion {
    #[serde(flatten)]
    pub zona: ZonaSala,
    pub mesas: Vec<MesaOcupacion>,
    /* [134A-12] Paredes visibles en reservas */
    pub paredes: Vec<ParedSala>,
}

/// Plano de sala con ocupación — respuesta del endpoint
#[derive(Debug, Serialize, ToSchema)]
pub struct PlanoOcupacion {
    pub fecha: chrono::NaiveDate,
    pub zonas: Vec<ZonaOcupacion>,
}

/// Query para la ocupación del plano
#[derive(Debug, Deserialize, IntoParams)]
pub struct PlanoOcupacionQuery {
    pub fecha: chrono::NaiveDate,
    pub turno: Option<String>,
}
