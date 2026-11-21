/* [F2.2] Modelo de líneas de venta (items individuales).
 * Cada venta puede tener N líneas, cada una representando un artículo/servicio.
 * Si una venta no tiene líneas, bdp_sync usa el comportamiento legacy (1 artículo genérico). */

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Línea individual dentro de una venta
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct VentaLinea {
    pub id: Uuid,
    pub venta_id: Uuid,
    pub articulo_codigo: String,
    pub descripcion: String,
    pub cantidad: rust_decimal::Decimal,
    pub precio_unitario: rust_decimal::Decimal,
    pub iva_pct: rust_decimal::Decimal,
    pub descuento: rust_decimal::Decimal,
    /* [18AB-4] Línea marcada como invitación (precio 0 ante el cliente). */
    pub invitacion: bool,
    /* [21AB-3c] Comentario de la línea («sin hielo»); vacío = sin comentario.
     * Viaja al TPV como `Comments[]` del ítem. */
    pub comentario: String,
    pub created_at: DateTime<Utc>,
}

/// Request para crear una línea de venta (usado dentro de `CrearVentaRequest`)
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct CrearVentaLineaRequest {
    /// Código del artículo (puede mapearse a BDP via `bdp_article_map`)
    #[validate(length(max = 100))]
    pub articulo_codigo: Option<String>,
    /// Descripción del artículo/servicio
    #[validate(length(min = 1, max = 500, message = "Descripción requerida (max 500)"))]
    pub descripcion: String,
    /// Cantidad (puede ser decimal para peso, etc.)
    pub cantidad: Option<rust_decimal::Decimal>,
    /// Precio unitario sin IVA
    pub precio_unitario: rust_decimal::Decimal,
    /// Porcentaje de IVA aplicable
    pub iva_pct: Option<rust_decimal::Decimal>,
    /// Descuento aplicado a esta línea
    pub descuento: Option<rust_decimal::Decimal>,
    /* [18AB-4] `true` si la línea va como invitación: se envía `Invitation`
     * en el ítem BDP y la web la rechaza si el artículo no lo permite. */
    #[serde(default)]
    pub invitacion: Option<bool>,
    /* [21AB-3c] Comentario para el TPV («sin hielo», «poco hecho»). BDP no
     * sabe reescribir el comentario de una línea ya enviada: la modificación
     * lo resuelve con contrapartida + alta (doc 20/11). */
    #[validate(length(max = 200, message = "Comentario demasiado largo (max 200)"))]
    #[serde(default)]
    pub comentario: Option<String>,
}
