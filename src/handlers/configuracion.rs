/* [263A-17] Handlers de configuración del restaurante.
 * GET /api/configuracion — obtener config actual (crea defaults si no existe).
 * PATCH /api/configuracion — actualizar campos parcialmente.
 * [283A-23] GET/PUT /api/configuracion/integraciones — credentials marketing. */

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    ActualizarConfiguracionRequest, ActualizarIntegracionesRequest, ConfiguracionRestaurante,
    IntegracionMarketingPublica,
};
use crate::services::bdp_weblink::{BdpVersionResponse, BdpWeblinkClient};
use crate::services::{
    BdpSyncDryRunResponse, BdpSyncPreflightService, ConfiguracionService,
    IntegracionMarketingService,
};
use crate::AppState;

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Serialize, ToSchema)]
pub struct BdpDiagnosticoResponse {
    pub configurado: bool,
    pub sync_habilitado: bool,
    pub health_ok: bool,
    pub login_ok: bool,
    pub session_expires_in_seconds: Option<i64>,
    pub version: Option<i32>,
    pub sub_version: Option<i32>,
    pub application: Option<String>,
    pub application_description: Option<String>,
    pub mensaje: String,
}

impl BdpDiagnosticoResponse {
    fn sin_configurar(sync_habilitado: bool) -> Self {
        Self::base(false, sync_habilitado, "BDP no esta configurado")
    }

    fn health_error(sync_habilitado: bool, mensaje: impl Into<String>) -> Self {
        Self::base(true, sync_habilitado, mensaje)
    }

    fn login_error(sync_habilitado: bool, mensaje: impl Into<String>) -> Self {
        Self {
            health_ok: true,
            ..Self::base(true, sync_habilitado, mensaje)
        }
    }

    fn version_error(
        sync_habilitado: bool,
        expires_in_seconds: i64,
        mensaje: impl Into<String>,
    ) -> Self {
        Self {
            health_ok: true,
            login_ok: true,
            session_expires_in_seconds: Some(expires_in_seconds),
            ..Self::base(true, sync_habilitado, mensaje)
        }
    }

    fn version_ok(
        sync_habilitado: bool,
        expires_in_seconds: i64,
        version: BdpVersionResponse,
    ) -> Self {
        Self {
            health_ok: true,
            login_ok: true,
            session_expires_in_seconds: Some(expires_in_seconds),
            version: Some(version.version),
            sub_version: Some(version.sub_version),
            application: Some(version.application),
            application_description: Some(version.application_description),
            mensaje: "BDP WebLink REST API conectado correctamente".to_string(),
            ..Self::base(true, sync_habilitado, "")
        }
    }

    fn base(configurado: bool, sync_habilitado: bool, mensaje: impl Into<String>) -> Self {
        Self {
            configurado,
            sync_habilitado,
            health_ok: false,
            login_ok: false,
            session_expires_in_seconds: None,
            version: None,
            sub_version: None,
            application: None,
            application_description: None,
            mensaje: mensaje.into(),
        }
    }
}

