/* [263A-14] Handlers del plano de sala — CRUD zonas/mesas/combinaciones + export/import.
 * Tag OpenAPI: "PlanoSala" */

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    ActualizarMesaRequest, ActualizarParedRequest, ActualizarPosicionesParedesRequest,
    ActualizarPosicionesRequest, ActualizarZonaRequest, CombinacionMesas, CrearCombinacionRequest,
    CrearMesaRequest, CrearParedRequest, CrearZonaRequest, Mesa, ParedSala, PlanoExport,
    PlanoOcupacion, PlanoOcupacionQuery, PlanoSala, ZonaSala,
};
use crate::repositories::plano_sala::{CambiosPared, NuevaPared};
use crate::services::bdp_weblink_catalog::BdpCallWaiterRequest;
use crate::services::{
    BdpWeblinkClient, ConfiguracionService, ModoEfectivo, PlanoSalaService, ServicioModoOperacion,
};
use crate::AppState;

use super::rate_limit::{limitador_escrituras, limitar};

mod camarero;
mod combinaciones;
mod import_export;
mod mesas;
mod ocupacion;
mod paredes;
mod zonas;

pub use camarero::*;
pub use combinaciones::*;
pub use import_export::*;
pub use mesas::*;
pub use ocupacion::*;
pub use paredes::*;
pub use zonas::*;

/* ========== Plano completo ========== */

#[utoipa::path(
    get,
    path = "/api/plano-sala",
    tag = "PlanoSala",
    responses(
        (status = 200, description = "Plano completo", body = PlanoSala),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn obtener_plano(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<PlanoSala>, AppError> {
    let plano = PlanoSalaService::plano_completo(&state.pool, auth.user_id).await?;
    Ok(Json(plano))
}

/* ========== Router ========== */

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/plano-sala", get(obtener_plano))
        .route("/plano-sala/ocupacion", get(obtener_ocupacion))
        .route("/plano-sala/zonas", post(crear_zona))
        .route(
            "/plano-sala/zonas/:id",
            patch(actualizar_zona).delete(eliminar_zona),
        )
        .route("/plano-sala/mesas", post(crear_mesa))
        .route("/plano-sala/mesas/posiciones", patch(actualizar_posiciones))
        .route(
            "/plano-sala/mesas/:id/llamar-camarero",
            post(llamar_camarero),
        )
        .route(
            "/plano-sala/mesas/:id",
            patch(actualizar_mesa).delete(eliminar_mesa),
        )
        .route("/plano-sala/combinaciones", post(crear_combinacion))
        .route(
            "/plano-sala/combinaciones/:id",
            delete(eliminar_combinacion),
        )
        .route("/plano-sala/export", get(exportar_plano))
        .route("/plano-sala/import", post(importar_plano))
        /* [094A-7] Rutas de paredes */
        .route("/plano-sala/paredes", post(crear_pared))
        .route(
            "/plano-sala/paredes/posiciones",
            patch(actualizar_posiciones_paredes),
        )
        .route(
            "/plano-sala/paredes/:id",
            patch(actualizar_pared).delete(eliminar_pared),
        )
        /* Tope por IP de las escrituras de este módulo (ver rate_limit.rs). */
        .layer(axum::middleware::from_fn_with_state(
            limitador_escrituras(),
            limitar,
        ))
}
