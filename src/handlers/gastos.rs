/* 253A-5: Handlers de gastos — CRUD + categorías
283A-8: + digitalización de documentos vía Groq IA */

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    ActualizarGastoRequest, CategoriaGasto, CrearGastoRequest, DatosDocumentoExtraidos,
    DigitalizarDocumentoRequest, Gasto, GastosPaginados, GastosQuery, ProveedoresQuery,
};
use crate::services::{DigitalizacionService, GastoService};
use crate::AppState;

/// Crear un gasto
#[utoipa::path(
    post,
    path = "/api/gastos",
    tag = "Gastos",
    request_body = CrearGastoRequest,
    responses(
        (status = 201, description = "Gasto creado", body = Gasto),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 422, description = "Error de validación", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn crear_gasto(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CrearGastoRequest>,
) -> Result<(StatusCode, Json<Gasto>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let gasto = GastoService::create(&state.pool, auth.user_id, req).await?;
    Ok((StatusCode::CREATED, Json(gasto)))
}

/// Obtener un gasto por ID
#[utoipa::path(
    get,
    path = "/api/gastos/{id}",
    tag = "Gastos",
    params(("id" = Uuid, Path, description = "ID del gasto")),
    responses(
        (status = 200, description = "Gasto encontrado", body = Gasto),
        (status = 404, description = "Gasto no encontrado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn obtener_gasto(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Gasto>, AppError> {
    let gasto = GastoService::get(&state.pool, id, auth.user_id).await?;
    Ok(Json(gasto))
}

/// Listar gastos con paginación y filtros
#[utoipa::path(
    get,
    path = "/api/gastos",
    tag = "Gastos",
    params(GastosQuery),
    responses(
        (status = 200, description = "Lista de gastos", body = GastosPaginados),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_gastos(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<GastosQuery>,
) -> Result<Json<GastosPaginados>, AppError> {
    let gastos = GastoService::list(
        &state.pool,
        auth.user_id,
        params.page,
        params.per_page,
        params.desde,
        params.hasta,
        params.categoria_id,
        params.busqueda,
        params.tipo_documento,
        params.metodo_pago,
        params.sort_by,
        params.sort_order,
    )
    .await?;
    Ok(Json(gastos))
}

/// Actualizar un gasto
#[utoipa::path(
    put,
    path = "/api/gastos/{id}",
    tag = "Gastos",
    params(("id" = Uuid, Path, description = "ID del gasto")),
    request_body = ActualizarGastoRequest,
    responses(
        (status = 200, description = "Gasto actualizado", body = Gasto),
        (status = 404, description = "Gasto no encontrado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 422, description = "Error de validación", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn actualizar_gasto(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ActualizarGastoRequest>,
) -> Result<Json<Gasto>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let gasto = GastoService::update(&state.pool, id, auth.user_id, req).await?;
    Ok(Json(gasto))
}

/// Eliminar un gasto
#[utoipa::path(
    delete,
    path = "/api/gastos/{id}",
    tag = "Gastos",
    params(("id" = Uuid, Path, description = "ID del gasto")),
    responses(
        (status = 204, description = "Gasto eliminado"),
        (status = 404, description = "Gasto no encontrado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn eliminar_gasto(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    GastoService::delete(&state.pool, id, auth.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Listar categorías de gasto — endpoint público
#[utoipa::path(
    get,
    path = "/api/gastos/categorias",
    tag = "Gastos",
    operation_id = "listar_categorias_gasto",
    responses(
        (status = 200, description = "Lista de categorías", body = Vec<CategoriaGasto>)
    )
)]
pub async fn listar_categorias(
    State(state): State<AppState>,
) -> Result<Json<Vec<CategoriaGasto>>, AppError> {
    let cats = GastoService::categorias(&state.pool).await?;
    Ok(Json(cats))
}

/// Digitalizar un documento de gasto (factura, albarán, ticket) usando Groq IA
#[utoipa::path(
    post,
    path = "/api/gastos/digitalizar",
    tag = "Gastos",
    request_body = DigitalizarDocumentoRequest,
    responses(
        (status = 200, description = "Datos extraídos del documento", body = DatosDocumentoExtraidos),
        (status = 400, description = "Error de validación o API key faltante", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 500, description = "Error interno o del servicio de IA", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn digitalizar_documento(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<DigitalizarDocumentoRequest>,
) -> Result<Json<DatosDocumentoExtraidos>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let datos = DigitalizacionService::digitalizar(
        &state.pool,
        auth.user_id,
        &req.imagen_base64,
        &req.mime_type,
    )
    .await?;
    Ok(Json(datos))
}

/// Listar proveedores únicos para autocomplete
#[utoipa::path(
    get,
    path = "/api/gastos/proveedores",
    tag = "Gastos",
    params(ProveedoresQuery),
    responses(
        (status = 200, description = "Lista de proveedores únicos", body = Vec<String>),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_proveedores(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ProveedoresQuery>,
) -> Result<Json<Vec<String>>, AppError> {
    let proveedores = GastoService::proveedores(&state.pool, auth.user_id, params.busqueda).await?;
    Ok(Json(proveedores))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/gastos/categorias", get(listar_categorias))
        .route("/gastos/proveedores", get(listar_proveedores))
        .route("/gastos/digitalizar", post(digitalizar_documento))
        .route("/gastos", post(crear_gasto).get(listar_gastos))
        .route(
            "/gastos/:id",
            get(obtener_gasto)
                .put(actualizar_gasto)
                .delete(eliminar_gasto),
        )
}
