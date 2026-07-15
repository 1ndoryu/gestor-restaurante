/* 253A-5: Modelo de venta para el restaurante.
Campos basados en especificaciones del cliente (audios 4, 8-9) y roadmap sección 4. */

use super::venta_linea::CrearVentaLineaRequest;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

/// Turnos de servicio del restaurante
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "VARCHAR")]
#[serde(rename_all = "snake_case")]
pub enum Turno {
    #[sqlx(rename = "manana")]
    Manana,
    #[sqlx(rename = "mediodia")]
    Mediodia,
    #[sqlx(rename = "noche")]
    Noche,
}

/// Canales de venta disponibles
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "VARCHAR")]
#[serde(rename_all = "snake_case")]
pub enum CanalVenta {
    #[sqlx(rename = "comedor")]
    Comedor,
    #[sqlx(rename = "barra")]
    Barra,
    #[sqlx(rename = "terraza")]
    Terraza,
    #[sqlx(rename = "delivery")]
    Delivery,
    #[sqlx(rename = "just_eat")]
    JustEat,
    #[sqlx(rename = "eventos")]
    Eventos,
}

/// Métodos de pago — re-exportado desde common
pub use super::common::MetodoPago;

/// Venta registrada en el restaurante
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct Venta {
    pub id: Uuid,
    pub user_id: Uuid,
    pub fecha: NaiveDate,
    pub comensales: Option<i32>,
    pub descripcion: String,
    pub iva_porcentaje: rust_decimal::Decimal,
    pub turno: String,
    pub canal: String,
    pub metodo_pago: String,
    pub importe_base: rust_decimal::Decimal,
    pub importe_iva: rust_decimal::Decimal,
    /* [034A-5] Relaciones opcionales para trazabilidad */
    pub reserva_id: Option<Uuid>,
    pub cliente_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /* [064A-6] Tracking de sincronización con Haddock POS */
    pub haddock_synced: bool,
    pub haddock_synced_at: Option<DateTime<Utc>>,
    pub haddock_sync_error: Option<String>,
    /* [065A-5] Tracking de sincronización BDP (patrón idéntico a haddock_synced). */
    pub bdp_synced: bool,
    pub bdp_synced_at: Option<DateTime<Utc>>,
    pub bdp_sync_error: Option<String>,
    pub bdp_order_id: Option<i64>,
    /* [F4.1] Estado del pedido BDP: pending, confirmed, invoiced, error */
    pub bdp_order_status: Option<String>,
    /* [F8.4] Indica si la venta fue facturada en BDP (InvoiceOrder exitoso). */
    pub bdp_invoiced: bool,
}

/// Request para crear una venta
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CrearVentaRequest {
    pub fecha: NaiveDate,
    pub comensales: Option<i32>,
    #[validate(length(max = 500, message = "La descripción no debe exceder 500 caracteres"))]
    pub descripcion: Option<String>,
    pub iva_porcentaje: rust_decimal::Decimal,
    pub turno: Turno,
    pub canal: CanalVenta,
    pub metodo_pago: MetodoPago,
    pub importe_base: rust_decimal::Decimal,
    pub importe_iva: rust_decimal::Decimal,
    /* [F2.3] Líneas opcionales de la venta.
     * Si se proporcionan, bdp_sync usará estas líneas para construir un pedido multi-item.
     * Si es None o vacío, se usa el comportamiento legacy (1 artículo genérico). */
    pub lineas: Option<Vec<CrearVentaLineaRequest>>,
}

/* [283A-22] Request para actualizar una venta — todos los campos opcionales
 * para soportar actualizaciones parciales. */
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ActualizarVentaRequest {
    pub fecha: Option<NaiveDate>,
    pub comensales: Option<i32>,
    #[validate(length(max = 500, message = "La descripción no debe exceder 500 caracteres"))]
    pub descripcion: Option<String>,
    pub iva_porcentaje: Option<rust_decimal::Decimal>,
    pub turno: Option<Turno>,
    pub canal: Option<CanalVenta>,
    pub metodo_pago: Option<MetodoPago>,
    pub importe_base: Option<rust_decimal::Decimal>,
    pub importe_iva: Option<rust_decimal::Decimal>,
}

