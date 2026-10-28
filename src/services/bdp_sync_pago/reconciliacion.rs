// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [D15] Reconciliación de pagos ambiguos: cruza el intento con los Payments de BDP
 * (solo lectura) antes de permitir otra escritura. Extraído de bdp_sync_pago.rs para
 * mantener el archivo bajo el umbral de god-object del gate. */

use std::collections::HashMap;
use std::fmt::Write;

use rust_decimal::prelude::Decimal;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Venta;
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{BdpGetOrderRequest, BdpOrderIdentifier};

use super::super::bdp_sync::BdpSyncService;
use super::IntentoPago;

impl BdpSyncService {
    /* [D15] Reconcilia los add_payment ambiguos de la venta antes de escribir de nuevo. */
    pub(super) async fn reconciliar_pagos_ambiguos(
        client: &BdpWeblinkClient<'_>,
        pool: &PgPool,
        venta: &Venta,
    ) -> Result<(), String> {
        let filas: Vec<(Uuid, Option<String>, Value)> = sqlx::query_as(
            "SELECT id, idempotency_key, datos_enviados FROM bdp_audit_log \
             WHERE user_id = $1 AND operacion = 'add_payment' AND resultado = 'ambiguo' \
               AND datos_enviados ->> 'venta_id' = $2 ORDER BY created_at",
        )
        .bind(venta.user_id)
        .bind(venta.id.to_string())
        .fetch_all(pool)
        .await
        .map_err(|e| format!("No se pudo leer los pagos ambiguos: {e}"))?;
        for (audit_id, key, datos) in filas {
            Self::reconciliar_pago_ambiguo(client, pool, venta, audit_id, key.as_deref(), &datos)
                .await?;
        }
        Ok(())
    }

    /* [D15] Solo lectura en BDP (GetOrder). Aplicado: registra el pago en el ledger local
     * con el cierre atómico de un éxito. No aplicado: marca `error` para permitir el
     * reintento con el mismo PaymentId determinístico. Otro caso: deja el bloqueo. */
    async fn reconciliar_pago_ambiguo(
        client: &BdpWeblinkClient<'_>,
        pool: &PgPool,
        venta: &Venta,
        audit_id: Uuid,
        key: Option<&str>,
        datos: &Value,
    ) -> Result<(), String> {
        let key = key.ok_or(
            "Pago bloqueado: intento ambiguo sin clave de idempotencia; no se puede reconciliar",
        )?;
        let intento = IntentoPago {
            order_id: datos
                .get("order_id")
                .and_then(Value::as_i64)
                .ok_or("Pago bloqueado: intento ambiguo sin order_id")?,
            amount: Self::parse_remote_money(
                datos
                    .get("amount")
                    .ok_or("Pago bloqueado: intento ambiguo sin amount")?,
                "amount",
            )?,
            tender_id: datos
                .get("tender_id")
                .and_then(Value::as_i64)
                .and_then(|id| i32::try_from(id).ok())
                .ok_or("Pago bloqueado: intento ambiguo sin tender_id")?,
            is_partial: datos
                .get("is_partial")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        };
        let payment_id = payment_id_determinista(venta.id, key);

        let remota = client
            .get_order(&BdpGetOrderRequest {
                order_identifier: BdpOrderIdentifier::by_order_id(intento.order_id),
            })
            .await
            .map_err(|e| {
                format!("Pago bloqueado: no se pudo consultar la orden para reconciliar: {e}")
            })?;
        let payments = remota
            .get("Order")
            .and_then(|orden| orden.get("Payments"))
            .and_then(Value::as_array)
            .ok_or("Pago bloqueado: GetOrder no devolvió Payments para reconciliar")?;
        let (montos, _) = Self::parse_remote_payments(payments)?;

        match decidir_reconciliacion(&montos, &payment_id, intento.amount) {
            DecisionPago::Aplicado => {
                let respuesta = json!({
                    "reconciliado": true,
                    "PaymentId": payment_id,
                    "Amount": intento.amount.to_string(),
                });
                Self::confirmar_pago(
                    pool,
                    venta,
                    audit_id,
                    &intento,
                    &payment_id,
                    Some(key),
                    &respuesta,
                )
                .await
            }
            DecisionPago::NoAplicado => {
                let msg =
                    format!("Reconciliado: BDP no tiene el pago {payment_id}; reintento permitido");
                crate::services::BdpBackupService::actualizar_resultado(
                    pool,
                    audit_id,
                    "error",
                    None,
                    Some(&msg),
                )
                .await
            }
            DecisionPago::Discordante => Err(format!(
                "Pago bloqueado: PaymentId {payment_id} no cuadra con BDP; revisión manual"
            )),
        }
    }
}

