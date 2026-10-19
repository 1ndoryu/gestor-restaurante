/* [263A-14] Plano de sala: import y export.
 * Parte de plano_sala (ver mod.rs). */

use axum::extract::State;
use axum::Json;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{PlanoExport, PlanoSala};
use crate::services::PlanoSalaService;
use crate::AppState;

/* ========== Export / Import ========== */

#[utoipa::path(
    get,
    path = "/api/plano-sala/export",
    tag = "PlanoSala",
    responses(
        (status = 200, description = "Plano exportado", body = PlanoExport),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn exportar_plano(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<PlanoExport>, AppError> {
    let export = PlanoSalaService::exportar(&state.pool, auth.user_id).await?;
    Ok(Json(export))
}

#[utoipa::path(
    post,
    path = "/api/plano-sala/import",
    tag = "PlanoSala",
    request_body = PlanoExport,
    responses(
        (status = 200, description = "Plano importado", body = PlanoSala),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn importar_plano(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(data): Json<PlanoExport>,
) -> Result<Json<PlanoSala>, AppError> {
    let plano = PlanoSalaService::importar(&state.pool, auth.user_id, data).await?;
    Ok(Json(plano))
}
