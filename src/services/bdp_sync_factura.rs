// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [F4.2] Submodulo de facturacion BDP (InvoiceOrder). Codigo movido byte-identico desde bdp_sync.rs;
 * Self::conectar_bdp vive en el hub como pub(crate). */

use rust_decimal::prelude::Decimal;
use serde_json::Value;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::models::{ConfiguracionRestaurante, Venta};
use crate::repositories::BdpPagoRepository;
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{
    BdpGetOrderRequest, BdpInvoiceOrderRequest, BdpOrderIdentifier,
};
use crate::services::{ModoEfectivo, ServicioModoOperacion};

use super::bdp_sync::BdpSyncService;

impl BdpSyncService {
    /* [F3.2] Puerta de facturación: switch maestro, modo escritura y precondiciones. */
    fn validar_puerta_factura(
        venta: &Venta,
        config: &ConfiguracionRestaurante,
    ) -> Result<i64, String> {
        /* [128A-1/F1-1] M1: el switch maestro gatea también la facturación. */
        if ServicioModoOperacion::modo_efectivo_desde_config(config) != ModoEfectivo::Bdp {
            return Err("BDP no está habilitado o configurado".into());
        }

        /* [F3] Gate: en modo read_only, no facturar en BDP */
        if !crate::services::BdpWriteGuard::modo_permite_escritura(&config.bdp_sync_mode) {
            return Err(
                "BDP en modo solo lectura. Cambia el modo en configuración para facturar.".into(),
            );
        }
        if !config.bdp_auto_backup_before_write {
            return Err("Escritura BDP bloqueada: auto-backup pre-write desactivado".into());
        }

        venta
            .bdp_order_id
            .ok_or_else(|| format!("Venta {} no tiene bdp_order_id", venta.id))
    }
    /* [F3.2] Reconcilia la orden remota antes de escribir: parseo estricto
     * del objeto Order/Status; una orden cancelada bloquea. */
    async fn reconciliar_orden_factura(
        client: &BdpWeblinkClient<'_>,
        order_id: i64,
    ) -> Result<Value, String> {
        let current = client
            .get_order(&BdpGetOrderRequest {
                order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            })
            .await
            .map_err(|e| {
                format!("Factura bloqueada: no se pudo reconciliar la orden antes de escribir: {e}")
            })?;
        let order = current
            .get("Order")
            .ok_or_else(|| "Factura bloqueada: GetOrder no devolvió el objeto Order".to_string())?;
        let status = order
            .get("Status")
            .and_then(Value::as_i64)
            .ok_or_else(|| "Factura bloqueada: GetOrder no devolvió Status".to_string())?;
        if status == 2 {
            return Err("Factura bloqueada: la orden está cancelada".into());
        }
        Ok(order.clone())
    }

