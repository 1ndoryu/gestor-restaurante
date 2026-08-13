/* [BKP-004] Handlers para exploración, backup y auditoría BDP.
 * Endpoints de solo lectura para inventariar BDP y gestionar snapshots.
 * Ningún endpoint de este módulo modifica datos en BDP (excepto restaurar Glory). */

use axum::{extract::Path, extract::State, routing::get, Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::services::ConfiguracionService;
use crate::services::{BdpAuditEntry, BdpBackupService, BdpSnapshot, RestoreResult};
use crate::services::{BdpExploracionResultado, BdpExplorerService};

pub fn routes() -> Router<crate::handlers::AppState> {
    Router::new()
        .route("/bdp/explorar", get(explorar_bdp))
        .route(
            "/bdp/backup/completo",
            axum::routing::post(snapshot_completo),
        )
        .route("/bdp/backup/parcial", axum::routing::post(snapshot_parcial))
        .route("/bdp/backup/glory", axum::routing::post(snapshot_glory))
        .route("/bdp/backup/snapshots", get(listar_snapshots))
        .route(
            "/bdp/backup/snapshots/:id",
            get(obtener_snapshot).delete(eliminar_snapshot),
        )
        .route(
            "/bdp/backup/restaurar/:id",
            axum::routing::post(restaurar_glory),
        )
        .route("/bdp/audit", get(listar_audit))
}

/// Explora BDP de forma segura (solo lectura).
/// Devuelve un inventario de lo que hay en BDP sin modificar nada.
#[utoipa::path(
    get,
    path = "/api/bdp/explorar",
    tag = "BDP Backup",
    responses(
        (status = 200, description = "Exploración completada", body = BdpExploracionResultado),
        (status = 400, description = "BDP no configurado"),
        (status = 500, description = "Error interno"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn explorar_bdp(
    State(state): State<crate::handlers::AppState>,
    auth: AuthUser,
) -> Result<Json<BdpExploracionResultado>, AppError> {
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation(
            "BDP no está configurado. Configura URL y credenciales primero.".into(),
        ));
    }

    let resultado = BdpExplorerService::explorar_bdp_completo(&config).await;
    Ok(Json(resultado))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub(crate) struct SnapshotParcialRequest {
    tipos: Vec<String>,
    notas: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub(crate) struct SnapshotGloryRequest {
    tipos: Vec<String>,
    notas: Option<String>,
}

/// Snapshot completo de BDP.
/// Lee todos los endpoints de lectura y guarda el resultado.
/// ⚠️ Requiere autorización — hace llamadas a la API de BDP.
#[utoipa::path(
    post,
    path = "/api/bdp/backup/completo",
    tag = "BDP Backup",
    request_body = Option<String>,
    responses(
        (status = 200, description = "Snapshot creado", body = BdpSnapshot),
        (status = 400, description = "BDP no configurado"),
        (status = 500, description = "Error interno"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn snapshot_completo(
    State(state): State<crate::handlers::AppState>,
    auth: AuthUser,
    Json(notas): Json<Option<String>>,
) -> Result<Json<BdpSnapshot>, AppError> {
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation("BDP no está configurado.".into()));
    }

    let snapshot =
        BdpBackupService::snapshot_bdp_completo(&state.pool, auth.user_id, &config, notas)
            .await
            .map_err(AppError::Internal)?;

    Ok(Json(snapshot))
}

/// Snapshot parcial de BDP (solo los tipos seleccionados).
/// Tipos válidos: articulos, clientes, departamentos, salones, empleados
#[utoipa::path(
    post,
    path = "/api/bdp/backup/parcial",
    tag = "BDP Backup",
    request_body = SnapshotParcialRequest,
    responses(
        (status = 200, description = "Snapshot creado", body = BdpSnapshot),
        (status = 400, description = "BDP no configurado o tipos inválidos"),
        (status = 500, description = "Error interno"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn snapshot_parcial(
    State(state): State<crate::handlers::AppState>,
    auth: AuthUser,
    Json(req): Json<SnapshotParcialRequest>,
) -> Result<Json<BdpSnapshot>, AppError> {
    if req.tipos.is_empty() {
        return Err(AppError::Validation(
            "Debes seleccionar al menos un tipo de dato.".into(),
        ));
    }

    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation("BDP no está configurado.".into()));
    }

    let snapshot = BdpBackupService::snapshot_bdp_parcial(
        &state.pool,
        auth.user_id,
        &config,
        &req.tipos,
        req.notas,
    )
    .await
    .map_err(AppError::Internal)?;

    Ok(Json(snapshot))
}

/// Snapshot de datos locales de Glory (0 llamadas a BDP).
/// Tipos válidos: ventas, clientes, mapeos
#[utoipa::path(
    post,
    path = "/api/bdp/backup/glory",
    tag = "BDP Backup",
    request_body = SnapshotGloryRequest,
    responses(
        (status = 200, description = "Snapshot creado", body = BdpSnapshot),
        (status = 500, description = "Error interno"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn snapshot_glory(
    State(state): State<crate::handlers::AppState>,
    auth: AuthUser,
    Json(req): Json<SnapshotGloryRequest>,
) -> Result<Json<BdpSnapshot>, AppError> {
    if req.tipos.is_empty() {
        return Err(AppError::Validation(
            "Debes seleccionar al menos un tipo de dato.".into(),
        ));
    }

    let snapshot =
        BdpBackupService::snapshot_glory(&state.pool, auth.user_id, &req.tipos, req.notas)
            .await
            .map_err(AppError::Internal)?;

    Ok(Json(snapshot))
}

/// Lista snapshots del usuario (historial).
#[utoipa::path(
    get,
    path = "/api/bdp/backup/snapshots",
    tag = "BDP Backup",
    params(
        ("limit" = Option<i64>, Query, description = "Máximo de snapshots a devolver (default 50)")
    ),
    responses(
        (status = 200, description = "Lista de snapshots", body = Vec<BdpSnapshot>),
        (status = 500, description = "Error interno"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_snapshots(
    State(state): State<crate::handlers::AppState>,
    auth: AuthUser,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<BdpSnapshot>>, AppError> {
    let limit = params
        .get("limit")
        .and_then(|l| l.parse::<i64>().ok())
        .unwrap_or(50);

    let snapshots = BdpBackupService::listar_snapshots(&state.pool, auth.user_id, limit)
        .await
        .map_err(AppError::Internal)?;

    Ok(Json(snapshots))
}

/// Obtiene un snapshot por ID.
#[utoipa::path(
    get,
    path = "/api/bdp/backup/snapshots/{id}",
    tag = "BDP Backup",
    params(("id" = Uuid, Path, description = "ID del snapshot")),
    responses(
        (status = 200, description = "Snapshot encontrado", body = BdpSnapshot),
        (status = 404, description = "Snapshot no encontrado"),
        (status = 500, description = "Error interno"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn obtener_snapshot(
    State(state): State<crate::handlers::AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<BdpSnapshot>, AppError> {
    let snapshot = BdpBackupService::obtener_snapshot(&state.pool, id)
        .await
        .map_err(AppError::Internal)?
        .ok_or_else(|| AppError::NotFound("Snapshot no encontrado".into()))?;

    if snapshot.user_id != auth.user_id {
        return Err(AppError::NotFound("Snapshot no encontrado".into()));
    }

    Ok(Json(snapshot))
}

/// Elimina un snapshot.
#[utoipa::path(
    delete,
    path = "/api/bdp/backup/snapshots/{id}",
    tag = "BDP Backup",
    params(("id" = Uuid, Path, description = "ID del snapshot")),
    responses(
        (status = 200, description = "Snapshot eliminado"),
        (status = 404, description = "Snapshot no encontrado"),
        (status = 500, description = "Error interno"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn eliminar_snapshot(
    State(state): State<crate::handlers::AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let eliminado = BdpBackupService::eliminar_snapshot(&state.pool, id, auth.user_id)
        .await
        .map_err(AppError::Internal)?;

    if !eliminado {
        return Err(AppError::NotFound("Snapshot no encontrado".into()));
    }

    Ok(Json(serde_json::json!({ "eliminado": true })))
}

/// Restaura datos locales de Glory desde un snapshot.
/// ⚠️ Solo restaura datos locales — BDP no permite delete/update via API.
#[utoipa::path(
    post,
    path = "/api/bdp/backup/restaurar/{id}",
    tag = "BDP Backup",
    params(("id" = Uuid, Path, description = "ID del snapshot a restaurar")),
    request_body = RestoreGloryRequest,
    responses(
        (status = 200, description = "Restauración completada", body = RestoreResult),
        (status = 400, description = "Confirmación inválida o snapshot no es de tipo Glory"),
        (status = 404, description = "Snapshot no encontrado"),
        (status = 500, description = "Error interno"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn restaurar_glory(
    State(state): State<crate::handlers::AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RestoreGloryRequest>,
) -> Result<Json<RestoreResult>, AppError> {
    /* [AUDIT-11.3] Confirmación textual explícita antes de restaurar. */
    let expected = format!("RESTAURAR {id}");
    if req.confirmacion.trim() != expected {
        return Err(AppError::Validation(format!(
            "Confirmación inválida. Escriba exactamente: {expected}"
        )));
    }
    let result = BdpBackupService::restaurar_glory(&state.pool, id, auth.user_id)
        .await
        .map_err(|e| {
            if e.contains("no encontrado") || e.contains("No autorizado") {
                AppError::NotFound(e)
            } else {
                AppError::Validation(e)
            }
        })?;

    Ok(Json(result))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub(crate) struct RestoreGloryRequest {
    /// Debe ser exactamente "RESTAURAR {uuid}".
    pub confirmacion: String,
}

/// Lista entradas del audit log.
#[utoipa::path(
    get,
    path = "/api/bdp/audit",
    tag = "BDP Backup",
    params(
        ("limit" = Option<i64>, Query, description = "Máximo de entradas a devolver (default 100)")
    ),
    responses(
        (status = 200, description = "Audit log", body = Vec<BdpAuditEntry>),
        (status = 500, description = "Error interno"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_audit(
    State(state): State<crate::handlers::AppState>,
    auth: AuthUser,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<BdpAuditEntry>>, AppError> {
    let limit = params
        .get("limit")
        .and_then(|l| l.parse::<i64>().ok())
        .unwrap_or(100);

    let entries = BdpBackupService::listar_audit(&state.pool, auth.user_id, limit)
        .await
        .map_err(AppError::Internal)?;

    Ok(Json(entries))
}
