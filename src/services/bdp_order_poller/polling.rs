// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
use sqlx::PgPool;
use tracing::{info, warn};

use super::BdpOrderPollerService;
use crate::models::ConfiguracionRestaurante;
use crate::repositories::{VentaLineaRepository, VentaRepository};
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{BdpGetOrderRequest, BdpOrderIdentifier};
use crate::services::BdpSyncService;
use crate::services::{ModoEfectivo, ServicioModoOperacion};

/* Ciclo de polling: decide qué configuración sondear, reclama el turno y
 * actualiza el estado local de cada venta a partir de GetOrder. */
impl BdpOrderPollerService {
    /// Ejecuta únicamente configuraciones cuyo polling fue habilitado de forma
    /// explícita y cuya ventana está vencida. La tabla de agenda actúa como
    /// claim atómico entre múltiples instancias.
    pub async fn poll_due(
        pool: &PgPool,
        servicio: &ServicioModoOperacion,
    ) -> Result<usize, String> {
        let configs = sqlx::query_as::<_, ConfiguracionRestaurante>(
            "SELECT * FROM configuracion_restaurante \
             WHERE bdp_poll_enabled = TRUE \
               AND bdp_base_url <> '' AND modo_operacion <> 'standalone' \
               ORDER BY user_id LIMIT 100",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error listando configuraciones BDP para polling: {e}"))?;

        let mut total = 0;
        for config in configs {
            /* [128A-1/F1-1] M1: si la configuración ya no apunta a BDP (standalone,
             * o auto sin sync/credenciales) no hay nada que sondear ni reclamar. */
            if ServicioModoOperacion::modo_efectivo_desde_config(&config) != ModoEfectivo::Bdp {
                continue;
            }
            /* [21AB-5] Etapa `lectura` apagada: el sondeo es la lectura
             * periódica de la web, así que no se pregunta nada al TPV. Se
             * desbloquea en cuanto se enciende la etapa. */
            if !crate::services::etapa_activa(pool, config.user_id, "lectura")
                .await
                .unwrap_or(false)
            {
                continue;
            }
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
                /* [149A-1/P1.3] Si el modo efectivo degradó (M2: fallos consecutivos),
                 * el poller no puede llamar a BDP — y sin llamada no hay éxito que
                 * resetee el contador, que vive en memoria: la degradación quedaba
                 * permanente hasta reiniciar. El turno ya reclamado (misma cadencia
                 * que el polling) se usa para una sonda barata de recuperación. */
                if servicio.modo_efectivo_sin_red(&config) == ModoEfectivo::Bdp {
                    match Self::poll_pending(pool, config.user_id, &config, servicio).await {
                        Ok(updated) => total += updated,
                        Err(error) => warn!("Polling BDP usuario {}: {error}", config.user_id),
                    }
                } else {
                    Self::probar_recuperacion(&config, servicio).await;
                }
            }
        }
        Ok(total)
    }

    /// [149A-1/P1.3] Sonda de recuperación tras una degradación M2.
    /// Solo `Health` (no abre sesión ni toca datos): si el BDP vuelve a responder,
    /// un éxito resetea el contador y el modo efectivo retorna a Bdp en la
    /// siguiente consulta. Si sigue caído, no hace nada y se reintenta en el
    /// siguiente turno de la agenda (sin bucle apretado).
    async fn probar_recuperacion(
        config: &ConfiguracionRestaurante,
        servicio: &ServicioModoOperacion,
    ) {
        let client = BdpWeblinkClient::new(config);
        match client.health().await {
            Ok(health) if health.is_alive => {
                servicio.registrar_exito_bdp(config.user_id);
                info!(
                    "[149A-1/P1.3] BDP responde de nuevo (usuario {}): se retoma el modo BDP",
                    config.user_id
                );
            }
            Ok(_) => warn!(
                "[149A-1/P1.3] BDP responde Health pero IsAlive=false (usuario {}): sigue en local",
                config.user_id
            ),
            Err(error) => info!(
                "[149A-1/P1.3] BDP sigue sin responder (usuario {}): {error}. Se mantiene el modo local",
                config.user_id
            ),
        }
    }

