// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [F4.2] Submodulo de pagos BDP (AddOrderPayment). Codigo movido byte-identico desde bdp_sync.rs;
 * Self::conectar_bdp y Self::parse_remote_* viven en el hub / bdp_sync_venta.rs como pub(crate). */

use std::sync::Arc;

use rust_decimal::prelude::Decimal;
use serde_json::Value;
use sqlx::PgPool;
use tokio::sync::Mutex as TokioMutex;
use tracing::info;
use uuid::Uuid;

use crate::models::{ConfiguracionRestaurante, Venta};
use crate::repositories::BdpPagoRepository;
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{
    BdpAddOrderPaymentRequest, BdpGetOrderRequest, BdpOrderIdentifier, BdpOrderPayment,
};
use crate::services::{ModoEfectivo, ServicioModoOperacion};

use super::bdp_sync::{BdpSyncService, SyncLockGuard, SYNC_LOCKS};

/* [267A-8] Intención de pago validada: lo necesario para armar y ejecutar
 * el AddOrderPayment sin arrastrar 8 parámetros (límite clippy). */
#[derive(Debug, Clone, Copy)]
struct IntentoPago {
    order_id: i64,
    amount: Decimal,
    tender_id: i32,
    is_partial: bool,
}

/* [267A-8] Pago armado: intención autorizada + request listo para BDP. */
struct PagoArmado {
    audit_id: Uuid,
    intento: IntentoPago,
    payment_id: String,
    request: BdpAddOrderPaymentRequest,
}

impl BdpSyncService {
    /* [F8.1] Registrar pago contra una orden BDP existente.
     * Ahora soporta pagos parciales controlados por el feature flag
     * ff_bdp_partial_payments. Cada pago se registra en el ledger local
     * bdp_pagos para evitar sobrepagos y mantener historial.
     *
     * ️ REQUIERE AUTORIZACIÓN DEL USUARIO para llamadas reales a BDP. */
    pub async fn add_order_payment(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
        amount: Decimal,
        tender_id: i32,
        idempotency_key: Option<&str>,
    ) -> Result<Option<String>, String> {
        let order_id = Self::validar_puerta_pago(venta, config, amount, tender_id)?;

        /* [247A-9] Exclusión por venta para evitar pagos concurrentes que
         * podrían sobrepasar el saldo pendiente. */
        let lock = {
            let mut map = SYNC_LOCKS
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            map.entry(venta.id)
                .or_insert_with(|| Arc::new(TokioMutex::new(())))
                .clone()
        };
        let Ok(_guard) = lock.try_lock() else {
            return Err("Ya hay un pago en proceso para esta venta; inténtalo de nuevo".into());
        };
        let _sync_lock_guard = SyncLockGuard::new(venta.id);

        /* [247A-9] Idempotencia: si la clave ya existe con éxito, no repetir. */
        if let Some(key) = idempotency_key {
            if Self::pago_ya_exitoso(pool, venta, key, amount, tender_id).await? {
                return Ok(None);
            }
        }

        let client = Self::conectar_bdp(config).await?;

        crate::services::BdpWriteGuard::ensure_no_unresolved(
            pool,
            venta.user_id,
            "venta_id",
            venta.id,
            &["add_payment"],
        )
        .await?;

        let is_partial =
            Self::reconciliar_orden_pago(&client, pool, venta, config, order_id, amount).await?;
        let intento = IntentoPago {
            order_id,
            amount,
            tender_id,
            is_partial,
        };
        let armado = Self::armar_pago(pool, venta, config, &intento, idempotency_key).await?;

        Self::ejecutar_pago(&client, pool, venta, &armado, idempotency_key).await
    }

    /* [267A-8] Puerta de pago: modo, read_only, backup, order_id e
     * importe/tender válidos. Devuelve el order_id BDP. */
    fn validar_puerta_pago(
        venta: &Venta,
        config: &ConfiguracionRestaurante,
        amount: Decimal,
        tender_id: i32,
    ) -> Result<i64, String> {
        /* [128A-1/F1-1] M1: el switch maestro gatea también los pagos. */
        if ServicioModoOperacion::modo_efectivo_desde_config(config) != ModoEfectivo::Bdp {
            return Err("BDP no está habilitado o configurado".into());
        }

        /* [F3] Gate: en modo read_only, no registrar pagos en BDP */
        if !crate::services::BdpWriteGuard::modo_permite_escritura(&config.bdp_sync_mode) {
            return Err(
                "BDP en modo solo lectura. Cambia el modo en configuración para registrar pagos."
                    .into(),
            );
        }
        if !config.bdp_auto_backup_before_write {
            return Err("Escritura BDP bloqueada: auto-backup pre-write desactivado".into());
        }

        let order_id = venta
            .bdp_order_id
            .ok_or_else(|| format!("Venta {} no tiene bdp_order_id", venta.id))?;

        if amount <= Decimal::ZERO || tender_id <= 0 {
            return Err("Pago BDP bloqueado: importe o tender inválido".into());
        }
        Ok(order_id)
    }

