/* Mapeos de artículos BDP: listado, CRUD e importación del catálogo.
 * Parte de bdp_article_map (ver mod.rs). */

use super::{
    exigir_modo_bdp, payload_crear_articulo, payload_modificar_articulo, respuesta_cacheable,
    verificar_permiso, AccionPermiso, ActualizarBdpArticleMapRequest, AppError, AppState, AuthUser,
    BdpArticleMap, BdpArticleMapRepository, BdpPushService, BdpSyncService, BdpWeblinkClient,
    ConfiguracionService, CrearBdpArticleMapRequest, HeaderMap, Json, Path, Response, State, Uuid,
    Validate, VisibilidadCache,
};

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
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let maps = BdpArticleMapRepository::listar(&state.pool, auth.user_id).await?;
    /* [169A-5/D2.3] Catálogo del restaurante: 0,1 ms en origen, TTL 60 s. */
    respuesta_cacheable(&maps, VisibilidadCache::Publica, 60, &headers)
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
    /* [128A-1/F8] Permiso por acción: edición de catálogo (D8/M17). */
    verificar_permiso(&state.pool, AccionPermiso::CatalogoEdicion, &auth).await?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let mut map = BdpArticleMapRepository::crear(&state.pool, auth.user_id, &req).await?;
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;

    if map.articulo_bdp_codigo.trim().parse::<i64>().is_ok() {
        /* [198A-1/F1] Artículo ya mapeado a BDP: la edición local se empuja
         * como modificación. */
        let payload = payload_modificar_articulo(&config, &map).map_err(AppError::Internal)?;
        BdpPushService::encolar(
            &state.pool,
            auth.user_id,
            crate::services::bdp_push::DOMINIO_ARTICULO,
            &map.articulo_glory_codigo,
            crate::services::bdp_push::OPERACION_MODIFICAR,
            &payload,
        )
        .await
        .map_err(AppError::Internal)?;
    } else {
        /* [198A-1/D3] Artículo local puro: asignar código del rango reservado
         * (configurable, default 90 000 000, M11/M22) y encolar el alta. */
        let codigo = BdpArticleMapRepository::siguiente_codigo_reservado(
            &state.pool,
            auth.user_id,
            config.bdp_articulo_rango_inicial,
        )
        .await?;
        map =
            BdpArticleMapRepository::asignar_codigo_bdp(&state.pool, map.id, auth.user_id, codigo)
                .await?
                .ok_or_else(|| AppError::NotFound("Mapeo no encontrado".into()))?;
        let payload = payload_crear_articulo(&config, &map).map_err(AppError::Internal)?;
        BdpPushService::encolar(
            &state.pool,
            auth.user_id,
            crate::services::bdp_push::DOMINIO_ARTICULO,
            &map.articulo_glory_codigo,
            crate::services::bdp_push::OPERACION_CREAR,
            &payload,
        )
        .await
        .map_err(AppError::Internal)?;
    }

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
    /* [128A-1/F8] Permiso por acción: edición de catálogo (D8/M17). */
    verificar_permiso(&state.pool, AccionPermiso::CatalogoEdicion, &auth).await?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let map = BdpArticleMapRepository::actualizar(&state.pool, id, auth.user_id, &req)
        .await?
        .ok_or_else(|| AppError::NotFound("Mapeo no encontrado".into()))?;

    /* [198A-1/F1] Push de modificación para artículos ya mapeados a BDP. */
    if map.articulo_bdp_codigo.trim().parse::<i64>().is_ok() {
        let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
        let payload = payload_modificar_articulo(&config, &map).map_err(AppError::Internal)?;
        BdpPushService::encolar(
            &state.pool,
            auth.user_id,
            crate::services::bdp_push::DOMINIO_ARTICULO,
            &map.articulo_glory_codigo,
            crate::services::bdp_push::OPERACION_MODIFICAR,
            &payload,
        )
        .await
        .map_err(AppError::Internal)?;
    }

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
    /* [128A-1/F8] Permiso por acción: edición de catálogo (D8/M17). */
    verificar_permiso(&state.pool, AccionPermiso::CatalogoEdicion, &auth).await?;
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
    /* [149A-3/F4-B] Importar escribe mapeos locales: permiso de edición de
     * catálogo (default solo Admin, delegable por el dueño). */
    verificar_permiso(&state.pool, AccionPermiso::CatalogoEdicion, &auth).await?;
    exigir_modo_bdp(&state, auth.user_id).await?;
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

    let result = BdpSyncService::sync_catalog(
        &client,
        &state.pool,
        auth.user_id,
        config.bdp_catalog_price_type,
    )
    .await
    .map_err(AppError::Internal)?;
    Ok(Json(serde_json::json!({
        "imported": result.creados + result.actualizados,
        "unchanged": result.sin_cambios,
        "omitidos_ediciones_locales": result.omitidos_ediciones_locales,
        "desactivados_localmente": result.desactivados_localmente,
        "errors": result.errores,
        "total": result.total_bdp,
        "compatibility_endpoint": true
    })))
}