    /// Consulta BDP para todas las ventas pendientes de este usuario.
    /// Retorna el número de ventas actualizadas.
    pub async fn poll_pending(
        pool: &PgPool,
        user_id: uuid::Uuid,
        config: &ConfiguracionRestaurante,
        servicio: &ServicioModoOperacion,
    ) -> Result<usize, String> {
        /* [128A-1/F1-1] M1: standalone nunca llama a BDP. [F1-2] M2: si el
         * modo degradó por fallos consecutivos, el poller también se detiene. */
        if servicio.modo_efectivo_sin_red(config) != ModoEfectivo::Bdp {
            return Ok(0);
        }

        /* [21AB-8] Antes de mirar lo que ya se conoce: enterarse de lo que no.
         * Va fuera del corte temprano porque una comanda creada en el TPV no
         * tiene venta local que mirar. */
        let client = BdpWeblinkClient::new(config);
        let (descubiertas, mut fallo_bdp) =
            Self::descubrir_comandas_tpv(pool, user_id, &client).await;
        let mut llamo_bdp = true;
        let mut updated = descubiertas;

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

        /* [AUDIT-N2] Buscar clientes huérfanos: bdp_synced=true pero auditoría
         * pendiente/ambiguo para create_customer. */
        let orphaned_customers =
            crate::repositories::VentaRepository::list_bdp_orphaned_customers(pool, user_id)
                .await
                .unwrap_or_else(|e| {
                    warn!("[AUDIT-N2] Error buscando clientes huérfanos: {e}");
                    Vec::new()
                });

        /* [D23b] Ventas con el acuse perdido: sin `bdp_order_id` no aparecen en
         * ninguna de las dos listas de arriba, y son justo las que la
         * reconciliación por `MarketplaceOrderId` sí puede recuperar. Si no se
         * cuentan aquí, el corte de abajo deja pasar el turno sin mirarlas y la
         * venta queda invisible para siempre. */
        let en_vuelo = VentaRepository::list_bdp_en_vuelo(pool, user_id)
            .await
            .unwrap_or_else(|e| {
                warn!("[D23b] Error buscando ventas con el acuse perdido: {e}");
                Vec::new()
            });

        /* [21AB-5d] Una modificación ambigua no la trae ninguna de las listas de
         * ventas (la comanda existe y la venta puede estar ya facturada), así que
         * el corte temprano la dejaría bloqueada sin que nadie la mirase. */
        let hay_auditorias_abiertas = Self::hay_auditorias_abiertas(pool, user_id).await;

        if ventas.is_empty()
            && orphaned.is_empty()
            && orphaned_customers.is_empty()
            && en_vuelo.is_empty()
            && !hay_auditorias_abiertas
        {
            /* [21AB-8] Nada de lo conocido tiene trabajo, pero el listado sí se
             * hizo: se alimenta la histéresis igual que el resto de la pasada. */
            if llamo_bdp {
                if fallo_bdp {
                    servicio.registrar_fallo_bdp(user_id);
                } else {
                    servicio.registrar_exito_bdp(user_id);
                }
            }
            return Ok(updated);
        }

        /* [R1] Reconciliar auditorías ambiguas antes de procesar estados normales.
         * [D23b] Lo reconciliado aquí también cuenta: si no, la venta se recupera
         * de verdad pero la pantalla dice «0 actualizadas» y parece que no se
         * hizo nada. */
        match Self::reconcile_ambiguous(pool, user_id, config, &client).await {
            Ok(count) => {
                if count > 0 {
                    info!(
                        "[R1] {} auditorías ambiguas reconciliadas para usuario {}",
                        count, user_id
                    );
                    llamo_bdp = true;
                    updated += count;
                }
            }
            Err(error) => {
                llamo_bdp = true;
                fallo_bdp = true;
                warn!("[R1] Error reconciliando auditorías ambiguas: {error}");
            }
        }

        /* [M21] Las comandas en el aire se resuelven preguntando una vez por
         * `MarketplaceOrderId`; va después del cierre de auditorías para no
         * repetir el mismo `GetOrder` en la misma pasada. */
        let (resueltas, pregunto) =
            Self::resolver_comandas_en_vuelo(pool, user_id, config, &client, en_vuelo, &ventas)
                .await;
        llamo_bdp |= pregunto;
        updated += resueltas;

        if !orphaned.is_empty() {
            llamo_bdp = true;
            let (reconciliadas, fallo) =
                Self::reconciliar_huerfanas(pool, &client, &orphaned).await;
            updated += reconciliadas;
            fallo_bdp |= fallo;
        }

        updated += Self::cerrar_auditorias_clientes(pool, user_id, &orphaned_customers).await;

        let (actualizadas, hubo_fallo) =
            Self::consultar_ventas_pendientes(pool, config, &client, &ventas).await;
        updated += actualizadas;
        llamo_bdp |= !ventas.is_empty();
        fallo_bdp |= hubo_fallo;

        /* [128A-1/F1-2] M2: alimentar la histéresis del conmutador. Solo se
         * registra si realmente se intentó llamar a BDP (no-op no altera el
         * contador, para no enmascarar una degradación activa). */
        if llamo_bdp {
            if fallo_bdp {
                servicio.registrar_fallo_bdp(user_id);
            } else {
                servicio.registrar_exito_bdp(user_id);
            }
        }

        Ok(updated)
    }

