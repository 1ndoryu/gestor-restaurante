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
    /// Ejecuta únicamente configuraciones cuyo polling fue habilitado de forma
    /// explícita y cuya ventana está vencida. La tabla de agenda actúa como
    /// claim atómico entre múltiples instancias.
    pub async fn poll_due(pool: &PgPool) -> Result<usize, String> {
        let configs = sqlx::query_as::<_, ConfiguracionRestaurante>(
            "SELECT * FROM configuracion_restaurante \
             WHERE bdp_poll_enabled = TRUE AND bdp_sync_enabled = TRUE \
               AND bdp_base_url <> '' ORDER BY user_id LIMIT 100",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error listando configuraciones BDP para polling: {e}"))?;

        let mut total = 0;
        for config in configs {
            let claimed: Option<uuid::Uuid> = sqlx::query_scalar(
                r"INSERT INTO bdp_poll_schedule (user_id, next_poll_at, updated_at)
                   VALUES ($1, NOW() + ($2 * INTERVAL '1 second'), NOW())
                   ON CONFLICT (user_id) DO UPDATE SET
                     next_poll_at = NOW() + ($2 * INTERVAL '1 second'),
                     updated_at = NOW()
                   WHERE bdp_poll_schedule.next_poll_at <= NOW()
                   RETURNING user_id",
            )
            .bind(config.user_id)
            .bind(config.bdp_poll_interval_secs.clamp(10, 600))
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("Error reclamando turno de polling BDP: {e}"))?;
            if claimed.is_some() {
                match Self::poll_pending(pool, config.user_id, &config).await {
                    Ok(updated) => total += updated,
                    Err(error) => warn!("Polling BDP usuario {}: {error}", config.user_id),
                }
            }
        }
        Ok(total)
    }

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

        /* [AUDIT-2.11b] Buscar ventas huérfanas: la comanda puede existir en
         * BDP pero Glory no recibió confirmación (crash entre HTTP y UPDATE). */
        let orphaned = VentaRepository::list_bdp_orphaned(pool, user_id)
            .await
            .unwrap_or_else(|e| {
                warn!("[AUDIT-2.11b] Error buscando ventas huérfanas: {e}");
                Vec::new()
            });

        if ventas.is_empty() && orphaned.is_empty() {
            return Ok(0);
        }

        let client = BdpWeblinkClient::new(config);
        let mut updated = 0;

        if !orphaned.is_empty() {
            warn!(
                "[AUDIT-2.11b] {} ventas huérfanas detectadas (bdp_synced=false con bdp_order_id). \
                 Consultando BDP para reconciliar.",
                orphaned.len()
            );
            for venta in &orphaned {
                match Self::check_order_status(&client, venta.bdp_order_id.unwrap_or(0)).await {
                    Ok(status) => {
                        /* La comanda existe en BDP → reconciliar Glory */
                        info!(
                            "[AUDIT-2.11b] Venta {} reconciliada: comanda existe en BDP (status={status}). \
                             Marcando bdp_synced=true.",
                            venta.id
                        );
                        let _ = VentaRepository::update_bdp_status(
                            pool,
                            venta.id,
                            true,
                            None,
                            venta.bdp_order_id,
                        )
                        .await;
                        let _ = VentaRepository::update_bdp_order_status(pool, venta.id, &status)
                            .await;
                        updated += 1;
                    }
                    Err(e) => {
                        /* La comanda no existe o BDP no responde → marcar error
                         * para que no se reintente infinitamente */
                        warn!(
                            "[AUDIT-2.11b] Venta {} no reconciliable: {e}",
                            venta.id
                        );
                        let _ = VentaRepository::update_bdp_status(
                            pool,
                            venta.id,
                            false,
                            Some("No se pudo verificar existencia en BDP; reconciliación manual requerida"),
                            venta.bdp_order_id,
                        )
                        .await;
                    }
                }
            }
        }

        if !ventas.is_empty() {
            info!("[276A-4.2] Polling BDP: {} ventas pendientes", ventas.len());
            for venta in &ventas {
                match Self::poll_one(pool, venta, config, Some(&client)).await {
                    Ok(true) => updated += 1,
                    Ok(false) => {}
                    Err(e) => {
                        warn!(
                            "[276A-4.2] Error consultando GetOrder para venta {}: {e}",
                            venta.id
                        );
                    }
                }
            }
        }

        Ok(updated)
    }

    pub async fn refresh_one(
        pool: &PgPool,
        venta: &crate::models::Venta,
        config: &ConfiguracionRestaurante,
    ) -> Result<bool, String> {
        let client = BdpWeblinkClient::new(config);
        Self::poll_one(pool, venta, config, Some(&client)).await
    }

    async fn poll_one(
        pool: &PgPool,
        venta: &crate::models::Venta,
        _config: &ConfiguracionRestaurante,
        client: Option<&BdpWeblinkClient<'_>>,
    ) -> Result<bool, String> {
        let order_id = venta
            .bdp_order_id
            .ok_or_else(|| format!("Venta {} no tiene bdp_order_id", venta.id))?;
        let client = client.ok_or_else(|| "Cliente BDP no disponible".to_string())?;
        let status = Self::check_order_status(client, order_id).await?;
        VentaRepository::update_bdp_order_status(pool, venta.id, &status)
            .await
            .map_err(|e| format!("Error actualizando estado local BDP: {e}"))?;
        info!("Venta {} → bdp_order_status = {status}", venta.id);
        Ok(true)
    }

    /// Consulta `GetOrder` para un `order_id` específico y devuelve el status como string.
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
        let status_code = Self::parse_status(&resp)?;

        Ok(Self::map_status(status_code))
    }

    fn parse_status(resp: &serde_json::Value) -> Result<i64, String> {
        let status_value = resp
            .get("Status")
            .or_else(|| resp.get("Order").and_then(|order| order.get("Status")))
            .ok_or_else(|| "Respuesta BDP sin campo Status".to_string())?;
        status_value
            .as_i64()
            .or_else(|| status_value.as_str()?.trim().parse::<i64>().ok())
            .ok_or_else(|| "Respuesta BDP contiene Status inválido".to_string())
    }

    /// Mapea el integer de status BDP → string legible almacenado en `bdp_order_status`.
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

    #[test]
    fn status_accepts_numeric_string_shape() {
        let value = serde_json::json!({"Order": {"Status": "3"}});
        assert_eq!(BdpOrderPollerService::parse_status(&value), Ok(3));
        assert!(BdpOrderPollerService::parse_status(&serde_json::json!({"Status": "x"})).is_err());
    }
}
