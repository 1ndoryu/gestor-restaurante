/* [F1.5] Handlers CRUD para mapeos de artículos Glory → BDP.
 * GET    /api/bdp/article-maps              — listar todos los mapeos
 * POST   /api/bdp/article-maps              — crear/upsert un mapeo
 * POST   /api/bdp/article-maps/import-catalog — importar catálogo desde BDP
 * PATCH  /api/bdp/article-maps/:id          — actualizar parcialmente
 * DELETE /api/bdp/article-maps/:id          — eliminar un mapeo
 * [147A-F5.7] import-catalog conecta con BDP Weblink para rellenar mapeos. */

use axum::extract::{Path, State};
use axum::routing::{get, patch};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    ActualizarBdpArticleMapRequest, BdpArticleMap, CrearBdpArticleMapRequest,
};
use crate::repositories::BdpArticleMapRepository;
use crate::services::{
    BdpExportArticlesRequest, BdpWeblinkClient, ConfiguracionService,
};
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/bdp/article-maps",
            get(listar_article_maps).post(crear_article_map),
        )
        .route(
            "/bdp/article-maps/import-catalog",
            axum::routing::post(importar_catalogo),
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

/// Importar catálogo completo de artículos desde BDP `WebLink`.
/// Llama a `ExportArticles`, extrae Code y Name de cada artículo,
/// y hace upsert en `bdp_article_map` (`articulo_glory_codigo` = Code).
#[utoipa::path(
    post,
    path = "/api/bdp/article-maps/import-catalog",
    tag = "BDP Mapeos",
    responses(
        (status = 200, description = "Catálogo importado", body = serde_json::Value),
        (status = 400, description = "BDP no configurado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn importar_catalogo(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation(
            "BDP no está configurado. Configura URL y credenciales primero.".into(),
        ));
    }

    let client = BdpWeblinkClient::new(&config);

    /* Login y exportar artículos — session needed for auth token lifecycle */
    let _session = client
        .login()
        .await
        .map_err(|e| AppError::Internal(format!("Error login BDP: {e}")))?;

    let articles_json = client
        .export_articles(&BdpExportArticlesRequest::all_web_articles(1))
        .await
        .map_err(|e| AppError::Internal(format!("Error exportando artículos: {e}")))?;

    /* Parsear array de artículos — BDP devuelve {"Articles": [...]} */
    let articles = articles_json
        .get("Articles")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            AppError::Internal(
                "Respuesta BDP no contiene array 'Articles'.".into(),
            )
        })?;

    let mut importados: u32 = 0;
    let mut errores: u32 = 0;

    for art in articles {
        let code = art
            .get("Code")
            .and_then(|v| v.as_str())
            .or_else(|| art.get("ItemCode").and_then(|v| v.as_str()));
        let name = art
            .get("Name")
            .and_then(|v| v.as_str())
            .or_else(|| art.get("Description").and_then(|v| v.as_str()));

        let (Some(code), Some(name)) = (code, name) else {
            errores += 1;
            continue;
        };

        if code.is_empty() {
            errores += 1;
            continue;
        }

        let req = CrearBdpArticleMapRequest {
            articulo_glory_codigo: code.to_string(),
            articulo_bdp_codigo: code.to_string(),
            articulo_bdp_nombre: Some(name.to_string()),
        };

        match BdpArticleMapRepository::crear(&state.pool, auth.user_id, &req).await {
            Ok(_) => importados += 1,
            Err(_) => errores += 1,
        }
    }

    Ok(Json(serde_json::json!({
        "imported": importados,
        "errors": errores,
        "total": articles.len(),
    })))
}
