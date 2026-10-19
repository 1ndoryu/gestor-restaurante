/* Sincronización BDP → Glory: catálogo, precios, mesas y definiciones de menús.
 * Parte de bdp_article_map (ver mod.rs). */

use super::{
    exigir_modo_bdp, verificar_permiso, AccionPermiso, AppError, AppState, AuthUser,
    BdpCatalogSyncResult, BdpGetFastfoodRequest, BdpGetMenuRequest, BdpGetPackRequest,
    BdpSyncService, BdpWeblinkClient, ConfiguracionService, Json, Path, State, SyncTablesRequest,
    SyncTablesResult, UserRole,
};

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
    /* [149A-3/F4-B] Sync escribe catálogo local: permiso de edición (ver
     * importar_catalogo). */
    verificar_permiso(&state.pool, AccionPermiso::CatalogoEdicion, &auth).await?;
    exigir_modo_bdp(&state, auth.user_id).await?;
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

    /* [287A-5] ExportArticles usa TypePrice (1-5), no el perfil de
     * CreateOrder/GetPosArticles. El valor persistido permite corregir un
     * catálogo vacío desde la UI sin tocar BDP. */
    let result = BdpSyncService::sync_catalog(
        &client,
        &state.pool,
        auth.user_id,
        config.bdp_catalog_price_type,
    )
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
pub(super) async fn sync_prices(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<BdpCatalogSyncResult>, AppError> {
    /* [149A-3/F4-B] Sync de precios escribe precio_tarifa1 local: permiso de
     * edición de catálogo. */
    verificar_permiso(&state.pool, AccionPermiso::CatalogoEdicion, &auth).await?;
    exigir_modo_bdp(&state, auth.user_id).await?;
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
pub(super) async fn sync_tables(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<SyncTablesRequest>,
) -> Result<Json<SyncTablesResult>, AppError> {
    /* [149A-3/F4-B] Con aplicar=true escribe ZonaSala+Mesa (plano): solo
     * Admin (sin AccionPermiso de plano; mismo patrón que bdp_push). */
    auth.require_role(&[UserRole::Admin])?;
    exigir_modo_bdp(&state, auth.user_id).await?;
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

    /* [039A-1/Q1] Bloqueo externo conocido (plan Q1.24): BDP sin suscripción de
     * salones responde "Subscripción no activada"; se clasifica como 409 honesto
     * (⏸ externo) en vez de 500 genérico. El error ocurre en la primera llamada
     * (GetRoomsTables), antes de cualquier escritura. */
    let result = BdpSyncService::sync_tables(&client, &state.pool, auth.user_id, req.aplicar)
        .await
        .map_err(|e| {
            if e.contains("Subscripción no activada") {
                AppError::Conflict(
                    "Salones BDP no disponibles para esta conexión (Subscripción no activada). \
                     No se realizaron cambios."
                        .into(),
                )
            } else {
                AppError::Internal(e)
            }
        })?;

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
pub(super) async fn get_menu_definition(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    exigir_modo_bdp(&state, auth.user_id).await?;
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
pub(super) async fn get_fastfood_definition(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    exigir_modo_bdp(&state, auth.user_id).await?;
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
pub(super) async fn get_pack_definition(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    exigir_modo_bdp(&state, auth.user_id).await?;
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
