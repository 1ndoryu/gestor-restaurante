/* [263A-17] Handlers de configuración del restaurante.
 * GET /api/configuracion — obtener config actual (crea defaults si no existe).
 * PATCH /api/configuracion — actualizar campos parcialmente.
 * [283A-23] GET/PUT /api/configuracion/integraciones — credentials marketing. */

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    ActualizarConfiguracionRequest, ActualizarIntegracionesRequest, ConfiguracionRestaurante,
    IntegracionMarketingPublica,
};
use crate::services::bdp_weblink::{BdpVersionResponse, BdpWeblinkClient};
use crate::services::{
    BdpBackupService, BdpSyncDryRunResponse, BdpSyncPreflightService, ConfiguracionService,
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
    if req.bdp_sync_mode.is_some() {
        return Err(AppError::Validation(
            "bdp_sync_mode solo puede cambiarse mediante /configuracion/bdp/sync-mode".into(),
        ));
    }
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    for (name, map, min) in [
        ("bdp_tender_map", req.bdp_tender_map.as_ref(), 1_i64),
        ("bdp_order_type_map", req.bdp_order_type_map.as_ref(), 0_i64),
    ] {
        if let Some(map) = map {
            let object = map
                .as_object()
                .ok_or_else(|| AppError::Validation(format!("{name} debe ser un objeto JSON")))?;
            for (key, value) in object {
                let parsed = value
                    .as_i64()
                    .or_else(|| value.as_str()?.trim().parse::<i64>().ok());
                if key.trim().is_empty() || parsed.is_none_or(|value| value < min) {
                    return Err(AppError::Validation(format!(
                        "{name} contiene una clave vacía o un ID inválido en '{key}'"
                    )));
                }
            }
        }
    }
    if req
        .bdp_poll_interval_secs
        .is_some_and(|seconds| !(10..=600).contains(&seconds))
    {
        return Err(AppError::Validation(
            "bdp_poll_interval_secs debe estar entre 10 y 600".into(),
        ));
    }
    if let Some(code) = req.bdp_default_article_code.as_deref() {
        if !code.trim().is_empty() && code.trim().parse::<i64>().ok().is_none_or(|id| id <= 0) {
            return Err(AppError::Validation(
                "bdp_default_article_code debe ser un código numérico positivo".into(),
            ));
        }
    }
    if let Some(code) = req.bdp_default_customer_code.as_deref() {
        if !code.trim().is_empty() && code.trim().parse::<i64>().ok().is_none_or(|id| id <= 0) {
            return Err(AppError::Validation(
                "bdp_default_customer_code debe ser un código numérico positivo".into(),
            ));
        }
    }
    let invalida_armado_bdp = req.bdp_base_url.is_some()
        || req.bdp_login.is_some()
        || req.bdp_password.is_some()
        || req.bdp_integrator_code.is_some()
        || req.bdp_pos_id.is_some()
        || req.bdp_employee_id.is_some()
        || req.bdp_items_profile_id.is_some()
        || req.bdp_default_article_code.is_some()
        || req.bdp_tender_map.is_some()
        || req.bdp_order_type_map.is_some()
        || req.bdp_default_customer_code.is_some()
        || req.bdp_auto_sync_customers.is_some();

    let mut config = ConfiguracionService::actualizar(&state.pool, auth.user_id, &req).await?;
    if invalida_armado_bdp {
        /* [187A-1] Cambiar cualquier dato que afecte destino o payload anula
         * permisos preparados. Aunque el UPDATE de configuración ya ocurrió,
         * la huella también impediría consumir un armado si esta transacción
         * fallara, conservando el comportamiento fail-closed. */
        let mut tx = state
            .pool
            .begin()
            .await
            .map_err(|error| AppError::Internal(format!("No se pudo desarmar BDP: {error}")))?;
        sqlx::query(
            "UPDATE configuracion_restaurante SET bdp_sync_mode = 'read_only', updated_at = NOW() WHERE user_id = $1",
        )
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| AppError::Internal(format!("No se pudo cerrar modo BDP: {error}")))?;
        sqlx::query("DELETE FROM bdp_write_arming WHERE user_id = $1")
            .bind(auth.user_id)
            .execute(&mut *tx)
            .await
            .map_err(|error| {
                AppError::Internal(format!("No se pudo borrar armado BDP: {error}"))
            })?;
        tx.commit().await.map_err(|error| {
            AppError::Internal(format!("No se pudo confirmar desarmado BDP: {error}"))
        })?;
        config.bdp_sync_mode = "read_only".to_string();
    }
    Ok(Json(config))
}

