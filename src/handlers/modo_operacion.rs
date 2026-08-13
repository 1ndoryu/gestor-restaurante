/* [128A-1/F1] Handler del conmutador de modo operativo BDP.
 * GET /api/configuracion/modo — modo efectivo derivado sin tocar BDP.
 * PATCH /api/configuracion/modo — cambiar el switch maestro (persistido). */

use axum::extract::State;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::ActualizarConfiguracionRequest;
use crate::services::{ServicioModoOperacion, MODO_AUTO, MODO_BDP, MODO_STANDALONE};
use crate::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct ModoOperacionResponse {
    pub modo_operacion: String,
    pub modo_efectivo: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CambiarModoOperacionRequest {
    /// 'auto' | 'standalone' | 'bdp'
    #[validate(length(min = 1, message = "modo es requerido"))]
    pub modo: String,
}

/// Obtener el modo operativo configurado y el modo efectivo derivado
#[utoipa::path(
    get,
    path = "/api/configuracion/modo",
    tag = "Configuracion",
    responses(
        (status = 200, description = "Modo operativo actual", body = ModoOperacionResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn obtener_modo_operacion(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ModoOperacionResponse>, AppError> {
    let config = crate::services::ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
    let modo_efectivo = ServicioModoOperacion::modo_efectivo_desde_config(&config).as_str();
    Ok(Json(ModoOperacionResponse {
        modo_operacion: config.modo_operacion,
        modo_efectivo: modo_efectivo.to_string(),
    }))
}

/// Cambiar el modo operativo (switch maestro M1); invalida la cache M3
#[utoipa::path(
    patch,
    path = "/api/configuracion/modo",
    tag = "Configuracion",
    request_body = CambiarModoOperacionRequest,
    responses(
        (status = 200, description = "Modo operativo actualizado", body = ModoOperacionResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 422, description = "Modo inválido", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn cambiar_modo_operacion(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CambiarModoOperacionRequest>,
) -> Result<Json<ModoOperacionResponse>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    if !matches!(req.modo.as_str(), MODO_AUTO | MODO_STANDALONE | MODO_BDP) {
        return Err(AppError::Validation(format!(
            "modo_operacion inválido: '{}'. Valores permitidos: {MODO_AUTO}, {MODO_STANDALONE}, {MODO_BDP}",
            req.modo
        )));
    }

    let update = ActualizarConfiguracionRequest {
        modo_operacion: Some(req.modo),
        ..Default::default()
    };
    let config =
        crate::services::ConfiguracionService::actualizar(&state.pool, auth.user_id, &update)
            .await?;
    state.modo_operacion.invalidar(auth.user_id);
    let modo_efectivo = ServicioModoOperacion::modo_efectivo_desde_config(&config).as_str();
    Ok(Json(ModoOperacionResponse {
        modo_operacion: config.modo_operacion,
        modo_efectivo: modo_efectivo.to_string(),
    }))
}

pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/configuracion/modo",
        axum::routing::get(obtener_modo_operacion).patch(cambiar_modo_operacion),
    )
}
