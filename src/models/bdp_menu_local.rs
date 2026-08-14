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

/* [128A-1/F7][F7-1] La conversión es FALLIBLE: un tipo desconocido ya no
 * defaultea a `Menu` con un warn (un tipo inválido se perseguiría como 'menu'
 * silenciosamente si el repo se reusara sin la validación del handler). El
 * repo devuelve `Protocol("tipo_invalido")` y el handler lo mapea a 400. */
impl TryFrom<String> for BdpMenuLocalTipo {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.as_str().try_into()
    }
}

impl TryFrom<&str> for BdpMenuLocalTipo {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pack" => Ok(BdpMenuLocalTipo::Pack),
            "menu" => Ok(BdpMenuLocalTipo::Menu),
            _ => Err("tipo inválido: debe ser 'menu' o 'pack'"),
        }
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
    /// Código del artículo del catálogo local (`bdp_article_map.
    /// articulo_glory_codigo`). [128A-1/F7][F7-2] Si llega, debe existir en
    /// el catálogo del usuario (422 si no); vacío/ausente = línea "sin
    /// código" (descripción libre).
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
