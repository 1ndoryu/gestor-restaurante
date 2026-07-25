/* [F1.5] Handlers CRUD para mapeos de artículos Glory → BDP.
 * GET    /api/bdp/article-maps              — listar todos los mapeos
 * POST   /api/bdp/article-maps              — crear/upsert un mapeo
 * POST   /api/bdp/article-maps/import-catalog — importar catálogo desde BDP
 * POST   /api/bdp/article-maps/sync-catalog   — sync enriquecida F9.1
 * POST   /api/bdp/article-maps/sync-prices    — refresh precios F9.3
 * PATCH  /api/bdp/article-maps/:id          — actualizar parcialmente
 * DELETE /api/bdp/article-maps/:id          — eliminar un mapeo
 * POST   /api/bdp/sync-tables               — sync mesas BDP F9.4
 * GET    /api/bdp/menus/:id                 — definición menú F9.5
 * GET    /api/bdp/fastfoods/:id             — definición fastfood F9.5
 * GET    /api/bdp/packs/:id                 — definición pack F9.5
 * [147A-F5.7] import-catalog conecta con BDP Weblink para rellenar mapeos.
 * [157A-7] F9.1: sync-catalog sincroniza catálogo completo con datos enriquecidos.
 * [157A-9] F9.3-F9.5: sync-prices, sync-tables, menús/fastfoods/packs. */

use axum::extract::{Path, State};
use axum::routing::{get, patch};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    ActualizarBdpArticleMapRequest, BdpArticleMap, BdpArticleStock, CrearBdpArticleMapRequest,
};
use crate::repositories::BdpArticleMapRepository;
use crate::services::bdp_weblink_catalog::{
    BdpGetFastfoodRequest, BdpGetMenuRequest, BdpGetPackRequest,
};
use crate::services::{
    BdpCatalogSyncResult, BdpSyncService, BdpWeblinkClient, ConfiguracionService, SyncTablesResult,
};
use crate::AppState;

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct SyncTablesRequest {
    #[serde(default)]
    pub aplicar: bool,
    pub confirmacion: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/bdp/article-maps",
            get(listar_article_maps).post(crear_article_map),
        )
        .route("/bdp/article-stock", get(listar_article_stock))
        .route(
            "/bdp/article-maps/import-catalog",
            axum::routing::post(importar_catalogo),
        )
        .route(
            "/bdp/article-maps/sync-catalog",
            axum::routing::post(sync_catalog),
        )
        /* [157A-9] F9.3-F9.5: endpoints de precios, mesas, menús */
        .route(
            "/bdp/article-maps/sync-prices",
            axum::routing::post(sync_prices),
        )
        .route("/bdp/sync-tables", axum::routing::post(sync_tables))
        .route("/bdp/menus/:id", get(get_menu_definition))
        .route("/bdp/fastfoods/:id", get(get_fastfood_definition))
        .route("/bdp/packs/:id", get(get_pack_definition))
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

/// Listar stock de artículos por almacén. Por defecto devuelve el almacén
/// "General" (`warehouse_id` = "0") mientras BDP no exponga desglose por almacén.
#[utoipa::path(
    get,
    path = "/api/bdp/article-stock",
    tag = "BDP Mapeos",
    responses(
        (status = 200, description = "Lista de stock por almacén", body = [BdpArticleStock]),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_article_stock(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<BdpArticleStock>>, AppError> {
    let stock = BdpArticleMapRepository::listar_stock(&state.pool, auth.user_id, None).await?;
    Ok(Json(stock))
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

    /* Compatibilidad: el endpoint legado usa exactamente el parser tipado y
     * el upsert enriquecido del flujo canónico. Así no quedan dos lógicas con
     * distintos formatos de código ni campos omitidos. */
    let _session = client
        .login()
        .await
        .map_err(|e| AppError::Internal(format!("Error login BDP: {e}")))?;

    let result = BdpSyncService::sync_catalog(&client, &state.pool, auth.user_id, 1)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(serde_json::json!({
        "imported": result.creados + result.actualizados,
        "unchanged": result.sin_cambios,
        "errors": result.errores,
        "total": result.total_bdp,
        "compatibility_endpoint": true
    })))
}

/* [157A-7] F9.1: sync-catalog — Sincronización enriquecida del catálogo BDP → Glory.
 * Similar a import-catalog pero almacena precios, IVA, departamento, familia, etc.
 * Usa BdpSyncService::sync_catalog() con parseo tipado. */
#[utoipa::path(
    post,
    path = "/api/bdp/article-maps/sync-catalog",
    tag = "BDP Mapeos",
    responses(
        (status = 200, description = "Catálogo sincronizado", body = BdpCatalogSyncResult),
        (status = 400, description = "BDP no configurado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn sync_catalog(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<BdpCatalogSyncResult>, AppError> {
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation(
            "BDP no está configurado. Configura URL y credenciales primero.".into(),
        ));
    }

    let client = BdpWeblinkClient::new(&config);

    /* Login — session needed for auth token lifecycle */
    let _session = client
        .login()
        .await
        .map_err(|e| AppError::Internal(format!("Error login BDP: {e}")))?;

    let result = BdpSyncService::sync_catalog(&client, &state.pool, auth.user_id, 1)
        .await
        .map_err(AppError::Internal)?;

    Ok(Json(result))
}

/* [157A-9] F9.3: Sincroniza precios de artículos mapeados desde BDP.
 * Consulta GetPricesArticles para cada artículo y actualiza precio_tarifa1. */
