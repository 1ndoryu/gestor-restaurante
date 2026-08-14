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

use rust_decimal::prelude::ToPrimitive;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::models::ConfiguracionRestaurante;
use crate::repositories::VentaRepository;
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{BdpGetOrderRequest, BdpOrderIdentifier};
use crate::services::{ModoEfectivo, ServicioModoOperacion};

pub struct BdpOrderPollerService;

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
             WHERE bdp_poll_enabled = TRUE AND bdp_sync_enabled = TRUE \
               AND bdp_base_url <> '' AND modo_operacion <> 'standalone' \
               ORDER BY user_id LIMIT 100",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error listando configuraciones BDP para polling: {e}"))?;

        let mut total = 0;
        for config in configs {
            /* [128A-1/F1-1/F1-2] M1+M2: el modo efectivo (switch maestro y
             * degradación reactiva) decide si este usuario puede hacer polling;
             * en standalone no se reclama turno ni se llama a BDP. */
            if servicio.modo_efectivo_sin_red(&config) != ModoEfectivo::Bdp {
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
                match Self::poll_pending(pool, config.user_id, &config, servicio).await {
                    Ok(updated) => total += updated,
                    Err(error) => warn!("Polling BDP usuario {}: {error}", config.user_id),
                }
            }
        }
        Ok(total)
    }

    /// Consulta BDP para todas las ventas pendientes de este usuario.
    /// Retorna el número de ventas actualizadas.
    #[allow(clippy::too_many_lines)]
    pub async fn poll_pending(
        pool: &PgPool,
        user_id: uuid::Uuid,
        config: &ConfiguracionRestaurante,
        servicio: &ServicioModoOperacion,
    ) -> Result<usize, String> {
        /* [128A-1/F1-1] M1: standalone nunca llama a BDP. [F1-2] M2: si el
         * modo degradó por fallos consecutivos, el poller también se detiene. */
        if !config.bdp_sync_enabled
            || servicio.modo_efectivo_sin_red(config) != ModoEfectivo::Bdp
        {
            return Ok(0);
        }

        let mut llamo_bdp = false;
        let mut fallo_bdp = false;

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

        if ventas.is_empty() && orphaned.is_empty() && orphaned_customers.is_empty() {
            return Ok(0);
        }

        let client = BdpWeblinkClient::new(config);

        /* [R1] Reconciliar auditorías ambiguas antes de procesar estados normales. */
        match Self::reconcile_ambiguous(pool, user_id, config, &client).await {
            Ok(count) => {
                if count > 0 {
                    info!(
                        "[R1] {} auditorías ambiguas reconciliadas para usuario {}",
                        count, user_id
                    );
                }
            }
            Err(error) => {
                llamo_bdp = true;
                fallo_bdp = true;
                warn!("[R1] Error reconciliando auditorías ambiguas: {error}");
            }
        }

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
                        llamo_bdp = true;
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
                        llamo_bdp = true;
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
        }

        /* [AUDIT-N2] Reconciliar clientes huérfanos: cerrar auditoría pendiente
         * ya que el cliente fue creado exitosamente en BDP (bdp_synced=true). */
        if !orphaned_customers.is_empty() {
            info!(
                "[AUDIT-N2] {} clientes huérfanos detectados (bdp_synced=true con auditoría pendiente). \
                 Cerrando auditoría.",
                orphaned_customers.len()
            );
            for cliente in &orphaned_customers {
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
                            updated += 1;
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
        }

        if !ventas.is_empty() {
            info!("[276A-4.2] Polling BDP: {} ventas pendientes", ventas.len());
            for venta in &ventas {
                match Self::poll_one(pool, venta, config, Some(&client)).await {
                    Ok(true) => {
                        llamo_bdp = true;
                        updated += 1;
                    }
                    Ok(false) => {
                        llamo_bdp = true;
                    }
                    Err(e) => {
                        llamo_bdp = true;
                        fallo_bdp = true;
                        warn!(
                            "[276A-4.2] Error consultando GetOrder para venta {}: {e}",
                            venta.id
                        );
                    }
                }
            }
        }

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
            other => {
                warn!("[R7] BDP devolvió status desconocido: {other}. Se almacena como unknown_{other}.");
                format!("unknown_{other}")
            }
        }
    }

    /* [R1] Reconciliar auditorías BDP marcadas como ambiguas.
     * [247A-10/P3] También reconcilia pagos ambiguos del ledger local.
     * Devuelve el número de auditorías/pagos cerrados como exito. */
    async fn reconcile_ambiguous(
        pool: &PgPool,
        user_id: uuid::Uuid,
        config: &ConfiguracionRestaurante,
        client: &BdpWeblinkClient<'_>,
    ) -> Result<usize, String> {
        let rows: Vec<(uuid::Uuid, String, uuid::Uuid, serde_json::Value)> = sqlx::query_as(
            r"SELECT id, operacion, target_entity_id, datos_enviados
               FROM bdp_audit_log
               WHERE user_id = $1
                 AND resultado = 'ambiguo'
                 AND operacion IN ('create_order', 'add_payment', 'invoice')
               ORDER BY created_at DESC
               LIMIT 100",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error listando auditorías ambiguas: {e}"))?;

        let mut reconciled = 0;
        for (audit_id, operacion, target_entity_id, datos_enviados) in rows {
            let result = match operacion.as_str() {
                "create_order" => {
                    Self::reconcile_create_order(pool, client, audit_id, target_entity_id).await
                }
                "add_payment" => {
                    Self::reconcile_add_payment(
                        pool,
                        client,
                        audit_id,
                        target_entity_id,
                        &datos_enviados,
                    )
                    .await
                }
                "invoice" => {
                    Self::reconcile_invoice(pool, client, audit_id, target_entity_id).await
                }
                _ => Ok(false),
            };
            match result {
                Ok(true) => {
                    info!("[R1] Auditoría {audit_id} reconciliada para {operacion}");
                    reconciled += 1;
                }
                Ok(false) => {}
                Err(e) => {
                    warn!("[R1] Error reconciliando auditoría {audit_id}: {e}");
                }
            }
        }

        /* [247A-10/P3] También reconciliar pagos ambiguos del ledger local. */
        reconciled += Self::reconcile_ambiguous_pagos(pool, user_id, config, client).await?;

        Ok(reconciled)
    }

    /* [247A-10/P3] Reconciliar pagos ambiguos del ledger bdp_pagos.
     * Devuelve el número de pagos reconciliados. */
    async fn reconcile_ambiguous_pagos(
        pool: &PgPool,
        user_id: uuid::Uuid,
        _config: &ConfiguracionRestaurante,
        client: &BdpWeblinkClient<'_>,
    ) -> Result<usize, String> {
        use crate::repositories::BdpPagoRepository;
        let pagos = BdpPagoRepository::listar_ambiguos(pool, user_id)
            .await
            .map_err(|e| format!("Error listando pagos ambiguos: {e}"))?;

        let mut reconciled = 0;
        for pago in pagos {
            let Some(order_id) = Self::find_bdp_order_id_for_venta(pool, pago.venta_id).await?
            else {
                continue;
            };
            let request = BdpGetOrderRequest {
                order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            };
            match client.get_order(&request).await {
                Ok(response) => {
                    let order = response.get("Order").cloned().unwrap_or(response);
                    let payments = order
                        .get("Payments")
                        .and_then(serde_json::Value::as_array)
                        .cloned()
                        .unwrap_or_default();
                    let expected_amount =
                        rust_decimal::Decimal::to_f64(&pago.amount).unwrap_or(0.0);
                    let expected_tender = i64::from(pago.tender_id);
                    let mut matched: Option<String> = None;
                    for payment in payments {
                        let tender = payment
                            .get("TenderId")
                            .and_then(serde_json::Value::as_i64)
                            .unwrap_or(-1);
                        /* [D1] No usar unwrap_or(0.0) en montos financieros.
                         * Si BDP devuelve Amount como string o null, no hacer match
                         * con 0.0 — eso podría reconciliar un pago fantasma. */
                        let Some(amount) =
                            payment.get("Amount").and_then(serde_json::Value::as_f64)
                        else {
                            continue;
                        };
                        if tender == expected_tender && (amount - expected_amount).abs() < 0.005 {
                            matched = payment
                                .get("PaymentId")
                                .and_then(serde_json::Value::as_str)
                                .map(String::from);
                            break;
                        }
                    }
                    if let Some(payment_id) = matched {
                        let datos =
                            serde_json::json!({ "order_id": order_id, "payment_id": payment_id });
                        if let Err(e) = BdpPagoRepository::reconciliar_exito(
                            pool,
                            pago.id,
                            Some(&payment_id),
                            Some(&datos),
                        )
                        .await
                        {
                            warn!("[247A-10/P3] Error cerrando pago ambiguo {}: {e}", pago.id);
                        } else {
                            info!("[247A-10/P3] Pago ambiguo {} reconciliado (PaymentId={payment_id})", pago.id);
                            reconciled += 1;
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "[247A-10/P3] GetOrder falló para reconciliar pago {}: {e}",
                        pago.id
                    );
                }
            }
        }
        Ok(reconciled)
    }

    async fn reconcile_create_order(
        pool: &PgPool,
        client: &BdpWeblinkClient<'_>,
        audit_id: uuid::Uuid,
        venta_id: uuid::Uuid,
    ) -> Result<bool, String> {
        let marketplace_id =
            crate::services::bdp_sync::BdpSyncService::marketplace_order_id(venta_id);
        let request = BdpGetOrderRequest {
            order_identifier: BdpOrderIdentifier::by_market(
                crate::services::bdp_sync::BDP_SYNC_MARKET_ID,
                marketplace_id.clone(),
            ),
        };
        match client.get_order(&request).await {
            Ok(response) => {
                let order_id = response
                    .get("OrderId")
                    .and_then(serde_json::Value::as_i64)
                    .or_else(|| response.get("Order")?.get("OrderId")?.as_i64())
                    .filter(|id| *id > 0);
                if let Some(order_id) = order_id {
                    let respuesta = serde_json::json!({ "order_id": order_id });
                    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
                    sqlx::query(
                        "UPDATE ventas SET bdp_synced = true, bdp_synced_at = NOW(), bdp_order_id = $2, bdp_sync_error = NULL WHERE id = $1"
                    )
                    .bind(venta_id)
                    .bind(order_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
                    sqlx::query(
                        r"UPDATE bdp_audit_log
                        SET resultado = 'exito', datos_respuesta = $2, error_mensaje = NULL, updated_at = NOW()
                        WHERE id = $1"
                    )
                    .bind(audit_id)
                    .bind(Some(&respuesta))
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
                    tx.commit().await.map_err(|e| e.to_string())?;
                    info!("[R1] Comanda reconciliada para venta {venta_id} → OrderId={order_id}");
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Err(e) => {
                warn!("[R1] GetOrder falló para reconciliar comanda {venta_id}: {e}");
                Ok(false)
            }
        }
    }

    async fn reconcile_add_payment(
        pool: &PgPool,
        client: &BdpWeblinkClient<'_>,
        audit_id: uuid::Uuid,
        venta_id: uuid::Uuid,
        datos_enviados: &serde_json::Value,
    ) -> Result<bool, String> {
        let Some(order_id) = Self::find_bdp_order_id_for_venta(pool, venta_id).await? else {
            return Ok(false);
        };
        let request = BdpGetOrderRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
        };
        match client.get_order(&request).await {
            Ok(response) => {
                let order = response.get("Order").cloned().unwrap_or(response);
                let payments = order
                    .get("Payments")
                    .and_then(serde_json::Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let Some(expected_tender) = datos_enviados.get("tender_id").and_then(json_i64)
                else {
                    warn!("[R1] Pago {venta_id} sin tender_id verificable; se mantiene ambiguo");
                    return Ok(false);
                };
                let Some(expected_amount) = datos_enviados.get("amount").and_then(json_f64) else {
                    warn!("[R1] Pago {venta_id} sin amount verificable; se mantiene ambiguo");
                    return Ok(false);
                };
                /* [D1+D5] No usar unwrap_or(0.0) en montos de BDP.
                 * Verificar PaymentId para evitar falsos positivos cuando
                 * hay múltiples pagos con mismo tender y monto similar. */
                let Some(expected_payment_id) = datos_enviados
                    .get("idempotency_key")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                else {
                    warn!("[R1] Pago {venta_id} sin idempotency_key; se mantiene ambiguo");
                    return Ok(false);
                };
                let matched = payments.iter().any(|payment| {
                    let tender = payment.get("TenderId").and_then(json_i64);
                    let amount = payment.get("Amount").and_then(json_f64);
                    let payment_id = payment.get("PaymentId").and_then(serde_json::Value::as_str);
                    let (Some(tender), Some(amount), Some(payment_id)) =
                        (tender, amount, payment_id)
                    else {
                        return false;
                    };
                    if tender != expected_tender || (amount - expected_amount).abs() >= 0.005 {
                        return false;
                    }
                    /* [D5/287A-4] Sin PaymentId no hay evidencia suficiente:
                     * tender+monto pueden coincidir con otro pago legítimo. */
                    payment_id.ends_with(expected_payment_id)
                });
                if !matched {
                    return Ok(false);
                }
                let invoice_number = order
                    .get("InvoiceNumber")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
                /* [D11] Solo marcar como facturado si NO lo estaba ya localmente.
                 * Evita sobreescribir datos de factura tras una corrección manual. */
                if invoice_number.is_some() {
                    sqlx::query(
                        "UPDATE ventas SET bdp_invoiced = true, bdp_order_status = 'invoiced', updated_at = NOW() WHERE id = $1 AND bdp_invoiced = FALSE"
                    )
                    .bind(venta_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
                }
                let respuesta =
                    serde_json::json!({ "order_id": order_id, "invoice_number": invoice_number });
                sqlx::query(
                    r"UPDATE bdp_audit_log
                    SET resultado = 'exito', datos_respuesta = $2, error_mensaje = NULL, updated_at = NOW()
                    WHERE id = $1"
                )
                .bind(audit_id)
                .bind(Some(&respuesta))
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
                tx.commit().await.map_err(|e| e.to_string())?;
                info!("[R1] Pago reconciliado para venta {venta_id} → OrderId={order_id}");
                Ok(true)
            }
            Err(e) => {
                warn!("[R1] GetOrder falló para reconciliar pago {venta_id}: {e}");
                Ok(false)
            }
        }
    }

    async fn reconcile_invoice(
        pool: &PgPool,
        client: &BdpWeblinkClient<'_>,
        audit_id: uuid::Uuid,
        venta_id: uuid::Uuid,
    ) -> Result<bool, String> {
        let Some(order_id) = Self::find_bdp_order_id_for_venta(pool, venta_id).await? else {
            return Ok(false);
        };
        let request = BdpGetOrderRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
        };
        match client.get_order(&request).await {
            Ok(response) => {
                let order = response.get("Order").cloned().unwrap_or(response);
                let status = order
                    .get("Status")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(-1);
                let invoice_number = order
                    .get("InvoiceNumber")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                if status != 3 && invoice_number.is_none() {
                    return Ok(false);
                }
                let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
                sqlx::query(
                    "UPDATE ventas SET bdp_invoiced = true, bdp_order_status = 'invoiced', updated_at = NOW() WHERE id = $1"
                )
                .bind(venta_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
                let respuesta =
                    serde_json::json!({ "order_id": order_id, "invoice_number": invoice_number });
                sqlx::query(
                    r"UPDATE bdp_audit_log
                    SET resultado = 'exito', datos_respuesta = $2, error_mensaje = NULL, updated_at = NOW()
                    WHERE id = $1"
                )
                .bind(audit_id)
                .bind(Some(&respuesta))
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
                tx.commit().await.map_err(|e| e.to_string())?;
                info!("[R1] Factura reconciliada para venta {venta_id} → OrderId={order_id}");
                Ok(true)
            }
            Err(e) => {
                warn!("[R1] GetOrder falló para reconciliar factura {venta_id}: {e}");
                Ok(false)
            }
        }
    }

    async fn find_bdp_order_id_for_venta(
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

fn json_f64(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str()?.trim().parse::<f64>().ok())
        .filter(|number| number.is_finite())
}

fn json_i64(value: &serde_json::Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str()?.trim().parse::<i64>().ok())
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

    #[test]
    fn numeric_evidence_accepts_number_or_string_but_rejects_invalid_values() {
        assert_eq!(json_f64(&serde_json::json!("50.50")), Some(50.5));
        assert_eq!(json_f64(&serde_json::json!(50.5)), Some(50.5));
        assert_eq!(json_f64(&serde_json::json!("NaN")), None);
        assert_eq!(json_f64(&serde_json::Value::Null), None);
        assert_eq!(json_i64(&serde_json::json!("7")), Some(7));
        assert_eq!(json_i64(&serde_json::json!(7)), Some(7));
        assert_eq!(json_i64(&serde_json::json!("7.2")), None);
    }
}