/// Request para cambiar el modo de sincronización BDP
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CambiarBdpSyncModeRequest {
    /// Nuevo modo: `read_only` o `unidirectional`.
    #[validate(length(min = 1, message = "modo es requerido"))]
    pub modo: String,
    /// Confirmación explícita de que el usuario autoriza escrituras reales en BDP.
    #[serde(default)]
    pub confirmar_escritura: bool,
    /// Debe coincidir literalmente con la URL BDP guardada al habilitar escritura.
    #[serde(default)]
    pub confirmar_destino: String,
    #[serde(default)]
    pub alcances: Vec<String>,
    #[serde(default)]
    pub duracion_minutos: i32,
    #[serde(default)]
    pub max_operaciones: i32,
    #[serde(default)]
    pub motivo: String,
    pub target_entity_type: Option<String>,
    pub target_entity_id: Option<Uuid>,
}

const VALID_BDP_WRITE_SCOPES: &[&str] =
    &["create_order", "create_customer", "add_payment", "invoice"];

/// Cambiar el modo de sincronización BDP (`read_only` / `unidirectional`)
///
/// En modo `read_only` ningún dato se envía a BDP (solo lectura).
/// En modo `unidirectional` Glory → BDP (ventas, clientes).
/// Las importaciones BDP→Glory son lecturas explícitas y no requieren un modo
/// de escritura distinto. `bidirectional` permanece bloqueado hasta definir un
/// contrato real para esa capacidad.
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
/* [187A-1] Este handler conserva junta la puerta de armado: valida destino,
 * alcance, objetivo, snapshot y huella antes de persistir una autorización. */
