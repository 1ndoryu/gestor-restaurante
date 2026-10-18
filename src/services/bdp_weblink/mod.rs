/* [065A-2] Cliente base para BDP WebLink REST API.
 * Centraliza login, headers y manejo de ErrorMessage para que la integracion
 * posterior de articulos/clientes/comandas no replique detalles de transporte.
 * Gotcha: el manual no explicita el header del token; se encapsula aqui para
 * ajustar una sola pieza durante la prueba remota si BDP usa otro nombre. */

mod cliente;
mod cliente_operaciones;
mod tipos;
mod transporte;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_respuestas;

pub use tipos::{
    BdpAuthSession, BdpHealthResponse, BdpLoginResponse, BdpVersionResponse, BdpWeblinkClient,
    BdpWeblinkError,
};
pub use transporte::{ensure_embedded_errors_empty, response_error_message};
