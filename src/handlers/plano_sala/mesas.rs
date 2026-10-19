/* [263A-14] Plano de sala: mesas.
 * Parte de plano_sala (ver mod.rs). */

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{ActualizarMesaRequest, ActualizarPosicionesRequest, CrearMesaRequest, Mesa};
use crate::services::PlanoSalaService;
use crate::AppState;

/* ========== Mesas ========== */

#[utoipa::path(
    post,
    path = "/api/plano-sala/mesas",
    tag = "PlanoSala",
    request_body = CrearMesaRequest,
    responses(
        (status = 201, description = "Mesa creada", body = Mesa),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 409, description = "Número de mesa duplicado en la zona", body = ErrorResponse),
        (status = 422, description = "Error de validación", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn crear_mesa(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<CrearMesaRequest>,
) -> Result<(StatusCode, Json<Mesa>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let mesa = PlanoSalaService::crear_mesa(&state.pool, req).await?;
    Ok((StatusCode::CREATED, Json(mesa)))
}

#[utoipa::path(
    patch,
    path = "/api/plano-sala/mesas/{id}",
    tag = "PlanoSala",
    params(("id" = Uuid, Path, description = "ID de la mesa")),
    request_body = ActualizarMesaRequest,
    responses(
        (status = 200, description = "Mesa actualizada", body = Mesa),
        (status = 404, description = "Mesa no encontrada", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn actualizar_mesa(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ActualizarMesaRequest>,
) -> Result<Json<Mesa>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let mesa = PlanoSalaService::actualizar_mesa(&state.pool, id, req).await?;
    Ok(Json(mesa))
}

#[utoipa::path(
    delete,
    path = "/api/plano-sala/mesas/{id}",
    tag = "PlanoSala",
    params(("id" = Uuid, Path, description = "ID de la mesa")),
    responses(
        (status = 204, description = "Mesa eliminada"),
        (status = 404, description = "Mesa no encontrada", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn eliminar_mesa(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    PlanoSalaService::eliminar_mesa(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    patch,
    path = "/api/plano-sala/mesas/posiciones",
    tag = "PlanoSala",
    request_body = ActualizarPosicionesRequest,
    responses(
        (status = 200, description = "Posiciones actualizadas", body = u64),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn actualizar_posiciones(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<ActualizarPosicionesRequest>,
) -> Result<Json<u64>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let posiciones: Vec<(Uuid, i32, i32)> = req
        .posiciones
        .into_iter()
        .map(|p| (p.id, p.pos_x, p.pos_y))
        .collect();
    let total = PlanoSalaService::actualizar_posiciones(&state.pool, &posiciones).await?;
    Ok(Json(total))
}
