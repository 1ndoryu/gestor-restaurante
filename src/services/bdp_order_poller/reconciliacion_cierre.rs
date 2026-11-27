// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
use sqlx::PgPool;
use tracing::{info, warn};

use super::BdpOrderPollerService;
use crate::services::bdp_weblink::{BdpWeblinkClient, BdpWeblinkError};

/* Cierre de auditorías que no se aplicaron: la parte de la reconciliación que
 * solo necesita dejar constancia, no comparar con BDP. Vive aparte porque es un
 * dominio propio y `reconciliacion.rs` ya iba justo de tamaño. */
impl BdpOrderPollerService {
    /// Cierra una auditoría sin tocar la venta: la escritura no se aplicó y la
    /// web nunca cambió, así que no hay nada que deshacer. `exito` cuando BDP
    /// demuestra que sí se aplicó; `error` cuando demuestra que no.
    pub(super) async fn cerrar_auditoria(
        pool: &PgPool,
        audit_id: uuid::Uuid,
        resultado: &str,
        datos_respuesta: Option<&serde_json::Value>,
        error_mensaje: Option<&str>,
    ) -> Result<(), String> {
        crate::services::BdpBackupService::actualizar_resultado(
            pool,
            audit_id,
            resultado,
            datos_respuesta,
            error_mensaje,
        )
        .await
    }

    /// [21AB-5g] ¿El fallo de `GetOrder` demuestra que la comanda no existe?
    /// Solo si es respuesta de negocio (`Remote`) **y** `Health` confirma que el
    /// TPV está sano: un texto de sesión o de configuración también llega como
    /// `Remote` y no prueba nada (mismo predicado que `estado_comanda`). Cuando
    /// lo demuestra, cierra la auditoría en `error` y devuelve `true`.
    pub(super) async fn cerrar_si_comanda_inexistente(
        pool: &PgPool,
        client: &BdpWeblinkClient<'_>,
        audit_id: uuid::Uuid,
        order_id: i64,
        error: &BdpWeblinkError,
        consecuencia: &str,
    ) -> Result<bool, String> {
        let BdpWeblinkError::Remote(mensaje) = error else {
            return Ok(false);
        };
        match client.health().await {
            Ok(health) if health.is_alive => {
                Self::cerrar_auditoria(
                    pool,
                    audit_id,
                    "error",
                    None,
                    Some(&format!(
                        "reconciliado: BDP no tiene la comanda {order_id} ({mensaje}); {consecuencia}"
                    )),
                )
                .await?;
                info!(
                    "[21AB-5g] Comanda {order_id} inexistente en BDP; auditoría cerrada en error"
                );
                Ok(true)
            }
            _ => {
                warn!("[21AB-5g] GetOrder de la comanda {order_id} respondió «{mensaje}» con el TPV no sano; la auditoría sigue abierta");
                Ok(false)
            }
        }
    }

    pub(super) async fn find_bdp_order_id_for_venta(
        pool: &PgPool,
        venta_id: uuid::Uuid,
    ) -> Result<Option<i64>, String> {
        let order_id: Option<i64> =
            sqlx::query_scalar("SELECT bdp_order_id FROM ventas WHERE id = $1")
                .bind(venta_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| e.to_string())?;
        Ok(order_id)
    }
}