/* [D15] ID de pago determinístico por venta e idempotencia: un reintento reutiliza el mismo.
 * BDP y bdp_pagos.bdp_payment_id admiten 50 caracteres. Con una clave UUID (36) el ID
 * pasaba de 50: BDP lo truncaba, el ledger local no podía guardarlo y la reconciliación
 * no lo encontraba. Si no cabe, se usa un resumen de la clave (32 hex). */
const LONGITUD_MAX_PAYMENT_ID: usize = 50;

pub(super) fn payment_id_determinista(venta_id: Uuid, key: &str) -> String {
    let prefijo = venta_id.simple().to_string();
    let id = format!("P{}-{key}", &prefijo[..14]);
    if id.len() <= LONGITUD_MAX_PAYMENT_ID {
        return id;
    }
    let resumen = Sha256::digest(key.as_bytes())
        .iter()
        .take(16)
        .fold(String::new(), |mut hex, b| {
            let _ = write!(hex, "{b:02x}");
            hex
        });
    format!("P{}-{resumen}", &prefijo[..14])
}

/* [D15] Resultado de cruzar un intento ambiguo con los Payments remotos. */
#[derive(Debug, PartialEq, Eq)]
enum DecisionPago {
    Aplicado,
    NoAplicado,
    Discordante,
}

/* [D15] Solo cuenta como aplicado si BDP tiene su PaymentId con el mismo importe. */
fn decidir_reconciliacion(
    montos: &HashMap<String, Decimal>,
    payment_id: &str,
    amount: Decimal,
) -> DecisionPago {
    match montos.get(payment_id) {
        None => DecisionPago::NoAplicado,
        Some(remoto) if *remoto == amount => DecisionPago::Aplicado,
        Some(_) => DecisionPago::Discordante,
    }
}

#[cfg(test)]
mod tests_reconciliacion {
    use super::*;
    use std::str::FromStr;

    fn d(texto: &str) -> Decimal {
        Decimal::from_str(texto).expect("decimal de prueba")
    }

    #[test]
    fn payment_id_coincide_con_el_de_la_prueba_d15() {
        let venta = Uuid::parse_str("6aa916e1-ca9e-4100-8000-000000000001").expect("uuid");
        assert_eq!(
            payment_id_determinista(venta, "d15-pago-1"),
            "P6aa916e1ca9e41-d15-pago-1"
        );
    }

    #[test]
    fn clave_uuid_no_supera_los_50_caracteres() {
        let venta = Uuid::parse_str("4cf3165a-df61-4570-99e1-6005f3a29549").expect("uuid");
        let clave = Uuid::new_v4().to_string();
        let id = payment_id_determinista(venta, &clave);
        assert!(id.len() <= 50, "PaymentId de {} caracteres", id.len());
        assert_eq!(id, payment_id_determinista(venta, &clave), "debe ser determinístico");
    }

    #[test]
    fn mismo_id_y_importe_se_considera_aplicado() {
        let montos = HashMap::from([("P1-k".to_string(), d("3.00"))]);
        assert_eq!(
            decidir_reconciliacion(&montos, "P1-k", d("3")),
            DecisionPago::Aplicado
        );
    }

    #[test]
    fn id_ausente_no_esta_aplicado() {
        let montos = HashMap::from([("otro".to_string(), d("3"))]);
        assert_eq!(
            decidir_reconciliacion(&montos, "P1-k", d("3")),
            DecisionPago::NoAplicado
        );
    }

    #[test]
    fn mismo_id_con_importe_distinto_es_discordante() {
        let montos = HashMap::from([("P1-k".to_string(), d("5"))]);
        assert_eq!(
            decidir_reconciliacion(&montos, "P1-k", d("3")),
            DecisionPago::Discordante
        );
    }
}
