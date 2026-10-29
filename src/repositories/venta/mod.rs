// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* 253A-5: Repositorio de ventas — queries SQL con parámetros */

mod anulacion;
mod escritura;
mod factura;
mod lectura;
mod listado;
mod sincronizacion;
mod tipos;

pub use listado::FiltrosListadoVentas;
pub use tipos::{ActualizarVentaData, MesaVenta, NuevaVenta, VentaRepository};
