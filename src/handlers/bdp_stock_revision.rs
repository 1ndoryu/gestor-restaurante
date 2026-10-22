/* [D26g] Revisión manual del libro de stock.
 *
 *   GET  /api/bdp/stock/revisiones        — filas que esperan dictamen humano
 *   POST /api/bdp/stock/revisiones/:id    — registra el dictamen y destraba
 *
 * Por qué existe: cuando la relectura de `GetStock` no puede atribuir el
 * movimiento, la auditoría se queda `ambiguo` y `comprobar_sin_pendiente`
 * bloquea ese artículo para siempre. Antes no había endpoint ni pantalla para
 * salir de ahí; estas dos rutas son esa salida, y ambas exigen Admin porque
 * deciden si un movimiento de stock consta en el BDP o no. */

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::UserRole;
use crate::services::{
    descartar_auditoria_stock, listar_auditorias_stock_bloqueadas, listar_revisiones_stock,
    resolver_revision_stock, AuditoriaStockBloqueada, DecisionRevisionStock, FilaRevisionStock,
    ResolucionStock,
};
use crate::AppState;

use super::rate_limit::{limitador_escrituras, limitar};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/bdp/stock/revisiones", get(listar))
        .route("/bdp/stock/auditorias-bloqueadas", get(listar_bloqueadas))
        // sentinel-disable-next-line ruta-post-sin-rate-limit: el limitador está en el `layer` de abajo, igual que en el resto de escrituras BDP.
        .route("/bdp/stock/revisiones/:id", post(resolver))
        // sentinel-disable-next-line ruta-post-sin-rate-limit: mismo `layer` que resuelve y escribe en auditoría.
        .route("/bdp/stock/auditorias-bloqueadas/:id", post(descartar))
        .layer(axum::middleware::from_fn_with_state(
            limitador_escrituras(),
            limitar,
        ))
}

#[derive(Debug, Deserialize)]
pub struct ResolverRevisionRequest {
    /// `aplicado` (BDP ya lo tiene) o `no_aplicado` (vuelve a la cola).
    pub decision: String,
    /// Por qué se decidió así: queda en la nota del libro y en la auditoría.
    pub motivo: String,
}

#[derive(Debug, Serialize)]
pub struct RespuestaRevision {
    pub ok: bool,
    pub resolucion: ResolucionStock,
}

/// Filas del libro en `requiere_revision`: lo que el restaurante tiene que
/// mirar en el BDP para decidir.
pub async fn listar(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<FilaRevisionStock>>, AppError> {
    auth.require_role(&[UserRole::Admin])?;
    let filas = listar_revisiones_stock(&state.pool, auth.user_id)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(filas))
}

/// Registra el dictamen: cierra la auditoría (destraba el guard), cierra la
/// fila del libro y deja la cola en `sincronizado` o `pendiente`.
pub async fn resolver(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id_local): Path<String>,
    Json(cuerpo): Json<ResolverRevisionRequest>,
) -> Result<Json<RespuestaRevision>, AppError> {
    auth.require_role(&[UserRole::Admin])?;
    let decision =
        DecisionRevisionStock::desde_texto(&cuerpo.decision).map_err(AppError::BadRequest)?;
    let resolucion = resolver_revision_stock(
        &state.pool,
        auth.user_id,
        &id_local,
        decision,
        &cuerpo.motivo,
    )
    .await
    .map_err(AppError::BadRequest)?;
    Ok(Json(RespuestaRevision {
        ok: true,
        resolucion,
    }))
}

#[derive(Debug, Deserialize)]
pub struct DescartarAuditoriaRequest {
    /// Por qué se da por no aplicada: queda en la auditoría para siempre.
    pub motivo: String,
}

#[derive(Debug, Serialize)]
pub struct RespuestaDescarte {
    pub ok: bool,
}

/* [D-auto-push] Auditorías de stock que ninguna revisión automática puede
 * cerrar (el envío falló antes de abrir el libro). Sin esta salida, ese
 * artículo y almacén no admitían otro movimiento nunca más. */
pub async fn listar_bloqueadas(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<AuditoriaStockBloqueada>>, AppError> {
    auth.require_role(&[UserRole::Admin])?;
    let filas = listar_auditorias_stock_bloqueadas(&state.pool, auth.user_id)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(filas))
}

pub async fn descartar(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<uuid::Uuid>,
    Json(cuerpo): Json<DescartarAuditoriaRequest>,
) -> Result<Json<RespuestaDescarte>, AppError> {
    auth.require_role(&[UserRole::Admin])?;
    descartar_auditoria_stock(&state.pool, auth.user_id, id, &cuerpo.motivo)
        .await
        .map_err(AppError::BadRequest)?;
    Ok(Json(RespuestaDescarte { ok: true }))
}
