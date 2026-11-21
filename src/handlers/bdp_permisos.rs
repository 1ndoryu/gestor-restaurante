/* [26AA-3] Permisos del modo automático por categoría.
 *
 *   GET  /api/bdp/permisos-automaticos             — las categorías y su interruptor
 *   PUT  /api/bdp/permisos-automaticos/:categoria  — enciende o apaga una categoría
 *   POST /api/bdp/permisos-automaticos/solo-lectura — apaga todas de golpe
 *   GET  /api/bdp/permisos-automaticos/actividad    — los últimos cambios
 *
 * Solo configura qué puede escribir el modo automático; no escribe nada en BDP.
 * Cada cambio queda en el historial del servicio. */

use axum::extract::{Path, Query, State};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::services::{
    actividad_reciente, apagar_todas_las_etapas, categoria_automatica_valida, fijar_permiso,
    listar_permisos, PermisoCategoria,
};
use crate::AppState;

use super::bdp_guard::exigir_modo_bdp;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/bdp/permisos-automaticos", get(listar))
        .route(
            "/bdp/permisos-automaticos/solo-lectura",
            post(volver_a_solo_lectura),
        )
        .route("/bdp/permisos-automaticos/actividad", get(actividad))
        .route("/bdp/permisos-automaticos/:categoria", put(cambiar))
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CambiarPermiso {
    pub activo: bool,
}

/// Un cambio de interruptor, con su fecha. Es lo que se muestra como registro
/// de actividad junto a los interruptores. El tipo vive en el servicio, que es
/// quien lee el historial.
/* [21AB-3c] Sin alias: utoipa nombra el esquema por el ident escrito en el
 * `body`, y un alias `as CambioPermiso` producía un `$ref` a
 * `CambioPermiso` que no existía (los dos `solo-lectura`/`actividad` no
 * estaban en el spec versionado). */
use crate::services::CambioActividad;

#[derive(Debug, Deserialize)]
pub struct LimiteActividad {
    limite: Option<i64>,
}

/// Interruptores de las categorías del modo automático (apagados si no hay fila).
#[utoipa::path(
    get,
    path = "/api/bdp/permisos-automaticos",
    tag = "BDP Backup",
    /* [18AB-4] Distinto de `bdp_lecturas::listar` (mismo tag ⇒ mismo fichero). */
    operation_id = "listar_permisos_automaticos",
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
    /* [18AB-4] Distinto de `bdp_lecturas::cambiar` para que no colisionen en el
     * cliente generado (mismo tag ⇒ mismo fichero). */
    operation_id = "cambiar_permiso_automatico",
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

/// Vuelta rápida a solo lectura: apaga todas las etapas de una vez.
#[utoipa::path(
    post,
    path = "/api/bdp/permisos-automaticos/solo-lectura",
    tag = "BDP Backup",
    operation_id = "permisos_automaticos_solo_lectura",
    responses((status = 200, body = Vec<PermisoCategoria>)),
    security(("bearer_auth" = []))
)]
pub async fn volver_a_solo_lectura(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<PermisoCategoria>>, AppError> {
    exigir_modo_bdp(&state, auth.user_id).await?;
    apagar_todas_las_etapas(&state.pool, auth.user_id)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(
        listar_permisos(&state.pool, auth.user_id)
            .await
            .map_err(AppError::Internal)?,
    ))
}

/// Actividad reciente de los interruptores: qué se encendió o apagó y cuándo.
#[utoipa::path(
    get,
    path = "/api/bdp/permisos-automaticos/actividad",
    tag = "BDP Backup",
    operation_id = "permisos_automaticos_actividad",
    responses((status = 200, body = Vec<CambioActividad>)),
    security(("bearer_auth" = []))
)]
pub async fn actividad(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(parametros): Query<LimiteActividad>,
) -> Result<Json<Vec<CambioActividad>>, AppError> {
    let cambios = actividad_reciente(&state.pool, auth.user_id, parametros.limite.unwrap_or(20))
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(cambios))
}