    /* [276A-4.2] Consulta el estado de cada venta pendiente. Devuelve cuántas
     * cambiaron y si algún `GetOrder` falló (para la histéresis del conmutador). */
    async fn consultar_ventas_pendientes(
        pool: &PgPool,
        config: &ConfiguracionRestaurante,
        client: &BdpWeblinkClient<'_>,
        ventas: &[crate::models::Venta],
    ) -> (usize, bool) {
        if ventas.is_empty() {
            return (0, false);
        }
        info!("[276A-4.2] Polling BDP: {} ventas pendientes", ventas.len());
        let (mut actualizadas, mut fallo) = (0, false);
        for venta in ventas {
            match Self::poll_one(pool, venta, config, Some(client)).await {
                Ok(true) => actualizadas += 1,
                Ok(false) => {}
                Err(e) => {
                    fallo = true;
                    warn!(
                        "[276A-4.2] Error consultando GetOrder para venta {}: {e}",
                        venta.id
                    );
                }
            }
        }
        (actualizadas, fallo)
    }

    /* [M21] Ventas con la comanda en el aire: se recuperan si BDP la tiene y se
     * reenvían una sola vez si BDP dice que no. Las que el cierre de auditorías
     * acaba de recuperar se descartan de la lista para no pedir dos veces la
     * misma comanda en la misma pasada. */
    async fn resolver_comandas_en_vuelo(
        pool: &PgPool,
        user_id: uuid::Uuid,
        config: &ConfiguracionRestaurante,
        client: &BdpWeblinkClient<'_>,
        en_vuelo: Vec<crate::models::Venta>,
        ventas: &[crate::models::Venta],
    ) -> (usize, bool) {
        let en_vuelo: Vec<_> = en_vuelo
            .into_iter()
            .filter(|venta| !ventas.iter().any(|fresca| fresca.id == venta.id))
            .collect();
        if en_vuelo.is_empty() {
            return (0, false);
        }
        let resueltas =
            BdpSyncService::recuperar_lista_en_vuelo(pool, user_id, config, client, en_vuelo).await;
        (resueltas, true)
    }

