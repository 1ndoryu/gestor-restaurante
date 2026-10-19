/* 253A-5: Ventas — CRUD, listados, anulación y reintento de sync con Haddock. */

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::{
    guardar_listado_cache, invalidar_listado, leer_listado_cache, respuesta_cacheable,
    respuesta_cacheable_bytes, AuthUser, VisibilidadCache, ENDPOINT_VENTAS,
};
use crate::models::{
    ActualizarVentaRequest, AnularVentaRequest, CrearVentaRequest, Venta, VentaLinea, VentasQuery,
};
use crate::repositories::venta::FiltrosListadoVentas;
use crate::services::{verificar_permiso, AccionPermiso, DashboardService, VentaService};
use crate::AppState;

/// Crear una venta
#[utoipa::path(
    post,
    path = "/api/ventas",
    tag = "Ventas",
    request_body = CrearVentaRequest,
    responses(
        (status = 201, description = "Venta creada", body = Venta),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 422, description = "Error de validación", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn crear_venta(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CrearVentaRequest>,
) -> Result<(StatusCode, Json<Venta>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let venta = VentaService::create(&state.pool, auth.user_id, req).await?;
    /* [169A-5/D2.2] Toda escritura de ventas invalida el resumen cacheado. */
    DashboardService::invalidar_resumen(&state, auth.user_id).await;
    /* [179A-1/F1] Toda escritura de ventas invalida su listado default. */
    invalidar_listado(&state, auth.user_id, ENDPOINT_VENTAS).await;
    Ok((StatusCode::CREATED, Json(venta)))
}

/// Obtener una venta por ID
#[utoipa::path(
    get,
    path = "/api/ventas/{id}",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    responses(
        (status = 200, description = "Venta encontrada", body = Venta),
        (status = 404, description = "Venta no encontrada", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn obtener_venta(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Venta>, AppError> {
    let venta = VentaService::get(&state.pool, id, auth.user_id).await?;
    Ok(Json(venta))
}

#[utoipa::path(
    get,
    path = "/api/ventas/{id}/lineas",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    responses(
        (status = 200, description = "Líneas de la venta", body = [VentaLinea]),
        (status = 404, description = "Venta no encontrada", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn obtener_lineas_venta(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<VentaLinea>>, AppError> {
    VentaService::get(&state.pool, id, auth.user_id).await?;
    let lineas =
        crate::repositories::VentaLineaRepository::listar_por_venta(&state.pool, id).await?;
    Ok(Json(lineas))
}

/// Listar ventas con paginación y filtros de fecha
#[utoipa::path(
    get,
    path = "/api/ventas",
    tag = "Ventas",
    params(VentasQuery),
    responses(
        (status = 200, description = "Lista de ventas", body = VentasPaginadas),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_ventas(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<VentasQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    /* [179A-1/F1] Combo default (1ª página, sin filtros ni orden): se sirve
     * de la caché en app y no toca postgres. Cualquier otro combo va a DB. */
    let es_default = params.page == 1
        && params.per_page == 20
        && params.desde.is_none()
        && params.hasta.is_none()
        && params.busqueda.is_none()
        && params.turno.is_none()
        && params.canal.is_none()
        && params.metodo_pago.is_none()
        && params.estado_haddock.is_none()
        && params.estado_bdp.is_none()
        && params.sort_by.is_none()
        && params.sort_order.is_none();
    if es_default {
        if let Some(bytes) = leer_listado_cache(&state, auth.user_id, ENDPOINT_VENTAS).await {
            return respuesta_cacheable_bytes(bytes, VisibilidadCache::Publica, 15, &headers);
        }
    }
    let filtros = FiltrosListadoVentas {
        page: params.page,
        per_page: params.per_page,
        desde: params.desde,
        hasta: params.hasta,
        busqueda: params.busqueda.as_deref(),
        turno: params.turno.as_deref(),
        canal: params.canal.as_deref(),
        metodo_pago: params.metodo_pago.as_deref(),
        estado_haddock: params.estado_haddock.as_deref(),
        estado_bdp: params.estado_bdp.as_deref(),
        sort_by: params.sort_by.as_deref(),
        sort_order: params.sort_order.as_deref(),
    };
    let ventas = VentaService::list(&state.pool, auth.user_id, filtros).await?;
    if es_default {
        if let Ok(bytes) = serde_json::to_vec(&ventas) {
            guardar_listado_cache(&state, auth.user_id, ENDPOINT_VENTAS, bytes).await;
        }
    }
    /* [169A-5/D2.3] Listado base: mismo cuerpo para todo el restaurante,
     * TTL corto (15 s) porque cambia con cada venta. */
    respuesta_cacheable(&ventas, VisibilidadCache::Publica, 15, &headers)
}

/// Actualizar una venta
#[utoipa::path(
    put,
    path = "/api/ventas/{id}",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    request_body = ActualizarVentaRequest,
    responses(
        (status = 200, description = "Venta actualizada", body = Venta),
        (status = 404, description = "Venta no encontrada", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 422, description = "Error de validación", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn actualizar_venta(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ActualizarVentaRequest>,
) -> Result<Json<Venta>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let venta = VentaService::update(&state.pool, id, auth.user_id, req).await?;
    DashboardService::invalidar_resumen(&state, auth.user_id).await;
    /* [179A-1/F1] Toda escritura de ventas invalida su listado default. */
    invalidar_listado(&state, auth.user_id, ENDPOINT_VENTAS).await;
    Ok(Json(venta))
}

/// Eliminar una venta
#[utoipa::path(
    delete,
    path = "/api/ventas/{id}",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    responses(
        (status = 204, description = "Venta eliminada"),
        (status = 404, description = "Venta no encontrada", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn eliminar_venta(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    /* [128A-1/F8-2] Escritura sensible (histórico fiscal local): el DELETE
     * exige el mismo permiso que la anulación. Decisión documentada: reusar
     * `AnulacionVentas` en vez de añadir una acción dedicada (misma clase de
     * escritura destructiva sobre ventas, default 'admin'). */
    verificar_permiso(&state.pool, AccionPermiso::AnulacionVentas, &auth).await?;
    VentaService::delete(&state.pool, id, auth.user_id).await?;
    DashboardService::invalidar_resumen(&state, auth.user_id).await;
    /* [179A-1/F1] Toda escritura de ventas invalida su listado default. */
    invalidar_listado(&state, auth.user_id, ENDPOINT_VENTAS).await;
    Ok(StatusCode::NO_CONTENT)
}

/* [128A-1/F4] POST /api/ventas/:id/anular — Anulación local de ventas (D4).
 * Modalidades configuradas en `anulacion_modalidad` (credito_completo default |
 * estado_solo). Confirmación dinámica `ANULAR {id}` (patrón PAGAR/FACTURAR).
 * M9: solo ventas no facturadas. C3=b: sin llamada CancelOrder. */
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct AnularVentaResponse {
    pub venta: Venta,
    pub anulada: bool,
}

#[utoipa::path(
    post,
    path = "/api/ventas/{id}/anular",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    request_body = AnularVentaRequest,
    responses(
        (status = 200, description = "Venta anulada", body = AnularVentaResponse),
        (status = 404, description = "Venta no encontrada", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 409, description = "Venta facturada o idempotency_key ya usada con otro resultado", body = ErrorResponse),
        (status = 422, description = "Motivo obligatorio en credito_completo", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn anular_venta(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AnularVentaRequest>,
) -> Result<Json<AnularVentaResponse>, AppError> {
    /* [128A-1/F8] Permiso por acción: anulación de ventas (D8/M17). */
    verificar_permiso(&state.pool, AccionPermiso::AnulacionVentas, &auth).await?;
    let venta = VentaService::anular(&state.pool, id, auth.user_id, req).await?;
    DashboardService::invalidar_resumen(&state, auth.user_id).await;
    /* [179A-1/F1] Toda escritura de ventas invalida su listado default. */
    invalidar_listado(&state, auth.user_id, ENDPOINT_VENTAS).await;
    Ok(Json(AnularVentaResponse {
        anulada: venta.anulada,
        venta,
    }))
}

/// Reintentar sincronización con Haddock
#[utoipa::path(
    post,
    path = "/api/ventas/{id}/haddock-sync",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    responses(
        (status = 200, description = "Sincronización completada", body = Venta),
        (status = 404, description = "Venta no encontrada", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 422, description = "Sync no habilitado o sin token", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn reintentar_sync_haddock(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Venta>, AppError> {
    let venta = VentaService::retry_haddock_sync(&state.pool, id, auth.user_id).await?;
    Ok(Json(venta))
}
