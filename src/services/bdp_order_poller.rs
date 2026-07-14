/* [276A-4.2] Servicio de polling BDP — consulta periódicamente el estado de comandas
 * enviadas al POS para detectar facturación (Status=3) u otros estados finales.
 *
 * Flujo:
 *   1. VentaRepository::list_bdp_pending() → ventas con bdp_synced=true, bdp_order_status no final
 *   2. Para cada venta: GetOrder(bdp_order_id) → Status de BDP
 *   3. Mapear status integer → string legible: pending(0), accepted(1), cancelled(2), invoiced(3)
 *   4. Actualizar bdp_order_status en la venta
 *
 * Status BDP (de la API Weblink REST, estructura Order):
 *   0 = Esperando validación → "pending"
 *   1 = Aceptada por el establecimiento → "accepted"
 *   2 = Cancelada → "cancelled"
 *   3 = Facturada → "invoiced"
 *
 * Gotchas:
 *   - GetOrder gratuito solo devuelve Status, no el Order completo (limitación subscripción).
 *   - El intervalo de polling se configura en bdp_poll_interval_secs (configuración restaurante).
 *   - Cada venta tiene un mutex para evitar polling concurrente del mismo order. */

use sqlx::PgPool;
use tracing::{info, warn};

use crate::models::ConfiguracionRestaurante;
use crate::repositories::VentaRepository;
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{BdpGetOrderRequest, BdpOrderIdentifier};

pub struct BdpOrderPollerService;

impl BdpOrderPollerService {
    /// Consulta BDP para todas las ventas pendientes de este usuario.
    /// Retorna el número de ventas actualizadas.
    pub async fn poll_pending(
        pool: &PgPool,
        user_id: uuid::Uuid,
        config: &ConfiguracionRestaurante,
    ) -> Result<usize, String> {
        if !config.bdp_sync_enabled {
            return Ok(0);
        }

        let ventas = VentaRepository::list_bdp_pending(pool, user_id)
            .await
            .map_err(|e| format!("Error consultando ventas BDP pendientes: {e}"))?;

        if ventas.is_empty() {
            return Ok(0);
        }

        info!(
            "[276A-4.2] Polling BDP: {} ventas pendientes",
            ventas.len()
        );

        let client = BdpWeblinkClient::new(config);
        let mut updated = 0;

        for venta in &ventas {
            let order_id = match venta.bdp_order_id {
                Some(id) => id,
                None => {
                    warn!(
                        "[276A-4.2] Venta {} tiene bdp_synced=true pero sin bdp_order_id, skip",
                        venta.id
                    );
                    continue;
                }
            };

            match Self::check_order_status(&client, order_id).await {
                Ok(status_str) => {
                    if let Err(e) =
                        VentaRepository::update_bdp_order_status(pool, venta.id, &status_str).await
                    {
                        warn!(
                            "[276A-4.2] Error actualizando bdp_order_status de venta {}: {e}",
                            venta.id
                        );
                    } else {
                        info!(
                            "[276A-4.2] Venta {} → bdp_order_status = {}",
                            venta.id, status_str
                        );
                        updated += 1;
                    }
                }
                Err(e) => {
                    warn!(
                        "[276A-4.2] Error consultando GetOrder para venta {} (order {}): {e}",
                        venta.id, order_id
                    );
                    /* No marcamos como error — reintentará en el próximo ciclo.
                     * Solo errores definitivos (orden no existe) marcarían 'error'. */
                }
            }
        }

        Ok(updated)
    }

    /// Consulta GetOrder para un order_id específico y devuelve el status como string.
    async fn check_order_status(
        client: &BdpWeblinkClient<'_>,
        order_id: i64,
    ) -> Result<String, String> {
        let req = BdpGetOrderRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
        };

        let resp = client
            .get_order(&req)
            .await
            .map_err(|e| format!("Error BDP GetOrder: {e}"))?;

        /* La respuesta de GetOrder tiene:
         *   { "Order": { ... }, "Status": <int>, "ErrorMessage": "" }
         * Status values: 0=pending, 1=accepted, 2=cancelled, 3=invoiced */
        let status_code = resp
            .get("Status")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| "Respuesta BDP sin campo Status".to_string())?;

        let error_msg = resp
            .get("ErrorMessage")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if !error_msg.is_empty() {
            /* Error remoto de BDP — puede indicar que la orden ya no existe
             * o subscripción expirada. No marcamos como error definitivo aún. */
            warn!(
                "[276A-4.2] GetOrder(order {}) devolvió ErrorMessage: {}",
                order_id, error_msg
            );
        }

        Ok(Self::map_status(status_code))
    }

    /// Mapea el integer de status BDP → string legible almacenado en bdp_order_status.
    fn map_status(code: i64) -> String {
        match code {
            0 => "pending".to_string(),
            1 => "accepted".to_string(),
            2 => "cancelled".to_string(),
            3 => "invoiced".to_string(),
            other => format!("unknown_{other}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_status() {
        assert_eq!(BdpOrderPollerService::map_status(0), "pending");
        assert_eq!(BdpOrderPollerService::map_status(1), "accepted");
        assert_eq!(BdpOrderPollerService::map_status(2), "cancelled");
        assert_eq!(BdpOrderPollerService::map_status(3), "invoiced");
        assert_eq!(BdpOrderPollerService::map_status(99), "unknown_99");
    }
}
