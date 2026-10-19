/* [263A-14] Plano de sala: combinaciones.
 * Parte de plano_sala (ver mod.rs). */

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{CombinacionMesas, CrearCombinacionRequest};
use crate::services::PlanoSalaService;
use crate::AppState;

/* ========== Combinaciones ========== */

#[utoipa::path(
    post,
    path = "/api/plano-sala/combinaciones",
    tag = "PlanoSala",
    request_body = CrearCombinacionRequest,
    responses(
        (status = 201, description = "Combinación creada", body = CombinacionMesas),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 422, description = "Error de validación", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn crear_combinacion(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CrearCombinacionRequest>,
) -> Result<(StatusCode, Json<CombinacionMesas>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let combo = PlanoSalaService::crear_combinacion(&state.pool, auth.user_id, req).await?;
    Ok((StatusCode::CREATED, Json(combo)))
}

#[utoipa::path(
    delete,
    path = "/api/plano-sala/combinaciones/{id}",
    tag = "PlanoSala",
    params(("id" = Uuid, Path, description = "ID de la combinación")),
    responses(
        (status = 204, description = "Combinación eliminada"),
        (status = 404, description = "Combinación no encontrada", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn eliminar_combinacion(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    PlanoSalaService::eliminar_combinacion(&state.pool, id, auth.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
