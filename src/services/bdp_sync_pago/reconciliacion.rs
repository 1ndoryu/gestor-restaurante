// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [D15] Reconciliación de pagos ambiguos: cruza el intento con los Payments de BDP
 * (solo lectura) antes de permitir otra escritura. Extraído de bdp_sync_pago.rs para
 * mantener el archivo bajo el umbral de god-object del gate. */

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
     * reintento con el mismo PaymentId del pago. Otro caso: deja el bloqueo. */
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
        /* [18AB-1] El id viene de la auditoría, donde lo guardó el armado. Las
         * auditorías anteriores a `18AB-1` no lo tienen: se deriva de la clave,
         * igual que entonces. */
        let payment_id = datos
            .get("payment_id")
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .map_or_else(
                || payment_id_de_pago(venta.id, key),
                |id| id.trim().to_string(),
            );

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

        match estado_pago_remoto(payments, &payment_id, intento.amount, intento.tender_id)? {
            PagoRemoto::Aplicado => {
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
            PagoRemoto::Ausente => {
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
        }
    }
}

/* [18AB-1] El `PaymentId` **identifica el pago, no el intento**: sale de la
 * clave de idempotencia del pago y no cambia entre reintentos. Es lo que
 * Guillermo pide el 18/11: un reintento del mismo pago no puede generar un id
 * nuevo, porque BDP 36.2 no deduplica y dejaría dos cobros (probado el
 * 18/11/2026, `AddOrderPayment` con el mismo `PaymentId` cobra dos veces).
 * La protección real es la lectura previa a BDP antes de reenviar
 * (`reconciliar_pagos_ambiguos`), no el id por sí solo.
 *
 * BDP y `bdp_pagos.bdp_payment_id` admiten 50 caracteres: con una clave UUID
 * (36) el id pasaba de 50, BDP lo truncaba y la reconciliación no lo
 * encontraba. Si no cabe, se usa un resumen (32 hex) de la clave. */
const LONGITUD_MAX_PAYMENT_ID: usize = 50;

pub(crate) fn payment_id_de_pago(venta_id: Uuid, key: &str) -> String {
    let prefijo = venta_id.simple().to_string();
    let id = format!("P{}-{key}", &prefijo[..14]);
    if id.len() <= LONGITUD_MAX_PAYMENT_ID {
        return id;
    }
    let digest = Sha256::digest(key.as_bytes());
    let resumen = digest.iter().take(16).fold(String::new(), |mut hex, b| {
        let _ = write!(hex, "{b:02x}");
        hex
    });
    format!("P{}-{resumen}", &prefijo[..14])
}

/* [18AB-1] Pago de un intento sin clave de idempotencia. Sin clave no hay
 * forma de reconocer un reintento del mismo cobro, así que lleva un id propio
 * por armado; la protección sigue estando en la reconciliación del ambiguo,
 * que lee el id guardado en la auditoría. */
pub(crate) fn payment_id_intento(venta_id: Uuid) -> String {
    let prefijo = venta_id.simple().to_string();
    let nonce = Uuid::new_v4().simple().to_string();
    format!("P{}-{}", &prefijo[..14], &nonce[..16])
}

/* [18AB-1] Resultado de buscar nuestro pago en los `Payments` de BDP. */
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PagoRemoto {
    Ausente,
    Aplicado,
}

/* [18AB-1] ¿El pago ya está en BDP? Guillermo pide no reenviar cuando aparece
 * con el mismo identificador, importe y forma de pago. Un identificador
 * coincidente con otro importe o con otra forma de pago no se reenvía ni se
 * da por bueno: bloquea. BDP no siempre devuelve `TenderId` en `Payments`, así
 * que sin él se compara solo identificador e importe. */
pub(crate) fn estado_pago_remoto(
    payments: &[Value],
    payment_id: &str,
    amount: Decimal,
    tender_id: i32,
) -> Result<PagoRemoto, String> {
    let mut encontrado = false;
    for payment in payments {
        let Some(id) = BdpSyncService::remote_payment_id(payment.get("PaymentId")) else {
            continue;
        };
        if id != payment_id {
            continue;
        }
        let remoto = BdpSyncService::parse_remote_money(
            payment
                .get("Amount")
                .ok_or("Pago bloqueado: un pago remoto no contiene Amount")?,
            "Payments.Amount",
        )?;
        if remoto != amount {
            return Err(format!(
                "Pago bloqueado: BDP ya tiene el pago {payment_id} por {remoto} y se pidió {amount}"
            ));
        }
        if let Some(tender) = payment.get("TenderId").and_then(Value::as_i64) {
            if tender != i64::from(tender_id) {
                return Err(format!(
                    "Pago bloqueado: el pago {payment_id} está en BDP con la forma de pago {tender} y se pidió {tender_id}"
                ));
            }
        }
        encontrado = true;
    }
    Ok(if encontrado {
        PagoRemoto::Aplicado
    } else {
        PagoRemoto::Ausente
    })
}

