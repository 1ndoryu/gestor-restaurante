/* [039A-1/H-P1-02][039A-1/H-P1-03] Fail-closed (N1): en modo independiente
 * ningún flujo que requiera conectarse al BDP se ejecuta. `modo_efectivo`
 * solo lee config+cache (M2/M3), nunca hace red, así que este guard no puede
 * generar tráfico. Compartido por los handlers BDP que conectan directo
 * (article_map, customer_sync, backup/explorar): una sola definición evita
 * que un handler nuevo copie el guard y se olvide de él. */

use uuid::Uuid;

use crate::errors::AppError;
use crate::services::ModoEfectivo;
use crate::AppState;

pub async fn exigir_modo_bdp(state: &AppState, user_id: Uuid) -> Result<(), AppError> {
    let modo = state
        .modo_operacion
        .modo_efectivo(&state.pool, user_id)
        .await?;
    if modo != ModoEfectivo::Bdp {
        return Err(AppError::Validation(
            "BDP no disponible: el sistema está en modo independiente.".into(),
        ));
    }
    Ok(())
}

/* [21AB-5] Etapa `lectura`: la web no pregunta nada al TPV mientras esté
 * apagada. Va aparte de `exigir_modo_bdp` porque no todos los handlers que
 * tocan BDP son lecturas (los de configuración y los permisos no lo son), y
 * porque apagar la etapa no debe dejar la pantalla sin abrir. */
pub async fn exigir_etapa_lectura(state: &AppState, user_id: Uuid) -> Result<(), AppError> {
    let activa = crate::services::etapa_activa(&state.pool, user_id, "lectura")
        .await
        .map_err(AppError::Internal)?;
    if !activa {
        return Err(AppError::Validation(
            "La etapa de lectura de BDP está apagada: enciéndala en Configuración → BDP para consultar el TPV.".into(),
        ));
    }
    Ok(())
}
