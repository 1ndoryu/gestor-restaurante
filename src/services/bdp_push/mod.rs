// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
/* [198A-1/F1] Cola unidireccional Glory -> BDP (bdp_push_pendientes).
 *
 * Las ediciones locales encolan una fila activa; un worker (o el botón
 * "Sincronizar a BDP") la procesa con los guards de escritura existentes. La
 * política de reintentos distingue (D2 resuelta):
 *   - error transitorio  -> reintento automático acotado (tope en config);
 *   - "Subscripción no activada" -> 'pendiente_suscripcion' SIN reintento
 *     automático (la suscripción puede no activarse nunca); solo manual;
 *   - rechazo definitivo (HTTP 4xx: payload inválido o conflicto) ->
 *     'rechazado' SIN reintento automático (el error no va a desaparecer
 *     solo); solo manual, y solo tras corregir el dato local.
 *
 * Concurrencia (M19): UNIQUE parcial sobre filas activas + upsert; una sola
 * fila por (user_id, dominio, entidad_id, operacion). */

mod clasificacion;
mod constantes;
mod cola;
mod enriquecimiento;
mod flush;
mod payloads;
#[cfg(test)]
mod tests;

pub use constantes::{
    DOMINIO_ARTICULO, DOMINIO_CLIENTE_PUNTOS, DOMINIO_DEPARTAMENTO, DOMINIO_FAMILIA,
    DOMINIO_PROPINA, DOMINIO_STOCK, DOMINIO_VENTA, ESTADO_DESCARTADO, ESTADO_ERROR,
    ESTADO_PENDIENTE, ESTADO_PENDIENTE_SUSCRIPCION, ESTADO_RECHAZADO, ESTADO_SINCRONIZADO,
    OPERACION_CANCELAR, OPERACION_CREAR, OPERACION_INVENTARIO, OPERACION_MODIFICAR,
    OPERACION_PRECIOS, OPERACION_PROPINA, OPERACION_PUNTOS, OPERACION_REGULARIZAR,
    OPERACION_TRASPASAR, REINTENTOS_MAX,
};
pub use cola::{BdpPushFila, BdpPushPendiente, BdpPushService, MarcarResultado};
pub use flush::{BdpPushFlushResumen, BdpPushFlushService};
pub use payloads::{
    payload_cancelar, payload_crear_articulo, payload_crear_departamento, payload_crear_familia,
    payload_inventario, payload_modificar_articulo, payload_propina, payload_puntos,
    payload_regularizacion,
};
