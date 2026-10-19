/* [263A-14] Plano de sala: zonas.
 * Parte de plano_sala (ver mod.rs). */

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{ActualizarZonaRequest, CrearZonaRequest, ZonaSala};
use crate::services::PlanoSalaService;
use crate::AppState;

/* ========== Zonas ========== */

#[utoipa::path(
    post,
    path = "/api/plano-sala/zonas",
    tag = "PlanoSala",
    request_body = CrearZonaRequest,
    responses(
        (status = 201, description = "Zona creada", body = ZonaSala),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 422, description = "Error de validación", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn crear_zona(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CrearZonaRequest>,
) -> Result<(StatusCode, Json<ZonaSala>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let zona = PlanoSalaService::crear_zona(&state.pool, auth.user_id, req).await?;
    Ok((StatusCode::CREATED, Json(zona)))
}

#[utoipa::path(
    patch,
    path = "/api/plano-sala/zonas/{id}",
    tag = "PlanoSala",
    params(("id" = Uuid, Path, description = "ID de la zona")),
    request_body = ActualizarZonaRequest,
    responses(
        (status = 200, description = "Zona actualizada", body = ZonaSala),
        (status = 404, description = "Zona no encontrada", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn actualizar_zona(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ActualizarZonaRequest>,
) -> Result<Json<ZonaSala>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let zona = PlanoSalaService::actualizar_zona(&state.pool, id, auth.user_id, req).await?;
    Ok(Json(zona))
}

#[utoipa::path(
    delete,
    path = "/api/plano-sala/zonas/{id}",
    tag = "PlanoSala",
    params(("id" = Uuid, Path, description = "ID de la zona")),
    responses(
        (status = 204, description = "Zona eliminada"),
        (status = 404, description = "Zona no encontrada", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn eliminar_zona(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    PlanoSalaService::eliminar_zona(&state.pool, id, auth.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