    /* [AUDIT-2.11b] Reconcilia ventas huérfanas consultando GetOrder. Devuelve
     * (reconciliadas, hubo_fallo_bdp) para que poll_pending alimente el conmutador. */
    async fn reconciliar_huerfanas(
        pool: &PgPool,
        client: &BdpWeblinkClient<'_>,
        huerfanas: &[crate::models::Venta],
    ) -> (usize, bool) {
        warn!(
            "[AUDIT-2.11b] {} ventas huérfanas detectadas (bdp_synced=false con bdp_order_id). \
             Consultando BDP para reconciliar.",
            huerfanas.len()
        );
        let mut updated = 0;
        let mut fallo_bdp = false;
        for venta in huerfanas {
            match Self::check_order_status(client, venta.bdp_order_id.unwrap_or(0)).await {
                Ok(status) => {
                    /* La comanda existe en BDP → reconciliar Glory */
                    info!(
                        "[AUDIT-2.11b] Venta {} reconciliada: comanda existe en BDP (status={status}). \
                         Marcando bdp_synced=true.",
                        venta.id
                    );
                    /* [D10] No descartar errores: un fallo persistente de BD
                     * haría que el poller repita reconciliaciones infinitamente. */
                    if let Err(e) = VentaRepository::update_bdp_status(
                        pool,
                        venta.id,
                        true,
                        None,
                        venta.bdp_order_id,
                    )
                    .await
                    {
                        warn!(
                            "[D10] Error actualizando bdp_status de venta huérfana {}: {e}",
                            venta.id
                        );
                        continue;
                    }
                    if let Err(e) =
                        VentaRepository::update_bdp_order_status(pool, venta.id, &status).await
                    {
                        warn!(
                            "[D10] Error actualizando order_status de venta huérfana {}: {e}",
                            venta.id
                        );
                    }
                    updated += 1;
                }
                Err(e) => {
                    fallo_bdp = true;
                    /* La comanda no existe o BDP no responde → marcar error
                     * para que no se reintente infinitamente */
                    warn!("[AUDIT-2.11b] Venta {} no reconciliable: {e}", venta.id);
                    /* [D10] Propagar errores de BD en vez de descartarlos. */
                    if let Err(e) = VentaRepository::update_bdp_status(
                        pool,
                        venta.id,
                        false,
                        Some("No se pudo verificar existencia en BDP; reconciliación manual requerida"),
                        venta.bdp_order_id,
                    )
                    .await
                    {
                        warn!("[D10] Error marcando venta huérfana {} como no reconciliable: {e}", venta.id);
                    }
                }
            }
        }
        (updated, fallo_bdp)
    }

    /* [AUDIT-N2] Clientes con bdp_synced=true y auditoría pendiente: el alta ya
     * fue exitosa en BDP, así que se cierra la auditoría. Devuelve cuántas cerró. */
    async fn cerrar_auditorias_clientes(
        pool: &PgPool,
        user_id: uuid::Uuid,
        clientes: &[crate::models::Cliente],
    ) -> usize {
        if clientes.is_empty() {
            return 0;
        }
        info!(
            "[AUDIT-N2] {} clientes huérfanos detectados (bdp_synced=true con auditoría pendiente). \
             Cerrando auditoría.",
            clientes.len()
        );
        let mut cerradas = 0;
        for cliente in clientes {
            if let Some(bdp_code) = cliente.bdp_customer_code {
                /* El cliente ya tiene bdp_synced=true → la operación fue exitosa.
                 * Cerrar todas las auditorías pendientes para este cliente. */
                let audit_update = sqlx::query(
                    r"UPDATE bdp_audit_log
                    SET resultado = 'exito', error_mensaje = 'reconciliado por polling N2', updated_at = NOW()
                    WHERE user_id = $1
                      AND target_entity_type = 'cliente'
                      AND target_entity_id = $2
                      AND operacion = 'create_customer'
                      AND resultado IN ('pendiente', 'ambiguo')",
                )
                .bind(user_id)
                .bind(cliente.id)
                .execute(pool)
                .await;
                match audit_update {
                    Ok(result) if result.rows_affected() > 0 => {
                        cerradas += 1;
                        info!(
                            "[AUDIT-N2] Cliente {} (code={bdp_code}) reconciliado: auditoría cerrada.",
                            cliente.id
                        );
                    }
                    Ok(_) => warn!(
                        "[AUDIT-N2] Cliente {} no tenía auditorías pendientes que cerrar.",
                        cliente.id
                    ),
                    Err(error) => warn!(
                        "[AUDIT-N2] No se pudo cerrar la auditoría del cliente {}: {error}",
                        cliente.id
                    ),
                }
            }
        }
        cerradas
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
        let resp = client
            .get_order(&BdpGetOrderRequest {
                order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            })
            .await
            .map_err(|e| format!("Error BDP GetOrder: {e}"))?;
        let status = Self::map_status(Self::parse_status(&resp)?);
        Self::vigilar_conflicto(pool, venta, order_id, &status, &resp).await;
        VentaRepository::update_bdp_order_status(pool, venta.id, &status)
            .await
            .map_err(|e| format!("Error actualizando estado local BDP: {e}"))?;
        info!("Venta {} → bdp_order_status = {status}", venta.id);
        Ok(true)
    }

