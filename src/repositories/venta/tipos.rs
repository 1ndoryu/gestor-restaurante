/* Tipos del repositorio de ventas: datos de entrada y el repositorio sin estado. */

use uuid::Uuid;

/// Datos necesarios para insertar una venta en BD
pub struct NuevaVenta<'a> {
    pub user_id: Uuid,
    pub fecha: chrono::NaiveDate,
    pub comensales: Option<i32>,
    pub descripcion: &'a str,
    pub iva_porcentaje: rust_decimal::Decimal,
    pub turno: &'a str,
    pub canal: &'a str,
    pub metodo_pago: &'a str,
    pub importe_base: rust_decimal::Decimal,
    pub importe_iva: rust_decimal::Decimal,
    /* [034A-5] Relaciones opcionales */
    pub reserva_id: Option<Uuid>,
    pub cliente_id: Option<Uuid>,
}

/* [283A-22] Datos para actualizar parcialmente una venta. */
pub struct ActualizarVentaData<'a> {
    pub id: Uuid,
    pub user_id: Uuid,
    pub fecha: Option<chrono::NaiveDate>,
    pub comensales: Option<i32>,
    pub descripcion: Option<&'a str>,
    pub iva_porcentaje: Option<rust_decimal::Decimal>,
    pub turno: Option<&'a str>,
    pub canal: Option<&'a str>,
    pub metodo_pago: Option<&'a str>,
    pub importe_base: Option<rust_decimal::Decimal>,
    pub importe_iva: Option<rust_decimal::Decimal>,
}

pub struct VentaRepository;
