/* [283A-2] Endpoints de gestión de API keys — protegidos por JWT (AuthUser).
 * El propietario del restaurante crea/lista/revoca keys para chatbots externos. */

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, post};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{ApiKeyCreatedResponse, ApiKeyResponse, CrearApiKeyRequest, UserRole};
use crate::services::ApiKeyService;
use crate::AppState;

#[utoipa::path(
    post,
    path = "/api/api-keys",
    tag = "ApiKeys",
    request_body = CrearApiKeyRequest,
    responses(
        (status = 201, description = "API key creada (la key completa solo se muestra aquí)", body = ApiKeyCreatedResponse),
        (status = 401, description = "No autorizado"),
        (status = 422, description = "Error de validación")
    ),
    security(("bearer_auth" = []))
)]
pub async fn crear_api_key(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CrearApiKeyRequest>,
) -> Result<(StatusCode, Json<ApiKeyCreatedResponse>), AppError> {
    /* [149A-3/H-06] Las API keys dan acceso máquina a máquina: solo el propietario. */
    auth.require_role(&[UserRole::Admin])?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let resp = ApiKeyService::create(&state.pool, auth.user_id, req).await?;
    Ok((StatusCode::CREATED, Json(resp)))
}

#[utoipa::path(
    get,
    path = "/api/api-keys",
    tag = "ApiKeys",
    responses(
        (status = 200, description = "Lista de API keys", body = Vec<ApiKeyResponse>),
        (status = 401, description = "No autorizado")
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_api_keys(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<ApiKeyResponse>>, AppError> {
    /* [149A-3/H-06] Solo el propietario puede listar credenciales de API. */
    auth.require_role(&[UserRole::Admin])?;
    let keys = ApiKeyService::list(&state.pool, auth.user_id).await?;
    Ok(Json(keys))
}

#[utoipa::path(
    delete,
    path = "/api/api-keys/{id}",
    tag = "ApiKeys",
    params(("id" = Uuid, Path, description = "ID de la API key a revocar")),
    responses(
        (status = 200, description = "API key revocada"),
        (status = 401, description = "No autorizado"),
        (status = 404, description = "API key no encontrada")
    ),
    security(("bearer_auth" = []))
)]
pub async fn revocar_api_key(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    /* [149A-3/H-06] Solo el propietario puede revocar credenciales de API. */
    auth.require_role(&[UserRole::Admin])?;
    ApiKeyService::revoke(&state.pool, id, auth.user_id).await?;
    Ok(Json(
        serde_json::json!({ "ok": true, "message": "API key revocada" }),
    ))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api-keys", post(crear_api_key).get(listar_api_keys))
        .route("/api-keys/:id", delete(revocar_api_key))
}
