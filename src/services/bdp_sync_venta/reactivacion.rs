// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [M21] Recuperación automática de ventas que quedaron «en el aire».
 *
 * Una comanda cuyo envío acabó `ambiguo` (respuesta perdida, timeout, 5xx) deja
 * una auditoría abierta y `ensure_no_unresolved` bloquea cualquier intento
 * posterior: la venta solo salía de ahí si BDP había creado la comanda y el
 * poller la encontraba. Si NO la había creado, quedaba muerta para siempre —el
 * botón manual también choca con el mismo bloqueo—.
 *
 * Turno del poller: se pregunta a BDP por el `MarketplaceOrderId` de cada venta
 * con auditoría abierta.
 *   - Existe  → se recupera: comanda, auditoría y mapeo, igual que el poller.
 *   - No existe y BDP contestó un error de negocio → se cierra la auditoría y se
 *     envía **una sola vez**.
 *   - No se pudo preguntar (transporte caído) → no se toca nada. Reenviar a
 *     ciegas es justo lo que esta pieza evita.
 *
 * No se clasifica por código de error: de `GetOrder` por MarketplaceOrderId con
 * una comanda inexistente no hay ninguna respuesta de BDP confirmada (solo por
 * OrderId, `[301000]`), y el simulador devuelve un texto sin código. La regla
 * segura es «respondió con un error de negocio = no la tiene».
 */

use sqlx::PgPool;
use tracing::{info, warn};

use crate::models::ConfiguracionRestaurante;
use crate::services::bdp_sync::{BdpSyncService, BDP_SYNC_MARKET_ID};
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{BdpGetOrderRequest, BdpOrderIdentifier};

/// Qué dice BDP sobre la comanda de una venta.
pub(crate) enum EstadoComanda {
    /// Existe: ya se creó aunque la respuesta se perdiera. Lleva la respuesta
    /// entera porque su `Status` también manda (una comanda cancelada en BDP
    /// anula la venta en la web).
    Existe(i64, serde_json::Value),
    /// BDP está vivo y contesta que no la tiene.
    NoExiste,
    /// No se pudo averiguar (transporte, credenciales, respuesta ilegible).
    Desconocido,
}

impl BdpSyncService {
    /// Turno del poller: recupera o reabre las ventas con la comanda en el aire.
    /// Recibe la lista ya leída (el poller la tiene y evita otra consulta) y
    /// devuelve cuántas quedaron resueltas, sin contar las que siguen en duda.
    pub(crate) async fn recuperar_lista_en_vuelo(
        pool: &PgPool,
        user_id: uuid::Uuid,
        config: &ConfiguracionRestaurante,
        client: &BdpWeblinkClient<'_>,
        ventas: Vec<crate::models::Venta>,
    ) -> usize {
        let mut resueltas = 0;
        for venta in ventas {
            match Self::estado_comanda(client, venta.id).await {
                EstadoComanda::Existe(order_id, respuesta) => {
                    if Self::recuperar_comanda(pool, user_id, &venta, order_id, &respuesta).await {
                        /* [18AB-2] Recuperada con su pago dentro: la factura,
                         * que antes pedía el `CreateOrder`, se emite ahora. */
                        Self::facturar_orden_cobrada(pool, &venta, config).await;
                        resueltas += 1;
                    }
                }
                EstadoComanda::NoExiste => {
                    /* BDP la tiene por inexistente: se cierra la auditoría y se
                     * reenvía una vez. `is_update = false`: es una creación. */
                    if Self::liberar_auditoria_abierta(pool, user_id, venta.id).await {
                        info!(
                            "[M21] Comanda inexistente en BDP para venta {}: se reenvía una vez",
                            venta.id
                        );
                        Self::sync_venta(pool, &venta, config, false, None).await;
                        resueltas += 1;
                    }
                }
                EstadoComanda::Desconocido => {
                    info!(
                        "[M21] No se pudo preguntar por la comanda de la venta {}; se mantiene en el aire",
                        venta.id
                    );
                }
            }
        }
        resueltas
    }

