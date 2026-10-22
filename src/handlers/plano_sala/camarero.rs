/* [263A-14] Plano de sala: llamar camarero.
 * Parte de plano_sala (ver mod.rs). */

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::services::bdp_weblink::BdpWeblinkError;
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
    /* [10AA-4] Es una escritura directa al TPV —no pasa por la cola ni por el
     * armado, así que no hay permiso que gastar— y por eso no escribe en solo
     * lectura. Antes solo miraba el modo maestro: con la cuenta en `read_only`
     * el aviso salía igual y nada lo frenaba.
     * Se deja pasar en `unidirectional`: ahí es la persona la que pulsa, y el
     * modo significa que puede haber escrituras autorizadas a mano. */
    if config.bdp_sync_mode == "read_only" {
        return Err(AppError::Forbidden(
            "Llamar camarero está bloqueado: el modo de sincronización es 'read_only' (solo lectura).".into(),
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
            /* [D31] El TPV contestó con un error de negocio (p. ej. el terminal
             * no tiene IP de Servidor de Mensajes): el resultado se registra
             * como `rechazado`, no como `error` de transporte. */
            let rechazado = matches!(e, BdpWeblinkError::Remote(_));
            let _ = crate::services::BdpBackupService::auditar_escritura_directa(
                crate::services::AuditoriaDirecta {
                    pool: &state.pool,
                    user_id: auth.user_id,
                    operacion: "call_waiter",
                    target_entity_type: "mesa",
                    target_entity_id: id,
                    datos_enviados: &datos_enviados,
                    resultado: if rechazado { "rechazado" } else { "error" },
                    error_mensaje: Some(&e.to_string()),
                },
            )
            .await;
            Err(error_llamar_camarero(e))
        }
    }
}

/* [D31] Un rechazo del TPV es un mensaje para el usuario, no un fallo interno:
 * `No se ha establecido una IP de Servidor de Mensajes…` es una
 * misconfiguración del terminal y quien llama tiene que leerla. Solo lo que no
 * llegó al TPV sigue siendo 500. */
fn error_llamar_camarero(e: BdpWeblinkError) -> AppError {
    match e {
        BdpWeblinkError::Remote(mensaje) => {
            AppError::Conflict(format!("El TPV rechazó el aviso: {mensaje}"))
        }
        BdpWeblinkError::NotConfigured => AppError::Forbidden("BDP no está configurado".into()),
        otro => AppError::Internal(format!("Error llamando camarero: {otro}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /* [D31] Lo comprobado con el TPV local el 2026-10-21: el aviso llegaba y el
     * terminal contestaba `[404602]-No se ha establecido una IP de Servidor de
     * Mensajes…`, que salía como 500 «Ocurrió un error interno». El operario no
     * podía saber qué arreglar y la auditoría lo contaba como caída. */
    #[test]
    fn rechazo_del_tpv_es_conflicto_con_su_mensaje() {
        let error = error_llamar_camarero(BdpWeblinkError::Remote(
            "[404602]-No se ha establecido una IP de Servidor de Mensajes".into(),
        ));
        assert!(matches!(error, AppError::Conflict(_)), "{error:?}");
        assert!(
            error.to_string().contains("404602"),
            "el mensaje del TPV se conserva: {error}"
        );
    }

    #[test]
    fn error_de_transporte_sigue_siendo_interno() {
        let error = error_llamar_camarero(BdpWeblinkError::Http("timeout".into()));
        assert!(matches!(error, AppError::Internal(_)), "{error:?}");
    }
}
