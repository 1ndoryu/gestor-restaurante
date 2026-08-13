/* [128A-1/F7] Modelo de menús/packs locales (D2, §4.10).
 * Agrupaciones de artículos del catálogo local, operativas sin BDP.
 * Patrón de líneas similar a `venta_lineas`. */

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Tipo de agrupación local: menú o pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "VARCHAR")]
pub enum BdpMenuLocalTipo {
    #[sqlx(rename = "menu")]
    Menu,
    #[sqlx(rename = "pack")]
    Pack,
}

impl BdpMenuLocalTipo {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            BdpMenuLocalTipo::Menu => "menu",
            BdpMenuLocalTipo::Pack => "pack",
        }
    }
}

impl From<String> for BdpMenuLocalTipo {
    fn from(value: String) -> Self {
        match value.as_str() {
            "pack" => BdpMenuLocalTipo::Pack,
            "menu" => BdpMenuLocalTipo::Menu,
            other => {
                tracing::warn!(
                    "[BdpMenuLocalTipo] valor desconocido '{}', defaulteando a Menu",
                    other
                );
                BdpMenuLocalTipo::Menu
            }
        }
    }
}

impl From<&str> for BdpMenuLocalTipo {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}

/// Cabecera de un menú/pack local.
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct BdpMenuLocal {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tipo: BdpMenuLocalTipo,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub precio: Decimal,
    pub activo: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Línea de un menú/pack local (artículo del catálogo).
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct BdpMenuLocalLinea {
    pub id: Uuid,
    pub menu_id: Uuid,
    pub articulo_codigo: Option<String>,
    pub descripcion: String,
    pub cantidad: Decimal,
    pub precio_unitario: Decimal,
    /// Posición de la línea dentro del menú/pack (orden de composición).
    pub orden: i32,
    pub created_at: DateTime<Utc>,
}

/// Menú/pack local con sus líneas cargadas (respuesta de detalle y listado).
/// Campos explícitos (sin `#[serde(flatten)]`) para compatibilidad con
/// `utoipa::ToSchema`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BdpMenuLocalConLineas {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tipo: BdpMenuLocalTipo,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub precio: Decimal,
    pub activo: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub lineas: Vec<BdpMenuLocalLinea>,
}

/// Línea de menú/pack en las peticiones de creación/edición.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BdpMenuLocalLineaRequest {
    /// Código del artículo del catálogo local (opcional, texto libre).
    pub articulo_codigo: Option<String>,
    pub descripcion: String,
    pub cantidad: Option<Decimal>,
    pub precio_unitario: Option<Decimal>,
}

/// Request para crear un menú/pack local (F7).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CrearBdpMenuLocalRequest {
    /// `menu` | `pack`.
    pub tipo: String,
    pub nombre: String,
    pub descripcion: Option<String>,
    /// Precio de venta. Si se omite y hay líneas, se calcula como suma de
    /// `cantidad * precio_unitario`.
    pub precio: Option<Decimal>,
    pub activo: Option<bool>,
    /// Artículos que componen el menú/pack. Obligatorio al crear.
    pub lineas: Vec<BdpMenuLocalLineaRequest>,
}

/// Request para actualizar un menú/pack local (F7).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ActualizarBdpMenuLocalRequest {
    pub tipo: Option<String>,
    pub nombre: Option<String>,
    pub descripcion: Option<String>,
    pub precio: Option<Decimal>,
    pub activo: Option<bool>,
    /// Si llega, se reemplazan las líneas existentes.
    pub lineas: Option<Vec<BdpMenuLocalLineaRequest>>,
}

/// Parámetros de consulta para listar menús/packs locales.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BdpMenuLocalListParams {
    #[serde(default)]
    pub tipo: Option<String>,
    #[serde(default)]
    pub activo: Option<bool>,
    #[serde(default)]
    pub busqueda: Option<String>,
}
