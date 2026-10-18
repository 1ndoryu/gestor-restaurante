/* [283A-26] Endpoint para reportar errores desde el frontend.
 * Los reportes se envían por email al correo configurado en ERROR_REPORT_EMAIL.
 * Requiere autenticación para incluir datos del usuario en el reporte. */

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::services::email::EmailService;
use crate::AppState;

use super::rate_limit::{limitador_escrituras, limitar};

#[derive(Deserialize, ToSchema)]
pub struct ReportarErrorRequest {
    /// Mensaje de error o descripción del problema
    pub mensaje: String,
    /// Stack trace (si disponible)
    pub stack: Option<String>,
    /// URL donde ocurrió el error
    pub url: String,
    /// User-Agent del navegador
    pub navegador: String,
}

#[derive(Serialize, ToSchema)]
pub struct ReportarErrorResponse {
    /// Siempre true — si SMTP no está configurado se loguea pero no falla
    pub ok: bool,
}

/// Reportar un error desde el frontend — envía email al administrador
#[utoipa::path(
    post,
    path = "/api/reportar-error",
    request_body = ReportarErrorRequest,
    responses(
        (status = 200, body = ReportarErrorResponse),
    ),
    security(("bearer" = []))
)]
pub async fn reportar_error(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<ReportarErrorRequest>,
) -> Result<Json<ReportarErrorResponse>, AppError> {
    let Some(smtp) = &state.config.smtp else {
        tracing::warn!("Error reportado pero SMTP no configurado: {}", req.mensaje);
        return Ok(Json(ReportarErrorResponse { ok: true }));
    };

    let Some(ref destino) = state.config.error_report_email else {
        tracing::warn!(
            "Error reportado pero ERROR_REPORT_EMAIL no configurado: {}",
            req.mensaje
        );
        return Ok(Json(ReportarErrorResponse { ok: true }));
    };

    let stack_html = req.stack.as_deref().unwrap_or("No disponible");
    let body = format!(
        "<h2>Error reportado desde la aplicación</h2>\
         <p><strong>Usuario:</strong> {}</p>\
         <p><strong>URL:</strong> {}</p>\
         <p><strong>Navegador:</strong> {}</p>\
         <p><strong>Mensaje:</strong></p>\
         <pre style=\"background:#f5f5f5;padding:12px;border-radius:4px\">{}</pre>\
         <p><strong>Stack trace:</strong></p>\
         <pre style=\"background:#f5f5f5;padding:12px;border-radius:4px;overflow-x:auto\">{}</pre>",
        auth.user_id, req.url, req.navegador, req.mensaje, stack_html
    );

    match EmailService::enviar_generico(smtp, destino, "Error reportado en la aplicación", &body)
        .await
    {
        Ok(()) => tracing::info!("Reporte de error enviado a {destino}"),
        Err(e) => tracing::error!("No se pudo enviar reporte de error: {e}"),
    }

    Ok(Json(ReportarErrorResponse { ok: true }))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/reportar-error", post(reportar_error))
        /* Tope por IP de las escrituras de este módulo (ver rate_limit.rs). */
        .layer(axum::middleware::from_fn_with_state(
            limitador_escrituras(),
            limitar,
        ))
}
