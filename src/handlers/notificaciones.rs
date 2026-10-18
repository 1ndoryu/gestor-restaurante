/* [283A-20] Handler de notificaciones en tiempo real via SSE.
 * Endpoints:
 *   GET  /api/notificaciones         — listar recientes (JWT)
 *   GET  /api/notificaciones/stream  — SSE stream en tiempo real (JWT)
 *   GET  /api/notificaciones/count   — contar no leídas (JWT)
 *   PATCH /api/notificaciones/:id/leer — marcar una como leída (JWT)
 *   PATCH /api/notificaciones/leer-todas — marcar todas como leídas (JWT) */

use std::convert::Infallible;

use axum::extract::{Path, State};
use axum::response::sse::{Event, Sse};
use axum::routing::{get, patch};
use axum::{Json, Router};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::Notificacion;
use crate::services::NotificacionService;
use crate::AppState;

/// Query param para limitar resultados
#[derive(serde::Deserialize, utoipa::IntoParams)]
pub struct NotificacionesQuery {
    /// Máximo de notificaciones a devolver (default: 50)
    pub limite: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/notificaciones",
    tag = "Notificaciones",
    params(NotificacionesQuery),
    responses(
        (status = 200, description = "Lista de notificaciones", body = Vec<Notificacion>),
        (status = 401, description = "No autorizado")
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_notificaciones(
    State(state): State<AppState>,
    auth: AuthUser,
    axum::extract::Query(query): axum::extract::Query<NotificacionesQuery>,
) -> Result<Json<Vec<Notificacion>>, AppError> {
    let limite = query.limite.unwrap_or(50).min(200);
    let notifs = NotificacionService::listar(&state.pool, auth.user_id, limite).await?;
    Ok(Json(notifs))
}

/// Respuesta del conteo de no leídas
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ConteoNoLeidas {
    pub count: i64,
}

#[utoipa::path(
    get,
    path = "/api/notificaciones/count",
    tag = "Notificaciones",
    responses(
        (status = 200, description = "Conteo de no leídas", body = ConteoNoLeidas),
        (status = 401, description = "No autorizado")
    ),
    security(("bearer_auth" = []))
)]
pub async fn contar_no_leidas(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ConteoNoLeidas>, AppError> {
    let count = NotificacionService::contar_no_leidas(&state.pool, auth.user_id).await?;
    Ok(Json(ConteoNoLeidas { count }))
}

#[utoipa::path(
    patch,
    path = "/api/notificaciones/{id}/leer",
    tag = "Notificaciones",
    params(("id" = Uuid, Path, description = "ID de la notificación")),
    responses(
        (status = 200, description = "Marcada como leída"),
        (status = 401, description = "No autorizado"),
        (status = 404, description = "No encontrada")
    ),
    security(("bearer_auth" = []))
)]
pub async fn marcar_leida(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    NotificacionService::marcar_leida(&state.pool, id, auth.user_id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[utoipa::path(
    patch,
    path = "/api/notificaciones/leer-todas",
    tag = "Notificaciones",
    responses(
        (status = 200, description = "Todas marcadas como leídas"),
        (status = 401, description = "No autorizado")
    ),
    security(("bearer_auth" = []))
)]
pub async fn marcar_todas_leidas(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let count = NotificacionService::marcar_todas_leidas(&state.pool, auth.user_id).await?;
    Ok(Json(serde_json::json!({ "ok": true, "marcadas": count })))
}

/// SSE stream que emite notificaciones en tiempo real para el usuario autenticado.
/// El cliente se conecta con `EventSource` y recibe eventos JSON.
/// Como `EventSource` no soporta headers, el JWT se pasa como query param `?token=`.
#[utoipa::path(
    get,
    path = "/api/notificaciones/stream",
    tag = "Notificaciones",
    params(("token" = String, Query, description = "JWT token (EventSource no soporta headers)")),
    responses(
        (status = 200, description = "Stream SSE de notificaciones"),
        (status = 401, description = "No autorizado")
    )
)]
pub async fn stream_notificaciones(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<SseTokenQuery>,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, AppError> {
    /* Validar JWT del query param */
    let claims = crate::services::AuthService::verify_token(&params.token, &state.jwt_secret)?;
    let user_id = claims.sub;

    /* Cada conexión recibe solo los avisos de su usuario */
    let rx = state.notif_hub.suscribir(user_id);

    let stream = ReceiverStream::new(rx).filter_map(|notif| {
        let json = serde_json::to_string(&notif).ok()?;
        Some(Ok(Event::default().event("notificacion").data(json)))
    });

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(30))
            .text("ping"),
    ))
}

/// Query param para el SSE stream (`EventSource` no soporta headers)
#[derive(serde::Deserialize, utoipa::IntoParams)]
pub struct SseTokenQuery {
    pub token: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/notificaciones", get(listar_notificaciones))
        .route("/notificaciones/count", get(contar_no_leidas))
        .route("/notificaciones/stream", get(stream_notificaciones))
        .route("/notificaciones/:id/leer", patch(marcar_leida))
        .route("/notificaciones/leer-todas", patch(marcar_todas_leidas))
}
