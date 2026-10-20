/* [D32] Operaciones que BDP no expone por API, con su procedimiento y responsable.
 * Solo lectura: la web muestra la lista, no ejecuta nada en BDP. */

use super::{AppError, AppState, AuthUser, ConfiguracionService, Json, State};
use crate::services::bdp_operaciones_no_api::{listar, OperacionNoApi};

pub async fn listar_operaciones_no_api(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<OperacionNoApi>>, AppError> {
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id)
        .await
        .map_err(|e| AppError::Internal(format!("Error obteniendo configuración: {e}")))?;
    Ok(Json(listar(&config.bdp_responsable_no_api)))
}
