/* Stock local de artículos: listado, ajustes, inventario y conteos.
 * Parte de bdp_article_map (ver mod.rs). */

use super::{
    payload_inventario, payload_regularizacion, respuesta_cacheable, verificar_permiso,
    AccionPermiso, AjustarBdpArticleStockRequest, AjusteStockError, AppError, AppState, AuthUser,
    BdpArticleMapRepository, BdpArticleStock, BdpConteoInventario, BdpPushService,
    BdpStockInfoEntry, ConfiguracionService, ConteoInventarioCreado, CrearConteoInventarioRequest,
    Decimal, HeaderMap, Json, Path, RegistrarInventarioRequest, Response, State, Uuid, Validate,
    VisibilidadCache,
};

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
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let stock = BdpArticleMapRepository::listar_stock(&state.pool, auth.user_id, None).await?;
    /* [169A-5/D2.3] Stock del restaurante: 0,2 ms en origen, TTL 60 s. */
    respuesta_cacheable(&stock, VisibilidadCache::Publica, 60, &headers)
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
    let conteos = BdpArticleMapRepository::listar_conteos(&state.pool, auth.user_id, 50).await?;
    Ok(Json(conteos))
}

pub async fn obtener_conteo_inventario(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(conteo_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let detalle =
        BdpArticleMapRepository::obtener_conteo(&state.pool, auth.user_id, conteo_id).await?;
    let Some((conteo, lineas)) = detalle else {
        return Err(AppError::NotFound(
            "Conteo de inventario no encontrado".into(),
        ));
    };
    Ok(Json(
        serde_json::json!({ "conteo": conteo, "lineas": lineas }),
    ))
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
    let (conteo, lineas, reutilizado, aplicadas) = BdpArticleMapRepository::crear_conteo(
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
