/* [263A-14] Plano de sala: llamar camarero.
 * Parte de plano_sala (ver mod.rs). */

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::services::bdp_weblink_catalog::BdpCallWaiterRequest;
use crate::services::{
    BdpWeblinkClient, ConfiguracionService, ModoEfectivo, ServicioModoOperacion,
};
use crate::AppState;

/* [198A-1/D10] Llamar camarero: push directo a BDP (no va a la cola). En
 * standalone no está disponible (la UI oculta el botón con el motivo). */
#[utoipa::path(
    post,
    path = "/api/plano-sala/mesas/{id}/llamar-camarero",
    tag = "PlanoSala",
    params(("id" = Uuid, Path, description = "ID de la mesa")),
    responses(
        (status = 200, description = "Aviso enviado al TPV"),
        (status = 403, description = "No disponible en modo standalone", body = ErrorResponse),
        (status = 404, description = "Mesa no encontrada", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn llamar_camarero(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (table, room) = crate::repositories::PlanoSalaRepository::codigos_mesa_para_bdp(
        &state.pool,
        id,
        auth.user_id,
    )
    .await?
    .ok_or_else(|| AppError::NotFound("Mesa no encontrada".into()))?;

    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
    let modo = ServicioModoOperacion::modo_efectivo_desde_config(&config);
    if modo == ModoEfectivo::Standalone {
        return Err(AppError::Forbidden(
            "Llamar camarero no está disponible sin BDP (modo standalone)".into(),
        ));
    }

    let client = BdpWeblinkClient::new(&config);
    let datos_enviados = serde_json::json!({ "mesa": table, "sala": room });
    /* [049A-1/H-W-1] Trazabilidad en bdp_audit_log como el resto de escrituras:
     * notificación sin estado, pero el ledger debe reflejarla con su resultado. */
    match client
        .call_waiter(&BdpCallWaiterRequest { table, room })
        .await
    {
        Ok(_) => {
            crate::services::BdpBackupService::auditar_escritura_directa(
                crate::services::AuditoriaDirecta {
                    pool: &state.pool,
                    user_id: auth.user_id,
                    operacion: "call_waiter",
                    target_entity_type: "mesa",
                    target_entity_id: id,
                    datos_enviados: &datos_enviados,
                    resultado: "exito",
                    error_mensaje: None,
                },
            )
            .await
            .map_err(AppError::Internal)?;
            Ok(Json(
                serde_json::json!({ "mensaje": "Aviso enviado al TPV" }),
            ))
        }
        Err(e) => {
            let _ = crate::services::BdpBackupService::auditar_escritura_directa(
                crate::services::AuditoriaDirecta {
                    pool: &state.pool,
                    user_id: auth.user_id,
                    operacion: "call_waiter",
                    target_entity_type: "mesa",
                    target_entity_id: id,
                    datos_enviados: &datos_enviados,
                    resultado: "error",
                    error_mensaje: Some(&e.to_string()),
                },
            )
            .await;
            Err(AppError::Internal(format!("Error llamando camarero: {e}")))
        }
    }
}
