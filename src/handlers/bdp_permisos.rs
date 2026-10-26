/* [26AA-3] Permisos del modo automático por categoría.
 *
 *   GET /api/bdp/permisos-automaticos             — las siete categorías y su interruptor
 *   PUT /api/bdp/permisos-automaticos/:categoria  — enciende o apaga una categoría
 *
 * Solo configura qué puede escribir el modo automático; no escribe nada en BDP.
 * Cada cambio queda en el historial del servicio. */

use axum::extract::{Path, State};
use axum::routing::{get, put};
use axum::{Json, Router};
use serde::Deserialize;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::services::{
    categoria_automatica_valida, fijar_permiso, listar_permisos, PermisoCategoria,
};
use crate::AppState;

use super::bdp_guard::exigir_modo_bdp;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/bdp/permisos-automaticos", get(listar))
        .route("/bdp/permisos-automaticos/:categoria", put(cambiar))
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CambiarPermiso {
    pub activo: bool,
}

/// Interruptores de las categorías del modo automático (apagados si no hay fila).
#[utoipa::path(
    get,
    path = "/api/bdp/permisos-automaticos",
    tag = "BDP Backup",
    responses((status = 200, body = Vec<PermisoCategoria>)),
    security(("bearer_auth" = []))
)]
pub async fn listar(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<PermisoCategoria>>, AppError> {
    let permisos = listar_permisos(&state.pool, auth.user_id)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(permisos))
}

/// Enciende o apaga una categoría del modo automático.
#[utoipa::path(
    put,
    path = "/api/bdp/permisos-automaticos/{categoria}",
    tag = "BDP Backup",
    request_body = CambiarPermiso,
    responses((status = 200, body = Vec<PermisoCategoria>), (status = 400, description = "Categoría desconocida")),
    security(("bearer_auth" = []))
)]
pub async fn cambiar(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(categoria): Path<String>,
    Json(req): Json<CambiarPermiso>,
) -> Result<Json<Vec<PermisoCategoria>>, AppError> {
    exigir_modo_bdp(&state, auth.user_id).await?;
    /* Una categoría inventada no debe llegar a la tabla: el CHECK la rechazaría
     * con un 500 poco claro. */
    if !categoria_automatica_valida(&categoria) {
        return Err(AppError::Validation("Categoría desconocida.".into()));
    }
    fijar_permiso(&state.pool, auth.user_id, &categoria, req.activo)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(
        listar_permisos(&state.pool, auth.user_id)
            .await
            .map_err(AppError::Internal)?,
    ))
}