    /* [F3.2] Si la orden ya está facturada (Status 3), reconcilia el ledger
     * local en transacción y devuelve el InvoiceNumber. `None` = continuar. */
    async fn conciliar_factura_emitida(
        pool: &PgPool,
        venta: &Venta,
        order: &Value,
    ) -> Result<Option<String>, String> {
        let status = order
            .get("Status")
            .and_then(Value::as_i64)
            .ok_or_else(|| "Factura bloqueada: GetOrder no devolvió Status".to_string())?;
        if status != 3 {
            return Ok(None);
        }
        let invoice_number = order
            .get("InvoiceNumber")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| "Orden ya facturada pero sin InvoiceNumber reconciliable".to_string())?;
        /* [AUDIT-N6] Envolver reconciliación en transacción por consistencia
         * con el path normal de facturación. */
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| format!("Error iniciando tx reconciliación factura: {e}"))?;
        sqlx::query(
            "UPDATE ventas SET bdp_invoiced = true, bdp_order_status = 'invoiced', updated_at = NOW() WHERE id = $1",
        )
        .bind(venta.id)
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("Factura BDP reconciliada, pero no se pudo persistir localmente: {error}"))?;
        tx.commit()
            .await
            .map_err(|e| format!("Error confirmando tx reconciliación factura: {e}"))?;
        Ok(Some(invoice_number))
    }

    /* [F3.2] Verifica que no quede saldo pendiente ni local ni en BDP.
     * [028A-BDP-WRITE] Parseo estricto: un importe remoto malformado
     * bloquea, nunca se interpreta como cero. */
    async fn verificar_saldo_facturable(
        pool: &PgPool,
        venta: &Venta,
        order: &Value,
    ) -> Result<(), String> {
        let total_decimal = Self::parse_remote_money(
            order
                .get("Total")
                .ok_or_else(|| "Factura bloqueada: GetOrder no devolvió Total".to_string())?,
            "Total",
        )?;
        let paid: Decimal = order
            .get("Payments")
            .and_then(Value::as_array)
            .ok_or_else(|| "Factura bloqueada: GetOrder no devolvió Payments".to_string())?
            .iter()
            .try_fold(Decimal::ZERO, |sum, payment| {
                let amount = Self::parse_remote_money(
                    payment.get("Amount").ok_or_else(|| {
                        "Factura bloqueada: un pago remoto no contiene Amount".to_string()
                    })?,
                    "Payments.Amount",
                )?;
                Ok::<Decimal, String>(sum + amount)
            })?;
        let local_paid = BdpPagoRepository::total_pagado(pool, venta.id)
            .await
            .map_err(|e| format!("Factura bloqueada: no se pudo calcular saldo local: {e}"))?;
        let tolerance = Decimal::new(5, 3); /* 0.005 */
        if (total_decimal - local_paid).abs() > tolerance {
            return Err("Factura bloqueada: la orden conserva saldo pendiente local".into());
        }
        let bdp_pending = total_decimal - paid;
        if bdp_pending.abs() > tolerance {
            return Err("Factura bloqueada: la orden conserva saldo pendiente en BDP".into());
        }
        Ok(())
    }

    /* [F3.2] Snapshot obligatorio + autorización de un solo uso [187A-1]. */
    async fn autorizar_factura(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
        order_id: i64,
        idempotency_key: Option<&str>,
    ) -> Result<Uuid, String> {
        let datos_factura = serde_json::json!({
            "venta_id": venta.id,
            "order_id": order_id
        });
        let snapshot_pre_id = crate::services::BdpBackupService::preparar_snapshot_escritura(
            pool,
            venta.user_id,
            "invoice",
            config,
            Some(order_id),
        )
        .await
        .map_err(|e| format!("Pre-write audit BDP falló; facturación bloqueada: {e}"))?;

        crate::services::BdpWriteGuard::authorize(
            pool,
            venta.user_id,
            config,
            "invoice",
            "venta",
            venta.id,
            "venta_id",
            &datos_factura,
            snapshot_pre_id,
            idempotency_key,
        )
        .await
    }

    /* [F3.2] Ejecuta `InvoiceOrder` y cierra la auditoría en error/ambiguo.
     * Devuelve la respuesta cruda y el InvoiceNumber ya validado. */
    async fn ejecutar_factura(
        client: &BdpWeblinkClient<'_>,
        config: &ConfiguracionRestaurante,
        order_id: i64,
        pool: &PgPool,
        audit_id: Uuid,
    ) -> Result<(Value, String), String> {
        let request = BdpInvoiceOrderRequest {
            pos_id: config.bdp_pos_id,
            employee_id: config.bdp_employee_id,
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            invoice_parameters: None,
        };

        let response = match client.invoice_order(&request).await {
            Ok(response) => response,
            Err(e) => {
                let msg = format!("Error InvoiceOrder: {e}");
                let resultado = match e {
                    crate::services::bdp_weblink::BdpWeblinkError::Http(_)
                    | crate::services::bdp_weblink::BdpWeblinkError::Api { .. }
                    | crate::services::bdp_weblink::BdpWeblinkError::Throttled(_) => "ambiguo",
                    _ => "error",
                };
                crate::services::BdpBackupService::actualizar_resultado(
                    pool,
                    audit_id,
                    resultado,
                    None,
                    Some(&msg),
                )
                .await
                .map_err(|audit_error| {
                    format!("{msg}; además falló el cierre de auditoría: {audit_error}")
                })?;
                return Err(msg);
            }
        };

        let invoice_number = response
            .get("InvoiceNumber")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if invoice_number.is_empty() {
            let msg = "BDP no devolvió InvoiceNumber; no se marcará la venta como facturada";
            crate::services::BdpBackupService::actualizar_resultado(
                pool,
                audit_id,
                "ambiguo",
                Some(&response),
                Some(msg),
            )
            .await
            .map_err(|audit_error| {
                format!("{msg}; además falló el cierre de auditoría: {audit_error}")
            })?;
            return Err(msg.to_string());
        }
        Ok((response, invoice_number))
    }

    /* [F3.2] Marca local + cierre de auditoría en una sola transacción
     * ([207A-2] S7-H2). Si la tx falla, BDP ya facturó → auditoría ambigua. */
    async fn confirmar_factura(
        pool: &PgPool,
        venta: &Venta,
        audit_id: Uuid,
        response: &Value,
        invoice_number: &str,
    ) -> Result<(), String> {
        let commit_result = async {
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| format!("Error iniciando tx post-factura: {e}"))?;

            /* [F8.3] Marcar venta como facturada. */
            sqlx::query(
                "UPDATE ventas SET bdp_invoiced = true, bdp_order_status = 'invoiced', updated_at = NOW() WHERE id = $1",
            )
            .bind(venta.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                format!("BDP confirmó InvoiceNumber={invoice_number}, pero no se pudo persistir localmente: {e}")
            })?;

            /* Cerrar auditoría dentro de la misma transacción. */
            sqlx::query(
                r"UPDATE bdp_audit_log
                SET resultado = 'exito', datos_respuesta = $2, error_mensaje = NULL, updated_at = NOW()
                WHERE id = $1",
            )
            .bind(audit_id)
            .bind(Some(response))
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Factura confirmada, pero falló el cierre de auditoría: {e}"))?;

            tx.commit()
                .await
                .map_err(|e| format!("Error confirmando tx post-factura: {e}"))
        }
        .await;

        if let Err(e) = commit_result {
            /* La tx falló pero BDP ya procesó la factura → auditoría ambigua. */
            let _ = crate::services::BdpBackupService::actualizar_resultado(
                pool,
                audit_id,
                "ambiguo",
                Some(response),
                Some(&e),
            )
            .await;
            return Err(e);
        }
        Ok(())
    }
    /* [F8.2] Facturar una orden BDP existente.
     * Llama a `POST /API/Orders/Invoice` con el order_id de la venta.
     * Retorna el InvoiceNumber.
     *
     * ⚠️ REQUIERE AUTORIZACIÓN DEL USUARIO para llamadas reales a BDP. */
    /* [187A-1] La secuencia factura/preflight/snapshot/autorización/auditoría
     * se mantiene lineal para conservar una única frontera de escritura. */
    pub async fn invoice_order(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
        idempotency_key: Option<&str>,
    ) -> Result<String, String> {
        let order_id = Self::validar_puerta_factura(venta, config)?;

        let client = Self::conectar_bdp(config).await?;

        crate::services::BdpWriteGuard::ensure_no_unresolved(
            pool,
            venta.user_id,
            "venta_id",
            venta.id,
            &["invoice"],
        )
        .await?;

        let order = Self::reconciliar_orden_factura(&client, order_id).await?;
        if let Some(invoice_number) = Self::conciliar_factura_emitida(pool, venta, &order).await? {
            return Ok(invoice_number);
        }
        Self::verificar_saldo_facturable(pool, venta, &order).await?;

        /* [187A-1] Snapshot obligatorio + autorización de un solo uso. */
        let audit_id =
            Self::autorizar_factura(pool, venta, config, order_id, idempotency_key).await?;

        let (response, invoice_number) =
            Self::ejecutar_factura(&client, config, order_id, pool, audit_id).await?;
        Self::confirmar_factura(pool, venta, audit_id, &response, &invoice_number).await?;

        info!(
            "[F8.2] Orden {} facturada en BDP → InvoiceNumber={invoice_number}",
            venta.id
        );

        Ok(invoice_number)
    }
}