    /* [02AB-2] Compara el total que BDP tiene con el que la web conoce: si el TPV
     * cambió la comanda lo deja visible en la venta, sin corregir nada. Solo en
     * comandas abiertas y cuando GetOrder trae el pedido completo. */
    async fn vigilar_conflicto(
        pool: &PgPool,
        venta: &crate::models::Venta,
        order_id: i64,
        status: &str,
        resp: &serde_json::Value,
    ) {
        /* [21AB-3] Una comanda cerrada ya no se compara: se limpia el aviso que
         * hubiera. Antes se salía sin limpiarlo y un conflicto transitorio se
         * quedaba pegado para siempre en una venta cobrada y facturada. */
        if matches!(status, "cancelled" | "invoiced") {
            let _ = VentaRepository::marcar_bdp_conflicto(pool, venta.id, None).await;
            return;
        }
        let Some(total_bdp) = Self::total_remoto(resp) else {
            return;
        };
        let lineas = VentaLineaRepository::listar_por_venta(pool, venta.id)
            .await
            .unwrap_or_default();
        let total_web =
            BdpSyncService::total_pedido(venta, (!lineas.is_empty()).then_some(&lineas[..]));
        let mensaje = crate::services::mensaje_conflicto(order_id, Some(total_bdp), total_web);
        if let Err(e) =
            VentaRepository::marcar_bdp_conflicto(pool, venta.id, mensaje.as_deref()).await
        {
            warn!(
                "[02AB-2] No se pudo anotar el conflicto de la venta {}: {e}",
                venta.id
            );
        }
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

    /* pub(crate): los tests del módulo raíz verifican el parseo y el mapeo, y el
     * worker de avisos lo reutiliza para copiar el estado a la venta. */
    pub(crate) fn parse_status(resp: &serde_json::Value) -> Result<i64, String> {
        let status_value = resp
            .get("Status")
            .or_else(|| resp.get("Order").and_then(|order| order.get("Status")))
            .ok_or_else(|| "Respuesta BDP sin campo Status".to_string())?;
        status_value
            .as_i64()
            .or_else(|| status_value.as_str()?.trim().parse::<i64>().ok())
            .ok_or_else(|| "Respuesta BDP contiene Status inválido".to_string())
    }

    /* [21AB-3] Total con el que comparar el conflicto, o `None` si no hay nada
     * que comparar. `Total` 0 o ausente no es un cambio hecho en el TPV: BDP
     * devuelve la comanda a medias mientras la crea (21/11/2026, venta
     * a13ce0f1: aviso «total en BDP 0.00» a los dos segundos de crearla,
     * comanda correcta y facturada después). Pura para poder probarla sin TPV
     * ni base de datos. */
    pub(crate) fn total_remoto(resp: &serde_json::Value) -> Option<f64> {
        resp.get("Order")
            .and_then(|o| o.get("Total"))
            .and_then(serde_json::Value::as_f64)
            .filter(|total| *total > 0.0)
    }

    /// Mapea el integer de status BDP → string legible almacenado en `bdp_order_status`.
    pub(crate) fn map_status(code: i64) -> String {
        match code {
            0 => "pending".to_string(),
            1 => "accepted".to_string(),
            2 => "cancelled".to_string(),
            3 => "invoiced".to_string(),
            other => {
                warn!("[R7] BDP devolvió status desconocido: {other}. Se almacena como unknown_{other}.");
                format!("unknown_{other}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BdpOrderPollerService;

    /* [21AB-3] Solo se compara un total real: la comanda que BDP devuelve a
     * medias (Total 0 o sin Total) no es un cambio del TPV. */
    #[test]
    fn sin_total_no_hay_nada_que_comparar() {
        let sin_total = serde_json::json!({"Status": 1, "Order": {"OrderId": 7}});
        assert_eq!(BdpOrderPollerService::total_remoto(&sin_total), None);

        let cero = serde_json::json!({"Status": 1, "Order": {"OrderId": 7, "Total": 0.0}});
        assert_eq!(BdpOrderPollerService::total_remoto(&cero), None);

        /* Sin objeto Order no se puede afirmar nada. */
        let sin_orden = serde_json::json!({"Status": 3, "Order": null});
        assert_eq!(BdpOrderPollerService::total_remoto(&sin_orden), None);

        let con_total = serde_json::json!({"Status": 1, "Order": {"OrderId": 7, "Total": 5.7}});
        assert_eq!(BdpOrderPollerService::total_remoto(&con_total), Some(5.7));
    }
}