    /// Cierra comanda, venta, auditoría y mapeo en una transacción, igual que
    /// `confirmar_orden` pero sin tocar `bdp_sync_error` salvo para limpiarlo.
    /// Devuelve `false` si la transacción no se pudo cerrar (queda `ambiguo`).
    pub(crate) async fn recuperar_comanda(
        pool: &PgPool,
        user_id: uuid::Uuid,
        venta: &crate::models::Venta,
        order_id: i64,
        respuesta_bdp: &serde_json::Value,
    ) -> bool {
        let respuesta = serde_json::json!({ "order_id": order_id });
        let resultado = async {
            let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
            sqlx::query(
                "UPDATE ventas SET bdp_synced = true, bdp_synced_at = NOW(), bdp_order_id = $2, \
                        bdp_sync_error = NULL WHERE id = $1",
            )
            .bind(venta.id)
            .bind(order_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
            /* [D10] La comanda se recuperó con su estado delante: si BDP la da
             * por cancelada, la venta queda anulada en la web en la misma
             * transacción. */
            if let Ok(status) = crate::services::bdp_estado_orden(respuesta_bdp) {
                crate::repositories::VentaRepository::update_bdp_order_status(
                    &mut *tx,
                    venta.id,
                    &crate::services::bdp_estado_nombre(status),
                )
                .await
                .map_err(|e| e.to_string())?;
            }
            sqlx::query(
                r"UPDATE bdp_audit_log
                  SET resultado = 'exito', datos_respuesta = $2, error_mensaje = NULL, updated_at = NOW()
                  WHERE user_id = $1 AND target_entity_type = 'venta' AND target_entity_id = $3
                    AND operacion = 'create_order' AND resultado IN ('pendiente', 'ambiguo')",
            )
            .bind(user_id)
            .bind(Some(&respuesta))
            .bind(venta.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
            sqlx::query(
                r"INSERT INTO bdp_id_map (id, user_id, entidad, glory_id, bdp_id, market_id, marketplace_order_id)
                  VALUES ($1, $2, 'venta', $3, $4, $5, $6)
                  ON CONFLICT (user_id, entidad, glory_id)
                  DO UPDATE SET bdp_id = EXCLUDED.bdp_id, market_id = EXCLUDED.market_id,
                                marketplace_order_id = EXCLUDED.marketplace_order_id,
                                actualizado_en = NOW()",
            )
            .bind(uuid::Uuid::new_v4())
            .bind(user_id)
            .bind(venta.id.to_string())
            .bind(order_id.to_string())
            .bind(BDP_SYNC_MARKET_ID.to_string())
            .bind(Self::marketplace_order_id(venta.id))
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
            tx.commit().await.map_err(|e| e.to_string())
        }
        .await;
        match resultado {
            Ok(()) => {
                info!(
                    "[M21] Venta {} recuperada de BDP sin reenviar → OrderId={order_id}",
                    venta.id
                );
                true
            }
            Err(error) => {
                warn!("[M21] No se pudo recuperar la venta {}: {error}", venta.id);
                false
            }
        }
    }

    /// Pregunta a BDP por la comanda de `venta_id` usando su `MarketplaceOrderId`.
    pub(crate) async fn estado_comanda(
        client: &BdpWeblinkClient<'_>,
        venta_id: uuid::Uuid,
    ) -> EstadoComanda {
        let request = BdpGetOrderRequest {
            order_identifier: BdpOrderIdentifier::by_market(
                BDP_SYNC_MARKET_ID,
                Self::marketplace_order_id(venta_id),
            ),
        };
        match client.get_order(&request).await {
            Ok(response) => {
                let order = response.get("Order").unwrap_or(&response);
                match order.get("OrderId").and_then(serde_json::Value::as_i64) {
                    Some(order_id) if order_id > 0 => {
                        EstadoComanda::Existe(order_id, response.clone())
                    }
                    /* Contesta sin comanda y sin error: tampoco hay evidencia de
                     * que no exista, y con esto no se reenvía. */
                    _ => EstadoComanda::Desconocido,
                }
            }
            /* `Remote` es la respuesta de negocio del TPV («no existe…»): está
             * vivo y dice que no la tiene. `Health` antes de fiarse: si el texto
             * es en realidad un fallo de sesión o de configuración, el TPV no
             * está sano y no se reenvía nada. */
            Err(crate::services::bdp_weblink::BdpWeblinkError::Remote(mensaje)) => {
                match client.health().await {
                    Ok(health) if health.is_alive => {
                        info!("[M21] BDP no tiene la comanda de {venta_id}: {mensaje}");
                        EstadoComanda::NoExiste
                    }
                    _ => EstadoComanda::Desconocido,
                }
            }
            Err(error) => {
                info!("[M21] GetOrder por MarketplaceOrderId falló para {venta_id}: {error}");
                EstadoComanda::Desconocido
            }
        }
    }

    /// Cierra la auditoría `create_order` abierta de una venta que BDP no tiene,
    /// para que el guard de escritura deje pasar el envío. Solo cierra si la
    /// marca sigue intacta. `true` si quedó libre para reenviar.
    pub(crate) async fn liberar_auditoria_abierta(
        pool: &PgPool,
        user_id: uuid::Uuid,
        venta_id: uuid::Uuid,
    ) -> bool {
        /* Se relee la marca tras confirmar con BDP: si cambió, otra instancia
         * está enviando ahora mismo y su auditoría es la que manda. */
        let marca: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
            "SELECT MAX(updated_at) FROM bdp_audit_log \
             WHERE user_id = $1 AND target_entity_type = 'venta' AND target_entity_id = $2 \
               AND operacion = 'create_order' AND resultado IN ('pendiente', 'ambiguo')",
        )
        .bind(user_id)
        .bind(venta_id)
        .fetch_one(pool)
        .await
        .unwrap_or(None);
        let Some(marca) = marca else {
            return true;
        };
        let nota =
            "BDP no tiene la comanda (consultada por MarketplaceOrderId); se reenvía una vez";
        let cerradas = sqlx::query(
            "UPDATE bdp_audit_log SET resultado = 'error', error_mensaje = $4, updated_at = NOW() \
             WHERE user_id = $1 AND target_entity_type = 'venta' AND target_entity_id = $2 \
               AND operacion = 'create_order' AND resultado IN ('pendiente', 'ambiguo') \
               AND updated_at = $3",
        )
        .bind(user_id)
        .bind(venta_id)
        .bind(marca)
        .bind(nota)
        .execute(pool)
        .await;
        match cerradas {
            Ok(resultado) if resultado.rows_affected() > 0 => true,
            Ok(_) => {
                warn!("[M21] Auditoría de la venta {venta_id} cambió mientras se reconciliaba; no se toca");
                false
            }
            Err(error) => {
                warn!("[M21] No se pudo liberar la auditoría de la venta {venta_id}: {error}");
                false
            }
        }
    }
}
