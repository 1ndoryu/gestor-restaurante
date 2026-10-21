// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
use rust_decimal::prelude::ToPrimitive;
use sqlx::PgPool;
use tracing::{info, warn};

use super::BdpOrderPollerService;
use crate::models::ConfiguracionRestaurante;
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{BdpGetOrderRequest, BdpOrderIdentifier};

/* Reconciliación de auditorías y pagos ambiguos: consulta GetOrder para saber si
 * la operación llegó a BDP aunque Glory no recibiera la confirmación. */
impl BdpOrderPollerService {
    /* [R1] Reconciliar auditorías BDP marcadas como ambiguas.
     * [247A-10/P3] También reconcilia pagos ambiguos del ledger local.
     * Devuelve el número de auditorías/pagos cerrados como exito. */
    pub(super) async fn reconcile_ambiguous(
        pool: &PgPool,
        user_id: uuid::Uuid,
        config: &ConfiguracionRestaurante,
        client: &BdpWeblinkClient<'_>,
    ) -> Result<usize, String> {
        /* [D23b] También las `pendiente`: son las que quedaron abiertas cuando el
         * proceso murió entre el HTTP y el cierre de la auditoría, y nadie las
         * miraba. Preguntar por su `MarketplaceOrderId` es inocuo: si la comanda
         * no existe en BDP, no se toca nada. */
        let rows: Vec<(uuid::Uuid, String, uuid::Uuid, serde_json::Value)> = sqlx::query_as(
            r"SELECT id, operacion, target_entity_id, datos_enviados
               FROM bdp_audit_log
               WHERE user_id = $1
                 AND resultado IN ('ambiguo', 'pendiente')
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
                    /* [D10] La comanda se recuperó con su estado delante: si BDP
                     * la da por cancelada, la venta queda anulada en la web en la
                     * misma transacción. Antes se recuperaba el `OrderId` y el
                     * estado se quedaba sin leer. */
                    if let Ok(status) = Self::parse_status(&response) {
                        crate::repositories::VentaRepository::update_bdp_order_status(
                            &mut *tx,
                            venta_id,
                            &Self::map_status(status),
                        )
                        .await
                        .map_err(|e| e.to_string())?;
                    }
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
                    /* [D20] Con el número: es contra lo que el worker cruza el aviso. */
                    sqlx::query(
                        "UPDATE ventas SET bdp_invoiced = true, bdp_order_status = 'invoiced', bdp_invoice_number = COALESCE(bdp_invoice_number, $2), updated_at = NOW() WHERE id = $1 AND bdp_invoiced = FALSE"
                    )
                    .bind(venta_id)
                    .bind(invoice_number.as_deref())
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
                /* [D20] Con el número (puede faltar: `COALESCE` lo deja como estaba). */
                sqlx::query(
                    "UPDATE ventas SET bdp_invoiced = true, bdp_order_status = 'invoiced', bdp_invoice_number = COALESCE(bdp_invoice_number, $2), updated_at = NOW() WHERE id = $1"
                )
                .bind(venta_id)
                .bind(invoice_number.as_deref())
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

/* Lectura tolerante de números que BDP puede devolver como texto. Los tests de la
 * fachada verifican que rechazan NaN, null y decimales donde se espera entero. */
pub(super) fn json_f64(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str()?.trim().parse::<f64>().ok())
        .filter(|number| number.is_finite())
}

pub(super) fn json_i64(value: &serde_json::Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str()?.trim().parse::<i64>().ok())
}
