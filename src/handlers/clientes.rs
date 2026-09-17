/* 263A-1: Handlers de clientes — CRUD CRM con búsqueda y paginación */

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::{
    guardar_listado_cache, invalidar_listado, leer_listado_cache, respuesta_cacheable,
    respuesta_cacheable_bytes, AuthUser, VisibilidadCache, ENDPOINT_CLIENTES,
};
use crate::models::{
    ActualizarClienteRequest, BdpPuntoCliente, Cliente, ClientesQuery,
    CrearClienteRequest, MergeClientesRequest, MergeClientesResponse, SumarPuntosRequest,
};
use crate::repositories::BdpPuntoClienteRepository;
use crate::services::{payload_puntos, BdpPushService, ClienteService};
use crate::AppState;

/// Crear un cliente
#[utoipa::path(
    post,
    path = "/api/clientes",
    tag = "Clientes",
    request_body = CrearClienteRequest,
    responses(
        (status = 201, description = "Cliente creado", body = Cliente),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 422, description = "Error de validación", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn crear_cliente(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CrearClienteRequest>,
) -> Result<(StatusCode, Json<Cliente>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let cliente = ClienteService::create(&state.pool, auth.user_id, req).await?;
    /* [179A-1/F1] Toda escritura de clientes invalida su listado default. */
    invalidar_listado(&state, auth.user_id, ENDPOINT_CLIENTES).await;
    Ok((StatusCode::CREATED, Json(cliente)))
}

/// Obtener un cliente por ID
#[utoipa::path(
    get,
    path = "/api/clientes/{id}",
    tag = "Clientes",
    params(("id" = Uuid, Path, description = "ID del cliente")),
    responses(
        (status = 200, description = "Cliente encontrado", body = Cliente),
        (status = 404, description = "Cliente no encontrado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn obtener_cliente(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Cliente>, AppError> {
    let cliente = ClienteService::get(&state.pool, id, auth.user_id).await?;
    Ok(Json(cliente))
}

/// Listar clientes con paginación y búsqueda
#[utoipa::path(
    get,
    path = "/api/clientes",
    tag = "Clientes",
    params(ClientesQuery),
    responses(
        (status = 200, description = "Lista de clientes", body = ClientesPaginados),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_clientes(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ClientesQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    /* [179A-1/F1] Combo default: caché en app, sin tocar postgres. */
    let es_default = query.page == 1
        && query.per_page == 20
        && query.busqueda.is_none()
        && query.sort_by.is_none()
        && query.sort_order.is_none();
    if es_default {
        if let Some(bytes) = leer_listado_cache(&state, auth.user_id, ENDPOINT_CLIENTES).await {
            return respuesta_cacheable_bytes(bytes, VisibilidadCache::Publica, 15, &headers);
        }
    }
    let resultado = ClienteService::list(&state.pool, auth.user_id, query).await?;
    if es_default {
        if let Ok(bytes) = serde_json::to_vec(&resultado) {
            guardar_listado_cache(&state, auth.user_id, ENDPOINT_CLIENTES, bytes).await;
        }
    }
    respuesta_cacheable(&resultado, VisibilidadCache::Publica, 15, &headers)
}

/// Actualizar un cliente
#[utoipa::path(
    put,
    path = "/api/clientes/{id}",
    tag = "Clientes",
    params(("id" = Uuid, Path, description = "ID del cliente")),
    request_body = ActualizarClienteRequest,
    responses(
        (status = 200, description = "Cliente actualizado", body = Cliente),
        (status = 404, description = "Cliente no encontrado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn actualizar_cliente(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ActualizarClienteRequest>,
) -> Result<Json<Cliente>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let cliente = ClienteService::update(&state.pool, id, auth.user_id, req).await?;
    /* [179A-1/F1] Toda escritura de clientes invalida su listado default. */
    invalidar_listado(&state, auth.user_id, ENDPOINT_CLIENTES).await;
    Ok(Json(cliente))
}

/// Eliminar un cliente
#[utoipa::path(
    delete,
    path = "/api/clientes/{id}",
    tag = "Clientes",
    params(("id" = Uuid, Path, description = "ID del cliente")),
    responses(
        (status = 204, description = "Cliente eliminado"),
        (status = 404, description = "Cliente no encontrado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn eliminar_cliente(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    ClienteService::delete(&state.pool, id, auth.user_id).await?;
    /* [179A-1/F1] Toda escritura de clientes invalida su listado default. */
    invalidar_listado(&state, auth.user_id, ENDPOINT_CLIENTES).await;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/clientes", post(crear_cliente).get(listar_clientes))
        .route("/clientes/merge", post(merge_clientes))
        .route(
            "/clientes/:id",
            get(obtener_cliente)
                .put(actualizar_cliente)
                .delete(eliminar_cliente),
        )
        .route(
            "/clientes/:id/puntos",
            get(listar_puntos_cliente).post(sumar_puntos_cliente),
        )
}

/* [263A-26] Merge de dos clientes duplicados.
 * Absorbe origen en destino: migra reservas, etiquetas, campañas,
 * rellena campos vacíos y elimina el origen. Operación atómica en transacción. */
/// Fusionar dos clientes duplicados
#[utoipa::path(
    post,
    path = "/api/clientes/merge",
    tag = "Clientes",
    request_body = MergeClientesRequest,
    responses(
        (status = 200, description = "Clientes fusionados", body = MergeClientesResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 404, description = "Cliente no encontrado", body = ErrorResponse),
        (status = 422, description = "Error de validación", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn merge_clientes(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<MergeClientesRequest>,
) -> Result<Json<MergeClientesResponse>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let resp = ClienteService::merge(&state.pool, auth.user_id, req).await?;
    /* [179A-1/F1] El merge elimina un cliente: invalida el listado default. */
    invalidar_listado(&state, auth.user_id, ENDPOINT_CLIENTES).await;
    Ok(Json(resp))
}

/* [198A-1/D9] Fidelización: saldo y movimientos de puntos del cliente. El
 * ledger local permite operar sin BDP; el push AddPoints se encola solo si el
 * cliente tiene `bdp_customer_code`. */
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct PuntosClienteResponse {
    pub saldo: rust_decimal::Decimal,
    pub historial: Vec<BdpPuntoCliente>,
}

#[utoipa::path(
    get,
    path = "/api/clientes/{id}/puntos",
    tag = "Clientes",
    params(("id" = Uuid, Path, description = "ID del cliente")),
    responses(
        (status = 200, description = "Saldo y movimientos de puntos", body = PuntosClienteResponse),
        (status = 404, description = "Cliente no encontrado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_puntos_cliente(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<PuntosClienteResponse>, AppError> {
    /* Verifica que el cliente existe (404 si no). */
    let _ = ClienteService::get(&state.pool, id, auth.user_id).await?;
    let saldo = BdpPuntoClienteRepository::saldo(&state.pool, auth.user_id, id).await?;
    let historial = BdpPuntoClienteRepository::listar(&state.pool, auth.user_id, id).await?;
    Ok(Json(PuntosClienteResponse { saldo, historial }))
}

#[utoipa::path(
    post,
    path = "/api/clientes/{id}/puntos",
    tag = "Clientes",
    params(("id" = Uuid, Path, description = "ID del cliente")),
    request_body = SumarPuntosRequest,
    responses(
        (status = 200, description = "Movimiento de puntos registrado", body = BdpPuntoCliente),
        (status = 404, description = "Cliente no encontrado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn sumar_puntos_cliente(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<SumarPuntosRequest>,
) -> Result<Json<BdpPuntoCliente>, AppError> {
    if req.points_added == rust_decimal::Decimal::ZERO {
        return Err(AppError::Validation(
            "La cantidad de puntos no puede ser cero".into(),
        ));
    }
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let cliente = ClienteService::get(&state.pool, id, auth.user_id).await?;

    let punto = BdpPuntoClienteRepository::registrar(
        &state.pool,
        auth.user_id,
        id,
        cliente.bdp_customer_code.unwrap_or(0),
        req.points_added,
        &req.reason,
    )
    .await?;

    /* M16 (análogo): solo encolar si el cliente tiene código BDP; si no, el
     * movimiento queda local (saldo visible sin BDP). */
    if let Some(codigo) = cliente.bdp_customer_code {
        let payload = payload_puntos(i64::from(codigo), req.points_added, &req.reason)
            .map_err(AppError::Internal)?;
        BdpPushService::encolar(
            &state.pool,
            auth.user_id,
            crate::services::bdp_push::DOMINIO_CLIENTE_PUNTOS,
            &id.to_string(),
            crate::services::bdp_push::OPERACION_PUNTOS,
            &payload,
        )
        .await
        .map_err(AppError::Internal)?;
    }

    Ok(Json(punto))
}