#[utoipa::path(
    post,
    path = "/api/bdp/article-maps/sync-prices",
    tag = "BDP Mapeos",
    responses(
        (status = 200, description = "Precios sincronizados", body = BdpCatalogSyncResult),
        (status = 400, description = "BDP no configurado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
async fn sync_prices(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<BdpCatalogSyncResult>, AppError> {
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id)
        .await
        .map_err(|e| AppError::Internal(format!("Error obteniendo configuración: {e}")))?;

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation(
            "Faltan credenciales BDP configuradas".into(),
        ));
    }

    let client = BdpWeblinkClient::new(&config);
    let _session = client
        .login()
        .await
        .map_err(|e| AppError::Internal(format!("Error login BDP: {e}")))?;

    let result = BdpSyncService::sync_prices(&client, &state.pool, auth.user_id)
        .await
        .map_err(AppError::Internal)?;

    Ok(Json(result))
}

/* [157A-9] F9.4: Sincroniza salones/mesas de BDP al plano de sala Glory.
 * Consulta GetRoomsTables → crea ZonaSala + Mesa según corresponda. */
#[utoipa::path(
    post,
    path = "/api/bdp/sync-tables",
    tag = "BDP Mapeos",
    request_body = SyncTablesRequest,
    responses(
        (status = 200, description = "Mesas sincronizadas", body = SyncTablesResult),
        (status = 400, description = "BDP no configurado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
async fn sync_tables(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<SyncTablesRequest>,
) -> Result<Json<SyncTablesResult>, AppError> {
    if req.aplicar && req.confirmacion.as_deref() != Some("IMPORTAR MESAS BDP") {
        return Err(AppError::Validation(
            "Aplicación bloqueada: escriba exactamente IMPORTAR MESAS BDP. No se realizaron cambios."
                .into(),
        ));
    }
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id)
        .await
        .map_err(|e| AppError::Internal(format!("Error obteniendo configuración: {e}")))?;

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation(
            "Faltan credenciales BDP configuradas".into(),
        ));
    }

    let client = BdpWeblinkClient::new(&config);
    let _session = client
        .login()
        .await
        .map_err(|e| AppError::Internal(format!("Error login BDP: {e}")))?;

    let result = BdpSyncService::sync_tables(&client, &state.pool, auth.user_id, req.aplicar)
        .await
        .map_err(AppError::Internal)?;

    Ok(Json(result))
}

/* [157A-9] F9.5: Consulta la definición de un menú en BDP (grupos + items). */
#[utoipa::path(
    get,
    path = "/api/bdp/menus/{id}",
    tag = "BDP Mapeos",
    params(("id" = i32, Path, description = "ID del menú en BDP")),
    responses(
        (status = 200, description = "Definición del menú"),
        (status = 400, description = "BDP no configurado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
async fn get_menu_definition(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id)
        .await
        .map_err(|e| AppError::Internal(format!("Error obteniendo configuración: {e}")))?;

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation(
            "Faltan credenciales BDP configuradas".into(),
        ));
    }

    let client = BdpWeblinkClient::new(&config);
    let _session = client
        .login()
        .await
        .map_err(|e| AppError::Internal(format!("Error login BDP: {e}")))?;

    let result = client
        .get_menu_definition(&BdpGetMenuRequest { menu_id: id })
        .await
        .map_err(|e| AppError::Internal(format!("Error GetMenuDefinition: {e}")))?;

    Ok(Json(result))
}

/* [157A-9] F9.5: Consulta la definición de un fastfood en BDP (items fijos + extras). */
#[utoipa::path(
    get,
    path = "/api/bdp/fastfoods/{id}",
    tag = "BDP Mapeos",
    params(("id" = i32, Path, description = "ID del fastfood en BDP")),
    responses(
        (status = 200, description = "Definición del fastfood"),
        (status = 400, description = "BDP no configurado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
async fn get_fastfood_definition(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id)
        .await
        .map_err(|e| AppError::Internal(format!("Error obteniendo configuración: {e}")))?;

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation(
            "Faltan credenciales BDP configuradas".into(),
        ));
    }

    let client = BdpWeblinkClient::new(&config);
    let _session = client
        .login()
        .await
        .map_err(|e| AppError::Internal(format!("Error login BDP: {e}")))?;

    let result = client
        .get_fastfood_definition(&BdpGetFastfoodRequest { fastfood_id: id })
        .await
        .map_err(|e| AppError::Internal(format!("Error GetFastfoodDefinition: {e}")))?;

    Ok(Json(result))
}

/* [157A-9] F9.5: Consulta la definición de un pack en BDP (grupos + items). */
#[utoipa::path(
    get,
    path = "/api/bdp/packs/{id}",
    tag = "BDP Mapeos",
    params(("id" = i32, Path, description = "ID del pack en BDP")),
    responses(
        (status = 200, description = "Definición del pack"),
        (status = 400, description = "BDP no configurado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
async fn get_pack_definition(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id)
        .await
        .map_err(|e| AppError::Internal(format!("Error obteniendo configuración: {e}")))?;

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation(
            "Faltan credenciales BDP configuradas".into(),
        ));
    }

    let client = BdpWeblinkClient::new(&config);
    let _session = client
        .login()
        .await
        .map_err(|e| AppError::Internal(format!("Error login BDP: {e}")))?;

    let result = client
        .get_pack_definition(&BdpGetPackRequest { pack_id: id })
        .await
        .map_err(|e| AppError::Internal(format!("Error GetPackDefinition: {e}")))?;

    Ok(Json(result))
}
