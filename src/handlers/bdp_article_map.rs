/* [F1.5] Handlers CRUD para mapeos de artículos Glory → BDP.
 * GET    /api/bdp/article-maps       — listar todos los mapeos
 * POST   /api/bdp/article-maps       — crear/upsert un mapeo
 * PATCH  /api/bdp/article-maps/:id   — actualizar parcialmente
 * DELETE /api/bdp/article-maps/:id   — eliminar un mapeo
 * Ninguno de estos endpoints llama a la API de BDP — solo operan en DB local. */

use axum::extract::{Path, State};
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    ActualizarBdpArticleMapRequest, BdpArticleMap, CrearBdpArticleMapRequest,
};
use crate::repositories::BdpArticleMapRepository;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/bdp/article-maps",
            get(listar_article_maps).post(crear_article_map),
        )
        .route(
            "/bdp/article-maps/:id",
            patch(actualizar_article_map).delete(eliminar_article_map),
        )
}

/// Listar todos los mapeos de artículos BDP del usuario
#[utoipa::path(
    get,
    path = "/api/bdp/article-maps",
    tag = "BDP Mapeos",
    responses(
        (status = 200, description = "Lista de mapeos", body = [BdpArticleMap]),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_article_maps(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<BdpArticleMap>>, AppError> {
    let maps = BdpArticleMapRepository::listar(&state.pool, auth.user_id).await?;
    Ok(Json(maps))
}

/// Crear o actualizar un mapeo de artículo (upsert por código Glory)
#[utoipa::path(
    post,
    path = "/api/bdp/article-maps",
    tag = "BDP Mapeos",
    request_body = CrearBdpArticleMapRequest,
    responses(
        (status = 201, description = "Mapeo creado/actualizado", body = BdpArticleMap),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 422, description = "Error de validación", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn crear_article_map(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CrearBdpArticleMapRequest>,
) -> Result<Json<BdpArticleMap>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let map = BdpArticleMapRepository::crear(&state.pool, auth.user_id, &req).await?;
    Ok(Json(map))
}

/// Actualizar parcialmente un mapeo de artículo
#[utoipa::path(
    patch,
    path = "/api/bdp/article-maps/{id}",
    tag = "BDP Mapeos",
    params(("id" = Uuid, Path, description = "ID del mapeo")),
    request_body = ActualizarBdpArticleMapRequest,
    responses(
        (status = 200, description = "Mapeo actualizado", body = BdpArticleMap),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 404, description = "Mapeo no encontrado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn actualizar_article_map(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ActualizarBdpArticleMapRequest>,
) -> Result<Json<BdpArticleMap>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let map = BdpArticleMapRepository::actualizar(&state.pool, id, auth.user_id, &req)
        .await?
        .ok_or_else(|| AppError::NotFound("Mapeo no encontrado".into()))?;
    Ok(Json(map))
}

/// Eliminar un mapeo de artículo
#[utoipa::path(
    delete,
    path = "/api/bdp/article-maps/{id}",
    tag = "BDP Mapeos",
    params(("id" = Uuid, Path, description = "ID del mapeo")),
    responses(
        (status = 200, description = "Mapeo eliminado"),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 404, description = "Mapeo no encontrado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn eliminar_article_map(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let eliminado = BdpArticleMapRepository::eliminar(&state.pool, id, auth.user_id).await?;
    if eliminado {
        Ok(Json(serde_json::json!({ "mensaje": "Mapeo eliminado" })))
    } else {
        Err(AppError::NotFound("Mapeo no encontrado".into()))
    }
}
