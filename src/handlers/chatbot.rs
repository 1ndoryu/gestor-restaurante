/* [283A-2] Endpoints para chatbot externo — autenticados via API key (X-API-Key header).
 * El chatbot puede consultar disponibilidad, crear/buscar/cancelar reservas
 * y obtener info del restaurante sin necesitar credenciales de usuario. */

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::ApiKeyAuth;
use crate::models::{
    ChatbotBuscarReservasQuery, ChatbotCrearReservaRequest, ChatbotReservaResponse,
    DisponibilidadResponse, RestauranteInfoResponse,
};
use crate::services::{ChatbotService, NotificacionService};
use crate::AppState;

/// Query param para el endpoint de disponibilidad
#[derive(serde::Deserialize, utoipa::IntoParams)]
pub struct DisponibilidadQuery {
    pub fecha: chrono::NaiveDate,
}

#[utoipa::path(
    get,
    path = "/api/chatbot/disponibilidad",
    tag = "Chatbot",
    params(DisponibilidadQuery),
    responses(
        (status = 200, description = "Disponibilidad por franjas horarias", body = DisponibilidadResponse),
        (status = 401, description = "API key inválida o ausente")
    ),
    security(("api_key_auth" = []))
)]
pub async fn disponibilidad(
    State(state): State<AppState>,
    auth: ApiKeyAuth,
    Query(query): Query<DisponibilidadQuery>,
) -> Result<Json<DisponibilidadResponse>, AppError> {
    let resp = ChatbotService::disponibilidad(&state.pool, auth.user_id, query.fecha).await?;
    Ok(Json(resp))
}

#[utoipa::path(
    get,
    path = "/api/chatbot/restaurante",
    tag = "Chatbot",
    responses(
        (status = 200, description = "Info pública del restaurante", body = RestauranteInfoResponse),
        (status = 401, description = "API key inválida o ausente")
    ),
    security(("api_key_auth" = []))
)]
pub async fn restaurante_info(
    State(state): State<AppState>,
    auth: ApiKeyAuth,
) -> Result<Json<RestauranteInfoResponse>, AppError> {
    let resp = ChatbotService::restaurante_info(&state.pool, auth.user_id).await?;
    Ok(Json(resp))
}

#[utoipa::path(
    post,
    path = "/api/chatbot/reservas",
    tag = "Chatbot",
    operation_id = "crear_reserva_chatbot",
    request_body = ChatbotCrearReservaRequest,
    responses(
        (status = 201, description = "Reserva creada", body = ChatbotReservaResponse),
        (status = 401, description = "API key inválida o ausente"),
        (status = 422, description = "Error de validación")
    ),
    security(("api_key_auth" = []))
)]
pub async fn crear_reserva(
    State(state): State<AppState>,
    auth: ApiKeyAuth,
    Json(req): Json<ChatbotCrearReservaRequest>,
) -> Result<(StatusCode, Json<ChatbotReservaResponse>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let reserva = ChatbotService::crear_reserva(&state.pool, auth.user_id, req).await?;

    /* [283A-20] Notificación en tiempo real al panel del usuario */
    let _ = NotificacionService::emitir(
        &state.pool,
        &state.notif_tx,
        auth.user_id,
        "reserva_nueva",
        "Nueva reserva via chatbot",
        &format!(
            "{} — {} persona(s) el {} a las {}",
            reserva.nombre_cliente, reserva.num_personas, reserva.fecha, reserva.hora
        ),
    )
    .await;

    let resp = ChatbotReservaResponse {
        id: reserva.id,
        fecha: reserva.fecha,
        hora: reserva.hora,
        nombre_cliente: reserva.nombre_cliente,
        apellidos_cliente: reserva.apellidos_cliente,
        num_personas: reserva.num_personas,
        estado: reserva.estado,
        telefono: reserva.telefono,
        notas: reserva.notas,
        mesa_numero: reserva.num_mesa,
    };

    Ok((StatusCode::CREATED, Json(resp)))
}

#[utoipa::path(
    get,
    path = "/api/chatbot/reservas",
    tag = "Chatbot",
    operation_id = "buscar_reservas_chatbot",
    params(
        ("telefono" = Option<String>, Query, description = "Filtrar por teléfono"),
        ("nombre" = Option<String>, Query, description = "Filtrar por nombre/apellidos"),
        ("fecha" = Option<chrono::NaiveDate>, Query, description = "Filtrar por fecha")
    ),
    responses(
        (status = 200, description = "Lista de reservas", body = Vec<ChatbotReservaResponse>),
        (status = 401, description = "API key inválida o ausente")
    ),
    security(("api_key_auth" = []))
)]
pub async fn buscar_reservas(
    State(state): State<AppState>,
    auth: ApiKeyAuth,
    Query(query): Query<ChatbotBuscarReservasQuery>,
) -> Result<Json<Vec<ChatbotReservaResponse>>, AppError> {
    let resp = ChatbotService::buscar_reservas(
        &state.pool,
        auth.user_id,
        query.telefono.as_deref(),
        query.nombre.as_deref(),
        query.fecha,
    )
    .await?;
    Ok(Json(resp))
}

#[utoipa::path(
    get,
    path = "/api/chatbot/reservas/{id}",
    tag = "Chatbot",
    operation_id = "obtener_reserva_chatbot",
    params(("id" = Uuid, Path, description = "ID de la reserva")),
    responses(
        (status = 200, description = "Detalle de la reserva", body = ChatbotReservaResponse),
        (status = 401, description = "API key inválida o ausente"),
        (status = 404, description = "Reserva no encontrada")
    ),
    security(("api_key_auth" = []))
)]
pub async fn obtener_reserva(
    State(state): State<AppState>,
    auth: ApiKeyAuth,
    Path(id): Path<Uuid>,
) -> Result<Json<ChatbotReservaResponse>, AppError> {
    let resp = ChatbotService::obtener_reserva(&state.pool, id, auth.user_id).await?;
    Ok(Json(resp))
}

#[utoipa::path(
    delete,
    path = "/api/chatbot/reservas/{id}",
    tag = "Chatbot",
    operation_id = "cancelar_reserva_chatbot",
    params(("id" = Uuid, Path, description = "ID de la reserva a cancelar")),
    responses(
        (status = 200, description = "Reserva cancelada"),
        (status = 401, description = "API key inválida o ausente"),
        (status = 404, description = "Reserva no encontrada")
    ),
    security(("api_key_auth" = []))
)]
pub async fn cancelar_reserva(
    State(state): State<AppState>,
    auth: ApiKeyAuth,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    /* [283A-20] Obtener datos de la reserva antes de cancelar para la notificación */
    let reserva = ChatbotService::obtener_reserva(&state.pool, id, auth.user_id).await?;

    ChatbotService::cancelar_reserva(&state.pool, id, auth.user_id).await?;

    let _ = NotificacionService::emitir(
        &state.pool,
        &state.notif_tx,
        auth.user_id,
        "reserva_cancelada",
        "Reserva cancelada via chatbot",
        &format!(
            "{} — reserva del {} cancelada",
            reserva.nombre_cliente, reserva.fecha
        ),
    )
    .await;

    Ok(Json(
        serde_json::json!({ "ok": true, "message": "Reserva cancelada" }),
    ))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/chatbot/disponibilidad", get(disponibilidad))
        .route("/chatbot/restaurante", get(restaurante_info))
        .route(
            "/chatbot/reservas",
            post(crear_reserva).get(buscar_reservas),
        )
        .route(
            "/chatbot/reservas/:id",
            get(obtener_reserva).delete(cancelar_reserva),
        )
}
