/* [D32] Operaciones que BDP no expone por API, con su procedimiento y responsable.
 * La web solo muestra la lista y guarda la asignación; no ejecuta nada en BDP. */

use axum::extract::Path;

use super::{AppError, AppState, AuthUser, ConfiguracionService, Json, State};
use crate::models::UserRole;
use crate::repositories::BdpOperacionesNoApiRepository;
use crate::services::bdp_operaciones_no_api::{self as operaciones_no_api, OperacionNoApi};

/// Cuerpo de PUT /bdp/operaciones-no-api/:codigo. Textos vacíos = usar el valor por defecto.
#[derive(serde::Deserialize)]
pub struct ActualizarOperacionNoApi {
    #[serde(default)]
    pub responsable: String,
    #[serde(default)]
    pub procedimiento: String,
}

pub async fn listar_operaciones_no_api(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<OperacionNoApi>>, AppError> {
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id)
        .await
        .map_err(|e| AppError::Internal(format!("Error obteniendo configuración: {e}")))?;
    let asignaciones = BdpOperacionesNoApiRepository::listar(&state.pool, auth.user_id)
        .await
        .map_err(|e| AppError::Internal(format!("Error leyendo operaciones no API: {e}")))?;
    Ok(Json(operaciones_no_api::listar(
        &config.bdp_responsable_no_api,
        &asignaciones,
    )))
}

/// Guarda responsable y procedimiento de una operación. Solo Admin: cambia lo que
/// verá el personal para cada operación, igual que la configuración general.
pub async fn actualizar_operacion_no_api(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(codigo): Path<String>,
    Json(req): Json<ActualizarOperacionNoApi>,
) -> Result<Json<OperacionNoApi>, AppError> {
    auth.require_role(&[UserRole::Admin])?;
    if !operaciones_no_api::es_codigo_valido(&codigo) {
        return Err(AppError::NotFound("Operación no API desconocida".into()));
    }
    let asignacion = operaciones_no_api::validar(&req.responsable, &req.procedimiento)
        .map_err(AppError::Validation)?;
    BdpOperacionesNoApiRepository::guardar(&state.pool, auth.user_id, &codigo, &asignacion)
        .await
        .map_err(|e| AppError::Internal(format!("Error guardando operación no API: {e}")))?;

    let config = ConfiguracionService::obtener(&state.pool, auth.user_id)
        .await
        .map_err(|e| AppError::Internal(format!("Error obteniendo configuración: {e}")))?;
    let (_, operacion) = operaciones_no_api::OPERACIONES_NO_API
        .iter()
        .find(|(c, _)| *c == codigo)
        .copied()
        .unwrap_or_default();
    Ok(Json(operaciones_no_api::resolver(
        &codigo,
        operacion,
        &config.bdp_responsable_no_api,
        Some(&asignacion),
    )))
}