/// Obtener la configuración del restaurante (crea defaults si es primera vez)
#[utoipa::path(
    get,
    path = "/api/configuracion",
    tag = "Configuracion",
    responses(
        (status = 200, description = "Configuración actual", body = ConfiguracionRestaurante),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn obtener_configuracion(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ConfiguracionRestaurante>, AppError> {
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
    Ok(Json(config))
}

/// Actualizar la configuración del restaurante (parcial)
#[utoipa::path(
    patch,
    path = "/api/configuracion",
    tag = "Configuracion",
    request_body = ActualizarConfiguracionRequest,
    responses(
        (status = 200, description = "Configuración actualizada", body = ConfiguracionRestaurante),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 422, description = "Error de validación", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn actualizar_configuracion(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<ActualizarConfiguracionRequest>,
) -> Result<Json<ConfiguracionRestaurante>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let config = ConfiguracionService::actualizar(&state.pool, auth.user_id, &req).await?;
    Ok(Json(config))
}

/// Request para cambiar el modo de sincronización BDP
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CambiarBdpSyncModeRequest {
    /// Nuevo modo: "read_only", "unidirectional" o "bidirectional"
    #[validate(length(min = 1, message = "modo es requerido"))]
    pub modo: String,
}

/// Cambiar el modo de sincronización BDP (read_only / unidirectional / bidirectional)
///
/// En modo `read_only` ningún dato se envía a BDP (solo lectura).
/// En modo `unidirectional` Glory → BDP (ventas, clientes).
/// En modo `bidirectional` BDP ↔ Glory (reservas, mesas, etc).
#[utoipa::path(
    put,
    path = "/api/configuracion/bdp/sync-mode",
    tag = "Configuracion",
    request_body = CambiarBdpSyncModeRequest,
    responses(
        (status = 200, description = "Modo actualizado", body = ConfiguracionRestaurante),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 422, description = "Modo inválido", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn cambiar_bdp_sync_mode(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CambiarBdpSyncModeRequest>,
) -> Result<Json<ConfiguracionRestaurante>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let update = ActualizarConfiguracionRequest {
        bdp_sync_mode: Some(req.modo),
        ..Default::default()
    };
    let config = ConfiguracionService::actualizar(&state.pool, auth.user_id, &update).await?;
    Ok(Json(config))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/configuracion",
            get(obtener_configuracion).patch(actualizar_configuracion),
        )
        .route(
            "/configuracion/integraciones",
            get(obtener_integraciones).put(actualizar_integraciones),
        )
        .route("/configuracion/bdp/diagnostico", get(diagnosticar_bdp))
        .route(
            "/configuracion/bdp/sync-dry-run",
            get(diagnosticar_bdp_sync_dry_run),
        )
        .route("/configuracion/bdp/sync-mode", axum::routing::put(cambiar_bdp_sync_mode))
}

/// Diagnosticar conexión BDP/WebLink sin exponer credenciales
#[utoipa::path(
    get,
    path = "/api/configuracion/bdp/diagnostico",
    tag = "Configuracion",
    responses(
        (status = 200, description = "Diagnóstico BDP", body = BdpDiagnosticoResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn diagnosticar_bdp(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<BdpDiagnosticoResponse>, AppError> {
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
    let configurado = bdp_configurado(&config);

    if !configurado {
        return Ok(Json(BdpDiagnosticoResponse::sin_configurar(
            config.bdp_sync_enabled,
        )));
    }

    let client = BdpWeblinkClient::new(&config);
    match client.health().await {
        Ok(health) if health.is_alive => {}
        Ok(_) => {
            return Ok(Json(BdpDiagnosticoResponse::health_error(
                config.bdp_sync_enabled,
                "BDP respondio Health pero IsAlive=false",
            )));
        }
        Err(error) => {
            return Ok(Json(BdpDiagnosticoResponse::health_error(
                config.bdp_sync_enabled,
                format!("No se pudo contactar BDP Health: {error}"),
            )));
        }
    }

    let session = match client.login().await {
        Ok(session) => session,
        Err(error) => {
            return Ok(Json(BdpDiagnosticoResponse::login_error(
                config.bdp_sync_enabled,
                format!("BDP Health OK, Login fallo: {error}"),
            )));
        }
    };

    let version: Result<BdpVersionResponse, _> = client
        .post_authenticated(
            "/Service/GetVersion",
            &serde_json::json!({}),
            &session.token,
        )
        .await;

    match version {
        Ok(version) if version.error_message.trim().is_empty() => {
            Ok(Json(BdpDiagnosticoResponse::version_ok(
                config.bdp_sync_enabled,
                session.expires_in_seconds,
                version,
            )))
        }
        Ok(version) => Ok(Json(BdpDiagnosticoResponse {
            ..BdpDiagnosticoResponse::version_error(
                config.bdp_sync_enabled,
                session.expires_in_seconds,
                format!(
                    "Login OK, GetVersion devolvio error: {}",
                    version.error_message
                ),
            )
        })),
        Err(error) => Ok(Json(BdpDiagnosticoResponse::version_error(
            config.bdp_sync_enabled,
            session.expires_in_seconds,
            format!("Login OK, GetVersion fallo: {error}"),
        ))),
    }
}

fn bdp_configurado(config: &ConfiguracionRestaurante) -> bool {
    !config.bdp_base_url.trim().is_empty()
        && !config.bdp_login.trim().is_empty()
        && !config.bdp_password.trim().is_empty()
        && !config.bdp_integrator_code.trim().is_empty()
}

/// Probar sincronización BDP sin crear datos reales
#[utoipa::path(
    get,
    path = "/api/configuracion/bdp/sync-dry-run",
    tag = "Configuracion",
    responses(
        (status = 200, description = "Dry-run de sincronización BDP", body = BdpSyncDryRunResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn diagnosticar_bdp_sync_dry_run(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<BdpSyncDryRunResponse>, AppError> {
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
    Ok(Json(
        BdpSyncPreflightService::execute(&state.pool, auth.user_id, &config).await,
    ))
}

/* ========== Integraciones de marketing ========== */

/// Obtener estado de integraciones (sin exponer credentials)
#[utoipa::path(
    get,
    path = "/api/configuracion/integraciones",
    tag = "Configuracion",
    responses(
        (status = 200, description = "Estado de integraciones", body = IntegracionMarketingPublica),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn obtener_integraciones(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<IntegracionMarketingPublica>, AppError> {
    let integ = IntegracionMarketingService::obtener_publica(&state.pool, auth.user_id).await?;
    Ok(Json(integ))
}

/// Actualizar credentials de integraciones de marketing
#[utoipa::path(
    put,
    path = "/api/configuracion/integraciones",
    tag = "Configuracion",
    request_body = ActualizarIntegracionesRequest,
    responses(
        (status = 200, description = "Integraciones actualizadas", body = IntegracionMarketingPublica),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 422, description = "Error de validación", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn actualizar_integraciones(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<ActualizarIntegracionesRequest>,
) -> Result<Json<IntegracionMarketingPublica>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let integ = IntegracionMarketingService::actualizar(&state.pool, auth.user_id, &req).await?;
    Ok(Json(integ))
}
