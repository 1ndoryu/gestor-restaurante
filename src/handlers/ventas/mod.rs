/* 253A-5: Handlers de ventas — CRUD endpoints */

mod bdp;
mod crud;
mod local;

pub use bdp::*;
pub use crud::*;
pub use local::*;

use axum::routing::{get, post};
use axum::Router;

use crate::AppState;

use super::rate_limit::{limitador_escrituras, limitar};

/* [263A-15] Axum 0.7 (matchit 0.7.x) usa :param, no {param}.
 * Todas las rutas con path params corregidas de {id} a :id.
 * Las anotaciones #[utoipa::path] mantienen {id} (sintaxis OpenAPI, no afecta routing). */
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/ventas", post(crear_venta).get(listar_ventas))
        .route(
            "/ventas/:id",
            get(obtener_venta)
                .put(actualizar_venta)
                .delete(eliminar_venta),
        )
        .route("/ventas/:id/haddock-sync", post(reintentar_sync_haddock))
        .route("/ventas/:id/bdp-sync", post(reintentar_sync_bdp))
        .route("/ventas/:id/bdp-status", get(obtener_bdp_status))
        .route("/ventas/:id/lineas", get(obtener_lineas_venta))
        .route("/ventas/:id/bdp-payment", post(bdp_payment))
        .route("/ventas/:id/bdp-invoice", post(bdp_invoice))
        .route("/ventas/:id/anular", post(anular_venta))
        .route("/ventas/:id/bdp-payments", get(listar_bdp_payments))
        .route("/ventas/:id/pagos-locales", post(pago_parcial_local))
        .route("/ventas/:id/factura-local", post(factura_local))
        .route("/ventas/:id/propina", post(agregar_propina))
        .route("/ventas/bdp-poll", post(bdp_poll))
        /* Tope por IP de las escrituras de este módulo (ver rate_limit.rs). */
        .layer(axum::middleware::from_fn_with_state(
            limitador_escrituras(),
            limitar,
        ))
}