#[cfg(test)]
mod tests_reconciliacion {
    use super::*;
    use std::str::FromStr;

    fn d(texto: &str) -> Decimal {
        Decimal::from_str(texto).expect("decimal de prueba")
    }

    fn pago(id: &str, importe: serde_json::Value, tender: Option<i64>) -> Value {
        let mut pago = json!({ "PaymentId": id, "Amount": importe });
        if let Some(tender) = tender {
            pago["TenderId"] = json!(tender);
        }
        pago
    }

    #[test]
    fn payment_id_coincide_con_el_de_la_prueba_d15() {
        let venta = Uuid::parse_str("6aa916e1-ca9e-4100-8000-000000000001").expect("uuid");
        assert_eq!(
            payment_id_de_pago(venta, "d15-pago-1"),
            "P6aa916e1ca9e41-d15-pago-1"
        );
    }

    /* [18AB-1] El id identifica el pago: dos armados de la misma clave llevan
     * el mismo, y claves distintas no colisionan. */
    #[test]
    fn el_mismo_pago_conserva_su_payment_id_entre_intentos() {
        let venta = Uuid::parse_str("4cf3165a-df61-4570-99e1-6005f3a29549").expect("uuid");
        let uno = payment_id_de_pago(venta, "cobro-1");
        let dos = payment_id_de_pago(venta, "cobro-1");
        assert_eq!(uno, dos, "un reintento no puede cambiar el id del pago");
        assert_ne!(uno, payment_id_de_pago(venta, "cobro-2"));
        assert!(uno.len() <= LONGITUD_MAX_PAYMENT_ID);
        assert!(uno.starts_with("P4cf3165adf614"));
    }

    /* [18AB-1] Sin clave no hay reintento reconocible: el id es del armado. */
    #[test]
    fn sin_clave_cada_armado_lleva_su_propio_payment_id() {
        let venta = Uuid::parse_str("4cf3165a-df61-4570-99e1-6005f3a29549").expect("uuid");
        let uno = payment_id_intento(venta);
        let dos = payment_id_intento(venta);
        assert_ne!(uno, dos);
        assert!(uno.len() <= LONGITUD_MAX_PAYMENT_ID && uno.len() <= 50);
        assert!(uno.starts_with("P4cf3165adf614"));
    }

    #[test]
    fn clave_uuid_no_supera_los_50_caracteres() {
        let venta = Uuid::parse_str("4cf3165a-df61-4570-99e1-6005f3a29549").expect("uuid");
        let clave = Uuid::new_v4().to_string();
        let id = payment_id_de_pago(venta, &clave);
        assert!(id.len() <= 50, "PaymentId de {} caracteres", id.len());
        assert_eq!(id, payment_id_de_pago(venta, &clave), "determinístico");
    }

    #[test]
    fn mismo_id_e_importe_se_considera_aplicado() {
        let payments = vec![
            pago("otro", json!("1.00"), None),
            pago("P1-k", json!("3.00"), Some(1)),
        ];
        assert_eq!(
            estado_pago_remoto(&payments, "P1-k", d("3"), 1),
            Ok(PagoRemoto::Aplicado)
        );
    }

    /* [18AB-1] BDP no siempre devuelve TenderId: sin él, id e importe bastan. */
    #[test]
    fn sin_tender_remoto_bastan_id_e_importe() {
        let payments = vec![pago("P1-k", json!(3.0), None)];
        assert_eq!(
            estado_pago_remoto(&payments, "P1-k", d("3"), 1),
            Ok(PagoRemoto::Aplicado)
        );
    }

    #[test]
    fn id_ausente_no_esta_aplicado() {
        let payments = vec![pago("otro", json!("3.00"), None)];
        assert_eq!(
            estado_pago_remoto(&payments, "P1-k", d("3"), 1),
            Ok(PagoRemoto::Ausente)
        );
    }

    #[test]
    fn id_repetido_se_sigue_considerando_aplicado() {
        let payments = vec![
            pago("P1-k", json!("3.00"), None),
            pago("P1-k", json!("3.00"), None),
        ];
        assert_eq!(
            estado_pago_remoto(&payments, "P1-k", d("3"), 1),
            Ok(PagoRemoto::Aplicado)
        );
    }

    #[test]
    fn importe_distinto_bloquea() {
        let payments = vec![pago("P1-k", json!("5.00"), None)];
        let error = estado_pago_remoto(&payments, "P1-k", d("3"), 1).unwrap_err();
        assert!(error.contains("ya tiene el pago P1-k por 5"));
    }

    #[test]
    fn forma_de_pago_distinta_bloquea() {
        let payments = vec![pago("P1-k", json!("3.00"), Some(2))];
        let error = estado_pago_remoto(&payments, "P1-k", d("3"), 1).unwrap_err();
        assert!(error.contains("forma de pago 2"));
    }
}
