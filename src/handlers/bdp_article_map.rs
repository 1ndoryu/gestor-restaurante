/* [F1.5] Handlers CRUD para mapeos de artículos Glory → BDP.
 * GET    /api/bdp/article-maps              — listar todos los mapeos
 * POST   /api/bdp/article-maps              — crear/upsert un mapeo
 * POST   /api/bdp/article-stock/ajustar      — ajuste manual de stock local (F3)
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
use rust_decimal::Decimal;
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    ActualizarBdpArticleMapRequest, AjustarBdpArticleStockRequest, BdpArticleMap, BdpArticleStock,
    BdpConteoInventario, ConteoInventarioCreado, CrearBdpArticleMapRequest,
    CrearConteoInventarioRequest, RegistrarInventarioRequest,
};
use crate::repositories::{AjusteStockError, BdpArticleMapRepository};
use crate::services::bdp_weblink_catalog::{
    BdpGetFastfoodRequest, BdpGetMenuRequest, BdpGetPackRequest, BdpStockInfoEntry,
};
use crate::services::{
    payload_crear_articulo, payload_inventario, payload_modificar_articulo, payload_regularizacion,
    verificar_permiso, AccionPermiso, BdpCatalogSyncResult, BdpPushService, BdpSyncService,
    BdpWeblinkClient, ConfiguracionService, SyncTablesResult,
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
        .route(
            "/bdp/article-stock",
            get(listar_article_stock).post(ajustar_stock),
        )
        .route("/bdp/inventario", axum::routing::post(registrar_inventario))
        /* [208A-2/C3] Conteos persistidos (D3/D4): listar, guardar+aplicar, retomar. */
        .route(
            "/bdp/inventario/conteos",
            get(listar_conteos_inventario).post(crear_conteo_inventario),
        )
        .route(
            "/bdp/inventario/conteos/:id",
            get(obtener_conteo_inventario),
        )
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

/* [128A-1/F3] Ajuste manual de stock local. Funciona sin BDP: escribe en
 * `bdp_article_stock` (fuente de verdad local por almacén) y audita en
 * `bdp_audit_log` (operacion='stock_ajuste', direccion='internal').
 * `bdp_article_map.stock_actual` (snapshot BDP) nunca se pisa. */
