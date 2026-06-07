/* 253A-5: Handlers de ventas — CRUD endpoints */

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    ActualizarVentaRequest, CrearVentaRequest, Venta, VentasPaginadas, VentasQuery,
};
use crate::services::VentaService;
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
) -> Result<Json<VentasPaginadas>, AppError> {
    let ventas = VentaService::list(
        &state.pool,
        auth.user_id,
        params.page,
        params.per_page,
        params.desde,
        params.hasta,
        params.busqueda,
        params.turno,
        params.canal,
        params.metodo_pago,
        params.estado_haddock,
        params.sort_by,
        params.sort_order,
    )
    .await?;
    Ok(Json(ventas))
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
    VentaService::delete(&state.pool, id, auth.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
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

/// Reintentar sincronización con `BDP` `WebLink`
#[utoipa::path(
    post,
    path = "/api/ventas/{id}/bdp-sync",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    responses(
        (status = 200, description = "Sincronización BDP completada", body = Venta),
        (status = 404, description = "Venta no encontrada", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 422, description = "BDP sync no habilitado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn reintentar_sync_bdp(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Venta>, AppError> {
    let venta = VentaService::retry_bdp_sync(&state.pool, id, auth.user_id).await?;
    Ok(Json(venta))
}

/* [263A-15] Axum 0.7 (matchit 0.7.x) usa :param, no {param}.
 * Todas las rutas con path params corregidas de {id} a :id.
 * Las anotaciones #[utoipa::path] mantienen {id} (sintaxis OpenAPI, no afecta routing). */
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/ventas", post(crear_venta).get(listar_ventas))
        .route(
            "/ventas/:id",
            get(obtener_venta)
                .put(actualizar_venta)
                .delete(eliminar_venta),
        )
        .route("/ventas/:id/haddock-sync", post(reintentar_sync_haddock))
        .route("/ventas/:id/bdp-sync", post(reintentar_sync_bdp))
}
