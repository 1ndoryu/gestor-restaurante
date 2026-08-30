/* [198A-1/D7] Clasificaciones locales (departamento/familia) con código BDP
 * secuencial. CRUD local 100% operativo sin BDP; el alta encola el push y el
 * worker decide según el modo (en standalone no envía nada).
 *
 * GET  /api/bdp/catalogo/:tipo   — listar por tipo (departamento | familia)
 * POST /api/bdp/catalogo          — crear { tipo, nombre }
 */

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    BdpCatalogoClasificacion, CrearBdpClasificacionRequest, TIPO_DEPARTAMENTO, TIPO_FAMILIA,
};
use crate::repositories::BdpCatalogoClasificacionRepository;
use crate::services::{
    payload_crear_departamento, payload_crear_familia, verificar_permiso, AccionPermiso,
    BdpPushService,
};
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/bdp/catalogo/:tipo", get(listar_clasificaciones))
        .route("/bdp/catalogo", axum::routing::post(crear_clasificacion))
}

fn validar_tipo(tipo: &str) -> Result<(), AppError> {
    match tipo {
        TIPO_DEPARTAMENTO | TIPO_FAMILIA => Ok(()),
        _ => Err(AppError::Validation(
            "tipo debe ser 'departamento' o 'familia'".into(),
        )),
    }
}

pub async fn listar_clasificaciones(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(tipo): Path<String>,
) -> Result<Json<Vec<BdpCatalogoClasificacion>>, AppError> {
    validar_tipo(&tipo)?;
    let items =
        BdpCatalogoClasificacionRepository::listar(&state.pool, auth.user_id, &tipo).await?;
    Ok(Json(items))
}

pub async fn crear_clasificacion(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CrearBdpClasificacionRequest>,
) -> Result<Json<BdpCatalogoClasificacion>, AppError> {
    /* D7/edición de catálogo: mismo permiso que el CRUD de artículos. */
    verificar_permiso(&state.pool, AccionPermiso::CatalogoEdicion, &auth).await?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    validar_tipo(&req.tipo)?;

    let item = BdpCatalogoClasificacionRepository::crear(&state.pool, auth.user_id, &req).await?;

    /* Encolar el push BDP (el worker no envía nada en standalone). */
    let (dominio, payload) = if req.tipo == TIPO_DEPARTAMENTO {
        (
            crate::services::bdp_push::DOMINIO_DEPARTAMENTO,
            payload_crear_departamento(item.code, &item.nombre),
        )
    } else {
        (
            crate::services::bdp_push::DOMINIO_FAMILIA,
            payload_crear_familia(item.code, &item.nombre),
        )
    };
    let payload = payload.map_err(AppError::Internal)?;
    BdpPushService::encolar(
        &state.pool,
        auth.user_id,
        dominio,
        &item.id.to_string(),
        crate::services::bdp_push::OPERACION_CREAR,
        &payload,
    )
    .await
    .map_err(AppError::Internal)?;

    Ok(Json(item))
}