/// Response paginada de ventas
/* [034A-5] Incluye nombre_cliente resuelto por LEFT JOIN para evitar N+1 en frontend */
#[derive(Debug, Serialize, ToSchema)]
pub struct VentasPaginadas {
    pub items: Vec<VentaConCliente>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

/* [034A-5] Venta enriquecida con nombre del cliente para listados.
 * Evita que el frontend haga un request por cada venta para resolver el nombre. */
#[derive(Debug, Clone, sqlx::FromRow, Serialize, ToSchema)]
pub struct VentaConCliente {
    pub id: Uuid,
    pub user_id: Uuid,
    pub fecha: NaiveDate,
    pub comensales: Option<i32>,
    pub descripcion: String,
    pub iva_porcentaje: rust_decimal::Decimal,
    pub turno: String,
    pub canal: String,
    pub metodo_pago: String,
    pub importe_base: rust_decimal::Decimal,
    pub importe_iva: rust_decimal::Decimal,
    pub reserva_id: Option<Uuid>,
    pub cliente_id: Option<Uuid>,
    pub nombre_cliente: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /* [064A-6] Tracking de sincronización con Haddock POS */
    pub haddock_synced: bool,
    pub haddock_synced_at: Option<DateTime<Utc>>,
    pub haddock_sync_error: Option<String>,
    /* [065A-5] Tracking de sincronización con BDP WebLink */
    pub bdp_synced: bool,
    pub bdp_synced_at: Option<DateTime<Utc>>,
    pub bdp_sync_error: Option<String>,
    pub bdp_order_id: Option<i64>,
    /* [F4.1] Estado del pedido BDP */
    pub bdp_order_status: Option<String>,
    /* [F8.4] Indica si la venta fue facturada en BDP */
    pub bdp_invoiced: bool,
}

/// Query params para listar ventas con filtro por fecha
/* [044A-8+9] Añadidos busqueda, sort_by, sort_order para buscador y ordenamiento.
 * [064A-3] Añadidos turno, canal, metodo_pago como filtros por columna (multi-valor separado por coma).
 * [064A-12] Filtro estado_haddock (multi-valor: synced, error, pending). */
#[derive(Debug, Deserialize, IntoParams)]
pub struct VentasQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
    /// Filtrar desde esta fecha (YYYY-MM-DD)
    pub desde: Option<NaiveDate>,
    /// Filtrar hasta esta fecha (YYYY-MM-DD)
    pub hasta: Option<NaiveDate>,
    /// Búsqueda por texto (descripción, cliente, canal)
    pub busqueda: Option<String>,
    /// Filtro por turno (valores separados por coma: `manana,mediodia,noche`)
    pub turno: Option<String>,
    /// Filtro por canal (valores separados por coma: `comedor,barra,terraza,delivery,just_eat,eventos`)
    pub canal: Option<String>,
    /// Filtro por método de pago (valores separados por coma: `efectivo,tarjeta,transferencia`)
    pub metodo_pago: Option<String>,
    /// Filtro por estado Haddock (valores separados por coma: `synced,error,pending`)
    pub estado_haddock: Option<String>,
    /// Filtro por estado BDP (valores separados por coma: `synced,accepted,invoiced,error,pending,cancelled`)
    pub estado_bdp: Option<String>,
    /// Campo de ordenamiento: `fecha`, `importe_base`, `turno`, `canal`, `metodo_pago`
    pub sort_by: Option<String>,
    /// Dirección de orden: asc o desc. Por defecto desc
    pub sort_order: Option<String>,
}

fn default_page() -> i64 {
    1
}
fn default_per_page() -> i64 {
    20
}
