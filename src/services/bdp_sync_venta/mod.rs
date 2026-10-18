/* [267A-9/F4.1] FASE 7: sync_venta y helpers, movidos tal cual desde bdp_sync.rs.
 * Sin cambios de mensajes, SQL, guardas ni orden. BdpSyncService sigue definido en el hub.
 * Partido por bloque; todas las funciones son métodos de BdpSyncService. */

mod articulos;
mod contexto;
mod envio;
mod flujo;
mod payload;
mod utilidades;
mod validacion;

/// Errores clasificados para decidir si reintentar.
pub(crate) enum BdpSyncError {
    /// Rechazo conocido: no se aplicó una operación válida y no se reintenta.
    Rejected(String),
    /// Timeout, HTTP anómalo o JSON inválido: BDP pudo haber aplicado la orden.
    AmbiguousTransport(String),
}

pub(crate) enum OrderSendFailure {
    Rejected(String),
    Ambiguous(String),
}