#[utoipa::path(
    post,
    path = "/api/bdp/article-stock/ajustar",
    tag = "BDP Mapeos",
    request_body = AjustarBdpArticleStockRequest,
    responses(
        (status = 200, description = "Stock ajustado", body = BdpArticleStock),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 409, description = "Idempotency key ya usada con otro resultado", body = ErrorResponse),
        (status = 422, description = "Error de validación", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn ajustar_stock(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<AjustarBdpArticleStockRequest>,
) -> Result<Json<BdpArticleStock>, AppError> {
    /* [128A-1/F8] Permiso por acción: ajuste de stock (D8/M17). */
    verificar_permiso(&state.pool, AccionPermiso::StockAjuste, &auth).await?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    if req.delta == rust_decimal::Decimal::ZERO {
        return Err(AppError::Validation(
            "delta no puede ser cero (usa un valor positivo o negativo)".into(),
        ));
    }

    let (stock, _audit_id, resultado_previo) = BdpArticleMapRepository::ajustar_stock(
        &state.pool,
        auth.user_id,
        &req.articulo_glory_codigo,
        req.delta,
        &req.motivo,
        req.warehouse_id.as_deref(),
        req.idempotency_key.as_deref(),
    )
    .await?;

    /* [128A-1/F3] Idempotencia: un reintento con la misma clave y resultado
     * previo 'exito' es un éxito idempotente (patrón ventas.rs C1-6); si la
     * clave ya se usó con otro resultado, es un conflicto. */
    if let Some(resultado) = resultado_previo {
        if resultado != "exito" {
            return Err(AppError::Conflict(format!(
                "idempotency_key ya usada (resultado previo: {resultado})"
            )));
        }
    }

    /* [198A-1/F1] Encolar el ajuste para push BDP. Solo si el artículo tiene
     * código BDP numérico (un artículo local puro no puede regularizarse en BDP
     * todavía — queda para F3/F4). El worker de flush decide según el modo. */
    if let Some(map) = BdpArticleMapRepository::buscar_por_codigo(
        &state.pool,
        auth.user_id,
        &req.articulo_glory_codigo,
    )
    .await?
    {
        if let Ok(bdp_code) = map.articulo_bdp_codigo.trim().parse::<i64>() {
            let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
            let payload =
                payload_regularizacion(&config, bdp_code, req.delta).map_err(AppError::Internal)?;
            BdpPushService::encolar(
                &state.pool,
                auth.user_id,
                crate::services::bdp_push::DOMINIO_STOCK,
                &map.articulo_glory_codigo,
                crate::services::bdp_push::OPERACION_REGULARIZAR,
                &payload,
            )
            .await
            .map_err(AppError::Internal)?;
        }
    }

    Ok(Json(stock))
}

/* [198A-1/D6] Inventario (conteo físico) → UpdateMassiveInventory. Localmente
 * no persiste el conteo (la diferencia esperada/contada se calcula en la UI);
 * el endpoint resuelve los códigos BDP, construye el lote y encola
 * `stock/inventario`. El worker no envía nada en standalone (independencia).
 * Los artículos locales puros (sin código BDP numérico) se omiten y se reportan. */
pub async fn registrar_inventario(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<RegistrarInventarioRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    verificar_permiso(&state.pool, AccionPermiso::StockAjuste, &auth).await?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let codigos: Vec<String> = req
        .articulos
        .iter()
        .map(|a| a.articulo_glory_codigo.clone())
        .collect();
    let resueltos =
        BdpArticleMapRepository::codigos_bdp_para_glory(&state.pool, auth.user_id, &codigos)
            .await?;

    let contadas: std::collections::HashMap<String, rust_decimal::Decimal> = req
        .articulos
        .iter()
        .map(|a| (a.articulo_glory_codigo.clone(), a.unidades_contadas))
        .collect();

    let mut lineas = Vec::new();
    for (glory, code) in &resueltos {
        if let Some(unidades) = contadas.get(glory) {
            lineas.push(BdpStockInfoEntry {
                article: *code,
                units: *unidades,
            });
        }
    }

    if lineas.is_empty() {
        return Err(AppError::Validation(
            "Ningún artículo del inventario tiene código BDP numérico; no hay lote que enviar"
                .into(),
        ));
    }

    let omitidos = req.articulos.len() - lineas.len();
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
    let payload = payload_inventario(&config, lineas).map_err(AppError::Internal)?;
    BdpPushService::encolar(
        &state.pool,
        auth.user_id,
        crate::services::bdp_push::DOMINIO_STOCK,
        "conteo",
        crate::services::bdp_push::OPERACION_INVENTARIO,
        &payload,
    )
    .await
    .map_err(AppError::Internal)?;

    Ok(Json(serde_json::json!({
        "enviados": resueltos.len(),
        "omitidos_sin_bdp": omitidos,
    })))
}

/* [208A-2/C3] Conteos de inventario persistidos (decisiones D3/D4).
 * GET /api/bdp/inventario/conteos        — historial de conteos (cabeceras).
 * POST /api/bdp/inventario/conteos       — guardar conteo: persiste líneas,
 *   aplica la diferencia al stock local (motivo 'conteo') y, si hay códigos
 *   BDP, encola el envío (el worker no envía nada en standalone).
 * GET /api/bdp/inventario/conteos/:id    — detalle con líneas (para retomar). */
pub async fn listar_conteos_inventario(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<BdpConteoInventario>>, AppError> {
    let conteos =
        BdpArticleMapRepository::listar_conteos(&state.pool, auth.user_id, 50).await?;
    Ok(Json(conteos))
}

pub async fn obtener_conteo_inventario(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(conteo_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let detalle = BdpArticleMapRepository::obtener_conteo(&state.pool, auth.user_id, conteo_id)
        .await?;
    let Some((conteo, lineas)) = detalle else {
        return Err(AppError::NotFound(
            "Conteo de inventario no encontrado".into(),
        ));
    };
    Ok(Json(serde_json::json!({ "conteo": conteo, "lineas": lineas })))
}

pub async fn crear_conteo_inventario(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CrearConteoInventarioRequest>,
) -> Result<Json<ConteoInventarioCreado>, AppError> {
    verificar_permiso(&state.pool, AccionPermiso::StockAjuste, &auth).await?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let articulos: Vec<(String, Decimal)> = req
        .articulos
        .iter()
        .map(|a| (a.articulo_glory_codigo.clone(), a.unidades_contadas))
        .collect();
    let (conteo, lineas, reutilizado, aplicadas) =
        BdpArticleMapRepository::crear_conteo(
            &state.pool,
            auth.user_id,
            req.observaciones.as_deref().unwrap_or(""),
            req.idempotency_key.as_deref(),
            &articulos,
        )
        .await
        .map_err(|e| match e {
            AjusteStockError::StockNegativo(m) => AppError::Validation(m),
            AjusteStockError::Db(db) => {
                AppError::Internal(format!("No se pudo guardar el conteo: {db}"))
            }
        })?;

    /* Encolar el envío de las líneas con código BDP (misma lógica que
     * registrar_inventario; el worker no envía nada en standalone). */
    let codigos: Vec<String> = lineas
        .iter()
        .map(|l| l.articulo_glory_codigo.clone())
        .collect();
    let resueltos =
        BdpArticleMapRepository::codigos_bdp_para_glory(&state.pool, auth.user_id, &codigos)
            .await?;
    let contadas: std::collections::HashMap<String, Decimal> = lineas
        .iter()
        .map(|l| (l.articulo_glory_codigo.clone(), l.contado))
        .collect();
    let mut lotes = Vec::new();
    for (glory, code) in &resueltos {
        if let Some(unidades) = contadas.get(glory) {
            lotes.push(BdpStockInfoEntry {
                article: *code,
                units: *unidades,
            });
        }
    }
    let omitidos_sin_bdp = lineas.len() - lotes.len();
    let mut encolados = 0usize;
    if !lotes.is_empty() {
        let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
        let payload = payload_inventario(&config, lotes).map_err(AppError::Internal)?;
        BdpPushService::encolar(
            &state.pool,
            auth.user_id,
            crate::services::bdp_push::DOMINIO_STOCK,
            "conteo",
            crate::services::bdp_push::OPERACION_INVENTARIO,
            &payload,
        )
        .await
        .map_err(AppError::Internal)?;
        encolados = resueltos.len();
    }

    Ok(Json(ConteoInventarioCreado {
        conteo,
        lineas,
        reutilizado,
        aplicadas,
        encolados,
        omitidos_sin_bdp,
    }))
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
