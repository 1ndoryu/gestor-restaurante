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
#[allow(clippy::struct_excessive_bools)]
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
    /* [128A-1/F4] Anulación local de ventas (D4, M9-M11).
     * `anulada=true` es un estado final: las ventas anuladas nunca se borran
     * físicamente (histórico con motivo, D5) y se excluyen del resumen diario
     * en modalidad `credito_completo` (M10). La transición única
     * pendiente/pagada -> anulada la garantiza el UPDATE con guard en el repo. */
    pub anulada: bool,
    pub anulada_at: Option<DateTime<Utc>>,
    pub anulacion_motivo: Option<String>,
    pub anulacion_usuario: Option<Uuid>,
    /* [128A-1/F6] Factura local mínima (A7/D9): numeración local secuencial +
     * estado `facturada`. `facturada_local=true` es final (doble facturación
     * bloqueada M9); las ventas facturadas no se pueden anular. Con BDP,
     * `InvoiceOrder` sigue el flujo actual (`bdp_invoiced`). */
    pub facturada_local: bool,
    pub factura_numero: Option<String>,
    pub factura_fecha: Option<DateTime<Utc>>,
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
    /// Si se incluye, reemplaza atómicamente todas las líneas de la venta.
    /// `None` conserva las líneas actuales; `Some([])` las elimina.
    pub lineas: Option<Vec<CrearVentaLineaRequest>>,
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
#[allow(clippy::struct_excessive_bools)]
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
    /* [128A-1/F4] Anulación local de ventas — mismos campos que `Venta`. */
    pub anulada: bool,
    pub anulada_at: Option<DateTime<Utc>>,
    pub anulacion_motivo: Option<String>,
    pub anulacion_usuario: Option<Uuid>,
    /* [128A-1/F6] Factura local mínima — mismos campos que `Venta`. */
    pub facturada_local: bool,
    pub factura_numero: Option<String>,
    pub factura_fecha: Option<DateTime<Utc>>,
}

/* [128A-1/F4] Request de anulación local de ventas.
 * - `motivo`: obligatorio en modalidad `credito_completo` (M10).
 * - `idempotency_key`: doble click seguro (guard C1); si se reenvía la misma
 *   clave tras un éxito previo, la operación es idempotente.
 * - `anulacion_usuario`: NO se acepta del cliente (spoofeable); el usuario
 *   que anula siempre se deriva de `auth.user_id` en el handler (F4-3). */
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AnularVentaRequest {
    #[validate(length(
        min = 1,
        max = 500,
        message = "El motivo de anulación es obligatorio y no debe exceder 500 caracteres"
    ))]
    pub motivo: Option<String>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
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
