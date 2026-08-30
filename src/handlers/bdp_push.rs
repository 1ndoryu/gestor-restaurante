/* [198A-1/F1] Flush manual de la cola de push Glory -> BDP (D1/D2).
 *
 * POST /api/bdp/push/flush — dispara `BdpPushFlushService::flush` con
 * `forzar_manual=true`. Es el botón "Sincronizar a BDP":
 *   - D1: el botón manual existe siempre (aunque `push_modalidad=automatico`).
 *   - D2: el reintento tras bloqueo por suscripción es SOLO manual; este
 *     endpoint procesa también las filas `pendiente_suscripcion`.
 *
 * [208A-2/C4] Visibilidad de la cola (decisión D5):
 *   GET /api/bdp/push/pendientes  — listar filas (estado, reintentos, error).
 *   POST /api/bdp/push/:id/reintentar — reintento individual de una fila.
 *
 * En standalone el worker no envía nada (no-op) y devuelve el resumen con
 * `omitidos_standalone`; el fail-closed se conserva porque cada fila pasa por
 * `armar_push` (arming) → backup → auditoría antes de cualquier HTTP. */

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::UserRole;
use crate::services::{BdpPushFlushResumen, BdpPushFlushService, BdpPushFila, BdpPushService};
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/bdp/push/flush", axum::routing::post(flush_manual))
        .route(
            "/bdp/push/pendientes",
            get(listar_pendientes),
        )
        .route(
            "/bdp/push/:id/reintentar",
            axum::routing::post(reintentar_fila),
        )
}

pub async fn flush_manual(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<BdpPushFlushResumen>, AppError> {
    /* Escritura BDP: disparar el push (armado + envío) es acción de Admin. */
    auth.require_role(&[UserRole::Admin])?;
    let resumen = BdpPushFlushService::flush(&state.pool, auth.user_id, true)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(resumen))
}

/// Listar las filas de la cola de sincronización (visibilidad D5).
pub async fn listar_pendientes(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<BdpPushFila>>, AppError> {
    /* Acción de sincronización BDP: solo Admin (misma política que flush). */
    auth.require_role(&[UserRole::Admin])?;
    let filas = BdpPushService::listar_filas(&state.pool, auth.user_id, 100)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(filas))
}

/// Reintentar individualmente una fila de la cola (decisión D5, regla D2).
pub async fn reintentar_fila(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<BdpPushFlushResumen>, AppError> {
    auth.require_role(&[UserRole::Admin])?;
    let fila = BdpPushService::obtener_fila(&state.pool, auth.user_id, id)
        .await
        .map_err(AppError::Internal)?;
    let Some(fila) = fila else {
        return Err(AppError::NotFound(
            "Fila de sincronización no encontrada".into(),
        ));
    };
    if matches!(fila.estado.as_str(), "sincronizado" | "descartado") {
        return Err(AppError::Validation(
            "La fila ya está sincronizada o descartada".into(),
        ));
    }
    let resumen = BdpPushFlushService::reintentar_uno(&state.pool, auth.user_id, id)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(resumen))
}
