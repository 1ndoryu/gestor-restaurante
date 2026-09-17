/* 253A-5: Modelo de resumen del dashboard.
Audio 9: Gastos, Ventas, Margen (ventas - gastos), rojo si negativo. */

use serde::Serialize;
use utoipa::ToSchema;

/// Resumen económico: Gastos totales, Ventas totales, Margen
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ResumenEconomico {
    pub total_ventas: rust_decimal::Decimal,
    pub total_gastos: rust_decimal::Decimal,
    /// Margen = ventas - gastos. Negativo si hay pérdidas
    pub margen: rust_decimal::Decimal,
    pub mes: String,
}