#[allow(clippy::too_many_lines)]
pub async fn cambiar_bdp_sync_mode(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CambiarBdpSyncModeRequest>,
) -> Result<Json<ConfiguracionRestaurante>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    if !matches!(req.modo.as_str(), "read_only" | "unidirectional") {
        return Err(AppError::Validation(
            "Modo BDP inválido; use read_only o unidirectional. bidirectional está bloqueado hasta que exista un contrato implementado y auditado.".into(),
        ));
    }

    if req.modo == "unidirectional" {
        if !req.confirmar_escritura {
            return Err(AppError::Validation(
                "Se requiere confirmación explícita para habilitar escrituras reales en BDP."
                    .into(),
            ));
        }

        let actual = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
        let target = BdpBackupService::canonical_target(&actual).map_err(AppError::Validation)?;
        if req.confirmar_destino.trim().trim_end_matches('/') != target {
            return Err(AppError::Validation(
                "La confirmación del destino no coincide exactamente con la URL BDP configurada."
                    .into(),
            ));
        }
        BdpWeblinkClient::new(&actual)
            .ensure_write_target_allowed()
            .map_err(|error| AppError::Validation(error.to_string()))?;
        if req.alcances.len() != 1
            || req
                .alcances
                .iter()
                .any(|scope| !VALID_BDP_WRITE_SCOPES.contains(&scope.as_str()))
            || !(1..=15).contains(&req.duracion_minutos)
            || req.max_operaciones != 1
            || req.motivo.trim().len() < 5
        {
            return Err(AppError::Validation(
                "El armado de pruebas requiere un único alcance, duración 1-15 minutos, exactamente una operación y un motivo explícito."
                    .into(),
            ));
        }
        let target_type = req.target_entity_type.as_deref().unwrap_or("");
        let target_id = req.target_entity_id.ok_or_else(|| {
            AppError::Validation(
                "El armado requiere el UUID exacto de la venta o cliente objetivo.".into(),
            )
        })?;
        let customer_only = req.alcances.iter().all(|scope| scope == "create_customer");
        let sale_only = req
            .alcances
            .iter()
            .all(|scope| matches!(scope.as_str(), "create_order" | "add_payment" | "invoice"));
        if !(customer_only && target_type == "cliente" || sale_only && target_type == "venta") {
            return Err(AppError::Validation(
                "El objetivo no coincide con los alcances: create_customer requiere cliente; order/payment/invoice requieren venta; no mezcle ambos tipos."
                    .into(),
            ));
        }
        if !actual.bdp_auto_backup_before_write {
            return Err(AppError::Validation(
                "No se puede habilitar escritura: bdp_auto_backup_before_write está desactivado."
                    .into(),
            ));
        }

        let fingerprint =
            BdpBackupService::connection_fingerprint(&actual).map_err(AppError::Validation)?;
        let snapshot_id: Option<Uuid> = sqlx::query_scalar(
            r"SELECT id
                FROM bdp_snapshots
                WHERE user_id = $1
                  AND tipo = 'completo'
                  AND direccion = 'bdp'
                  AND target_base_url = $2
                  AND connection_fingerprint = $3
                  AND (expires_at IS NULL OR expires_at > NOW())
                  AND created_at >= NOW() - INTERVAL '24 hours'
                  AND datos->'articulos' IS NOT NULL AND datos->'articulos' <> 'null'::jsonb
                  AND datos->'clientes' IS NOT NULL AND datos->'clientes' <> 'null'::jsonb
                  AND datos->'departamentos' IS NOT NULL AND datos->'departamentos' <> 'null'::jsonb
                  AND datos->'salones' IS NOT NULL AND datos->'salones' <> 'null'::jsonb
                  AND datos->'empleados' IS NOT NULL AND datos->'empleados' <> 'null'::jsonb
                ORDER BY created_at DESC
                LIMIT 1",
        )
        .bind(auth.user_id)
        .bind(&target)
        .bind(&fingerprint)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Error verificando snapshot BDP: {e}")))?;

        let snapshot_id = snapshot_id.ok_or_else(|| {
            AppError::Validation(
                "No se puede habilitar escritura: falta un snapshot completo de esta conexión BDP exacta, vigente y sin lecturas fallidas."
                    .into(),
            )
        })?;

        sqlx::query(
            r"INSERT INTO bdp_write_arming
               (user_id, base_url, scopes, target_entity_type, target_entity_id,
                reason, expires_at, remaining_operations, snapshot_id, connection_fingerprint)
               VALUES ($1, $2, $3, $4, $5, $6, NOW() + ($7 * INTERVAL '1 minute'), $8, $9, $10)
               ON CONFLICT (user_id) DO UPDATE SET
                 base_url = EXCLUDED.base_url,
                 scopes = EXCLUDED.scopes,
                 target_entity_type = EXCLUDED.target_entity_type,
                 target_entity_id = EXCLUDED.target_entity_id,
                 reason = EXCLUDED.reason,
                 expires_at = EXCLUDED.expires_at,
                 remaining_operations = EXCLUDED.remaining_operations,
                 snapshot_id = EXCLUDED.snapshot_id,
                 connection_fingerprint = EXCLUDED.connection_fingerprint,
                 created_at = NOW()",
        )
        .bind(auth.user_id)
        .bind(&target)
        .bind(&req.alcances)
        .bind(target_type)
        .bind(target_id)
        .bind(req.motivo.trim())
        .bind(req.duracion_minutos)
        .bind(req.max_operaciones)
        .bind(snapshot_id)
        .bind(&fingerprint)
        .execute(&state.pool)
        .await
        .map_err(|error| AppError::Internal(format!("No se pudo crear armado BDP: {error}")))?;
    } else {
        sqlx::query("DELETE FROM bdp_write_arming WHERE user_id = $1")
            .bind(auth.user_id)
            .execute(&state.pool)
            .await
            .map_err(|error| AppError::Internal(format!("No se pudo desarmar BDP: {error}")))?;
    }

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
        .route(
            "/configuracion/bdp/sync-mode",
            axum::routing::put(cambiar_bdp_sync_mode),
        )
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

/// Validar el contrato BDP contra el simulador local. `OnlyCheck` permanece
/// bloqueado para destinos externos salvo allowlist extraordinaria separada.
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