    /* [267A-8] Idempotencia [247A-9]: la clave debe pertenecer a la misma
     * venta y tener mismos amount/tender_id para considerarse un reintento
     * legítimo; de lo contrario es error de cliente o reuso malicioso.
     * Devuelve `true` si el pago ya fue exitoso (no reenviar). */
    async fn pago_ya_exitoso(
        pool: &PgPool,
        venta: &Venta,
        key: &str,
        amount: Decimal,
        tender_id: i32,
    ) -> Result<bool, String> {
        if let Ok(Some(pago)) = BdpPagoRepository::obtener_por_idempotency_key(pool, key).await {
            if pago.venta_id != venta.id {
                return Err("idempotencia_duplicada:ledger:otra_venta".into());
            }
            if pago.amount != amount || pago.tender_id != tender_id {
                return Err("idempotencia_duplicada:ledger:campos_distintos".into());
            }
            if pago.resultado == "exito" {
                info!(
                    "[247A-9] Pago con idempotencia {} ya existía; no se reenvía a BDP",
                    key
                );
                return Ok(true);
            }
        }
        Ok(false)
    }

    /* [267A-8] Reconciliación pre-pago: GetOrder remoto + unión de libros
     * local/remoto [028A-BDP-WRITE] + validación de parciales. Devuelve si
     * el pago es parcial respecto al saldo pendiente. */
    async fn reconciliar_orden_pago(
        client: &BdpWeblinkClient<'_>,
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
        order_id: i64,
        amount: Decimal,
    ) -> Result<bool, String> {
        let current = client
            .get_order(&BdpGetOrderRequest {
                order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            })
            .await
            .map_err(|e| {
                format!("Pago bloqueado: no se pudo reconciliar la orden antes de escribir: {e}")
            })?;
        let order = current
            .get("Order")
            .ok_or_else(|| "Pago bloqueado: GetOrder no devolvió el objeto Order".to_string())?;
        let status = order
            .get("Status")
            .and_then(Value::as_i64)
            .ok_or_else(|| "Pago bloqueado: GetOrder no devolvió Status".to_string())?;
        if matches!(status, 2 | 3) {
            return Err("Pago bloqueado: la orden está cancelada o facturada".into());
        }
        let total = Self::parse_remote_money(
            order
                .get("Total")
                .ok_or_else(|| "Pago bloqueado: GetOrder no devolvió Total".to_string())?,
            "Total",
        )?;
        let payments = order
            .get("Payments")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                "Pago bloqueado: GetOrder no devolvió la colección Payments".to_string()
            })?;
        /* PaymentId puede llegar como string o número según la versión de
         * WebLink. Conservamos también su importe para detectar respuestas
         * internamente inconsistentes y no duplicar una entrada repetida. */
        let (remote_payment_amounts, remote_paid) = Self::parse_remote_payments(payments)?;
        let requested = amount;

        /* [028A-BDP-WRITE] Calcula la unión de ambos libros. Un pago local con
         * bdp_payment_id coincidente ya está incluido en Payments; uno sin
         * identidad remota no puede demostrarse como el mismo pago, así que se
         * suma conservadoramente para no permitir un posible sobrepago. */
        let local_payments = BdpPagoRepository::listar_por_venta(pool, venta.id)
            .await
            .map_err(|e| format!("Pago bloqueado: no se pudo leer el ledger local: {e}"))?;
        let local_only_paid = local_payments
            .iter()
            .filter(|payment| payment.resultado == "exito")
            .try_fold(Decimal::ZERO, |sum, payment| {
                Self::local_payment_contribution(
                    &remote_payment_amounts,
                    payment.bdp_payment_id.as_deref(),
                    payment.amount,
                )
                .map(|contribution| sum + contribution)
            })?;
        let paid = remote_paid + local_only_paid;
        if paid > total + Decimal::new(5, 3) {
            return Err("Pago bloqueado: la unión de pagos supera el total remoto".into());
        }
        let pending = (total - paid).max(Decimal::ZERO);
        let tolerance = Decimal::new(5, 3);
        let is_partial = (requested - pending).abs() > tolerance;
        if is_partial && !config.ff_bdp_partial_payments {
            return Err(format!(
                "Pago bloqueado: pagos parciales desactivados. Saldo={pending:.2}, solicitado={requested:.2}"
            ));
        }
        if requested > pending + tolerance {
            return Err(format!(
                "Pago bloqueado: el importe {requested:.2} excede el saldo pendiente {pending:.2}"
            ));
        }
        Ok(is_partial)
    }

    /* [267A-8] Arma el pago: snapshot remoto + authorize [187A-1] + payment_id
     * determinístico [247A-9] + request AddOrderPayment. */
    async fn armar_pago(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
        intento: &IntentoPago,
        idempotency_key: Option<&str>,
    ) -> Result<PagoArmado, String> {
        /* [187A-1] El snapshot remoto debe completarse antes de consumir el
         * armado; authorize registra la intención y cierra el modo escritura. */
        let datos_pago = serde_json::json!({
            "venta_id": venta.id,
            "order_id": intento.order_id,
            "amount": intento.amount,
            "tender_id": intento.tender_id,
            "is_partial": intento.is_partial
        });
        let snapshot_pre_id = crate::services::BdpBackupService::preparar_snapshot_escritura(
            pool,
            venta.user_id,
            "add_payment",
            config,
            Some(intento.order_id),
        )
        .await
        .map_err(|e| format!("Pre-write audit BDP falló; pago bloqueado: {e}"))?;

        let audit_id = crate::services::BdpWriteGuard::authorize(
            pool,
            venta.user_id,
            config,
            "add_payment",
            "venta",
            venta.id,
            "venta_id",
            &datos_pago,
            snapshot_pre_id,
            idempotency_key,
        )
        .await?;

        /* [247A-9] ID de pago determinístico a partir de la idempotencia para
         * que los reintentos con la misma clave reutilicen el mismo identificador
         * en BDP y no se creen pagos duplicados. */
        let venta_prefix = venta.id.simple().to_string();
        let venta_prefix = &venta_prefix[..14];
        let payment_id = idempotency_key.map_or_else(
            || {
                let uuid_short = Uuid::new_v4().simple().to_string();
                let uuid_short = &uuid_short[..8];
                format!("P{venta_prefix}-{uuid_short}")
            },
            |k| format!("P{venta_prefix}-{k}"),
        );

        let request = BdpAddOrderPaymentRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(intento.order_id),
            payment: BdpOrderPayment {
                tender_id: intento.tender_id,
                amount: intento.amount,
                payment_id: payment_id.clone(),
            },
            invoice: None,
            pos_id: Some(config.bdp_pos_id),
            employee_id: Some(config.bdp_employee_id),
            invoice_parameters: None,
        };
        Ok(PagoArmado {
            audit_id,
            intento: *intento,
            payment_id,
            request,
        })
    }

    /* [F4.4] Clasifica el error de AddOrderPayment, cierra la auditoría
     * como ambigua/error y devuelve el mensaje para el llamante. */
    async fn gestionar_error_pago(
        pool: &PgPool,
        audit_id: Uuid,
        e: crate::services::bdp_weblink::BdpWeblinkError,
    ) -> String {
        let msg = format!("Error AddOrderPayment: {e}");
        let resultado = match e {
            crate::services::bdp_weblink::BdpWeblinkError::Http(_)
            | crate::services::bdp_weblink::BdpWeblinkError::Api { .. }
            | crate::services::bdp_weblink::BdpWeblinkError::Throttled(_) => "ambiguo",
            _ => "error",
        };
        let _ = crate::services::BdpBackupService::actualizar_resultado(
            pool,
            audit_id,
            resultado,
            None,
            Some(&msg),
        )
        .await;
        msg
    }

    /* [267A-8] Ejecuta AddOrderPayment en BDP y confirma marca + auditoría +
     * ledger en tx atómica [207A-2 S7-H2]. Devuelve el InvoiceNumber si BDP
     * facturó al cobrar. */
    async fn confirmar_pago(
        pool: &PgPool,
        venta: &Venta,
        armado: &PagoArmado,
        idempotency_key: Option<&str>,
        response: &serde_json::Value,
    ) -> Result<(), String> {
        let invoice_number = response
            .get("InvoiceNumber")
            .and_then(|v| v.as_str())
            .map(String::from);

        let mut tx = pool
            .begin()
            .await
            .map_err(|e| format!("Error iniciando tx post-pago: {e}"))?;

        if let Some(ref inv) = invoice_number {
            /* [F8.3] Si BDP devolvió InvoiceNumber, marcar venta como facturada. */
            info!(
                "[F8.1] Pago registrado en BDP para venta {} → InvoiceNumber={inv}",
                venta.id
            );
            sqlx::query(
                "UPDATE ventas SET bdp_invoiced = true, bdp_order_status = 'invoiced', updated_at = NOW() WHERE id = $1",
            )
            .bind(venta.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("BDP confirmó pago y factura {inv}, pero no se pudo persistir localmente: {e}"))?;
        } else {
            info!(
                "[F8.1] Pago registrado en BDP para venta {} (sin InvoiceNumber)",
                venta.id
            );
        }

        /* Cerrar auditoría dentro de la misma transacción. */
        sqlx::query(
            r"UPDATE bdp_audit_log
            SET resultado = 'exito', datos_respuesta = $2, error_mensaje = NULL, updated_at = NOW()
            WHERE id = $1",
        )
        .bind(armado.audit_id)
        .bind(Some(response))
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Pago confirmado, pero falló el cierre de auditoría: {e}"))?;

        /* [247A-9] Registrar pago local en el mismo commit atómico. */
        let idempotency_for_insert = idempotency_key.unwrap_or(&armado.payment_id).to_string();
        let pago_respuesta: Option<serde_json::Value> = Some(response.clone());
        sqlx::query(
            r"INSERT INTO bdp_pagos
            (venta_id, amount, tender_id, idempotency_key, bdp_order_id, bdp_payment_id, resultado, datos_respuesta)
            VALUES ($1, $2, $3, $4, $5, $6, 'exito', $7)
            ON CONFLICT (idempotency_key) DO UPDATE SET
                updated_at = NOW(),
                resultado = EXCLUDED.resultado,
                datos_respuesta = EXCLUDED.datos_respuesta
            WHERE bdp_pagos.venta_id = EXCLUDED.venta_id
              AND bdp_pagos.amount = EXCLUDED.amount
              AND bdp_pagos.tender_id = EXCLUDED.tender_id
            RETURNING id",
        )
        .bind(venta.id)
        .bind(armado.intento.amount)
        .bind(armado.intento.tender_id)
        .bind(idempotency_for_insert)
        .bind(armado.intento.order_id)
        .bind(Some(&armado.payment_id))
        .bind(pago_respuesta)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| format!("Pago confirmado, pero falló el registro en ledger: {e}"))?;

        tx.commit()
            .await
            .map_err(|e| format!("Error confirmando tx post-pago: {e}"))
    }

    /* [267A-8] Ejecuta AddOrderPayment en BDP y confirma marca + auditoría +
     * ledger en tx atómica [207A-2 S7-H2]. Devuelve el InvoiceNumber si BDP
     * facturó al cobrar. */
    async fn ejecutar_pago(
        client: &BdpWeblinkClient<'_>,
        pool: &PgPool,
        venta: &Venta,
        armado: &PagoArmado,
        idempotency_key: Option<&str>,
    ) -> Result<Option<String>, String> {
        let response = match client.add_order_payment(&armado.request).await {
            Ok(response) => response,
            Err(e) => {
                return Err(Self::gestionar_error_pago(pool, armado.audit_id, e).await);
            }
        };

        let invoice_number = response
            .get("InvoiceNumber")
            .and_then(|v| v.as_str())
            .map(String::from);

        /* [207A-2] S7-H2: Envolver marca local + auditoría + ledger en
         * transacción para consistencia. */
        let commit_result =
            Self::confirmar_pago(pool, venta, armado, idempotency_key, &response).await;

        if let Err(e) = commit_result {
            /* La tx falló pero BDP ya procesó el pago → auditoría ambigua. */
            let _ = crate::services::BdpBackupService::actualizar_resultado(
                pool,
                armado.audit_id,
                "ambiguo",
                Some(&response),
                Some(&e),
            )
            .await;
            return Err(e);
        }

        Ok(invoice_number)
    }
}
