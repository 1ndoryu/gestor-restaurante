/* [094A-3] Modelos de trabajadores y permisos.
 * Los trabajadores son staff del restaurante con login propio y permisos restringidos.
 * El propietario define qué secciones puede ver cada trabajador. */

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct Trabajador {
    pub id: Uuid,
    pub user_id: Uuid,
    pub nombre: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub cargo: String,
    pub activo: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/* Response público sin datos sensibles */
#[derive(Debug, Serialize, ToSchema)]
pub struct TrabajadorResponse {
    pub id: Uuid,
    pub nombre: String,
    pub email: String,
    pub cargo: String,
    pub activo: bool,
    pub permisos: Vec<PermisoSeccion>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CrearTrabajadorRequest {
    #[validate(length(min = 1, max = 100, message = "Nombre obligatorio (máx 100)"))]
    pub nombre: String,
    #[validate(email(message = "Email inválido"))]
    pub email: String,
    #[validate(length(min = 8, message = "La contraseña debe tener al menos 8 caracteres"))]
    pub password: String,
    #[validate(length(max = 100))]
    pub cargo: Option<String>,
    /* Lista de secciones permitidas */
    pub permisos: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ActualizarTrabajadorRequest {
    #[validate(length(min = 1, max = 100))]
    pub nombre: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    #[validate(length(min = 8))]
    pub password: Option<String>,
    #[validate(length(max = 100))]
    pub cargo: Option<String>,
    pub activo: Option<bool>,
    /* Si se envía, reemplaza todos los permisos */
    pub permisos: Option<Vec<String>>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct PermisoSeccion {
    pub seccion: String,
    pub permitido: bool,
}

/* Login de trabajador — auth separada del propietario */
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LoginTrabajadorRequest {
    #[validate(email)]
    pub email: String,
    pub password: String,
}

/* Respuesta de auth de trabajador — incluye permisos */
#[derive(Debug, Serialize, ToSchema)]
pub struct TrabajadorAuthResponse {
    pub token: String,
    pub trabajador_id: Uuid,
    pub user_id: Uuid,
    pub nombre: String,
    pub permisos: Vec<String>,
}

/* Secciones válidas del sistema.
 * [169A-3] Grano por entrada de menú (1 clave = 1 entrada del sidebar), para
 * que el dueño configure qué ve cada trabajador. "notificaciones" no tiene
 * entrada de menú (campana del header) pero sigue siendo configurable.
 * La clave gruesa histórica "marketing" se migra a sus hijas
 * (campanas, plantillas_wa, recordatorios) — ver migración
 * 20260916000000_permisos_secciones_granulares. */
pub const SECCIONES_VALIDAS: &[&str] = &[
    "dashboard",
    "ventas",
    "gastos",
    "compras",
    "stock",
    "inventario",
    "catalogo",
    "menus_packs",
    "reservas",
    "calendario",
    "clientes",
    "canales",
    "no_shows",
    "plano_sala",
    "campanas",
    "plantillas_wa",
    "recordatorios",
    "historial",
    "sincronizacion",
    "trabajadores",
    "resenas",
    "inactividad",
    "configuracion",
    "notificaciones",
];

/* [169A-3] Secciones concedidas por defecto a un trabajador nuevo (cuando el
 * dueño no envía `permisos` explícitos al crearlo): operativa diaria del
 * local. Todo lo técnico (BDP), comunicación con clientes y administración
 * queda denegado por defecto; el dueño lo habilita por trabajador. */
pub const PERMISOS_DEFECTO_TRABAJADOR: &[&str] = &[
    "dashboard",
    "ventas",
    "gastos",
    "reservas",
    "calendario",
    "clientes",
    "canales",
    "no_shows",
    "plano_sala",
];
