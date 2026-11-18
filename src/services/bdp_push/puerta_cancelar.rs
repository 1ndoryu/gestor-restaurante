//! [18AB-3] Puerta del `CancelOrder`: antes de cancelar en BDP se consultan la
//! comanda y sus pagos. Guillermo, 18/11/2026: «consultar siempre estado y pagos
//! antes de cancelar; si BDP no contesta, no cancelar a ciegas».

use serde_json::Value;

use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{BdpGetOrderRequest, BdpOrderIdentifier};

use super::cola::BdpPushPendiente;

/// `Ok` si la cancelación puede seguir; `Err(motivo)` la deja sin enviar.
/// El `OrderIdentifier` sale del payload de la fila (lo que se iba a enviar).
pub(super) async fn puede_cancelar(
    client: &BdpWeblinkClient<'_>,
    pendiente: &BdpPushPendiente,
) -> Result<(), String> {
    let order_id = order_id_del_payload(&pendiente.payload).ok_or_else(|| {
        "Cancelación bloqueada: el payload no lleva OrderId, no se puede comprobar el estado"
            .to_string()
    })?;
    let respuesta = client
        .get_order(&BdpGetOrderRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
        })
        .await
        .map_err(|error| {
            /* BDP sin respuesta: la fila queda pendiente y se reintenta cuando el
             * TPV vuelva. Nunca se cancela a ciegas. */
            format!("Cancelación bloqueada: BDP no confirmó el estado de la comanda {order_id} ({error})")
        })?;

    let Some(order) = respuesta.get("Order").filter(|order| !order.is_null()) else {
        /* Comanda facturada: `Order: null` con `Status: 3`. Con el cobro
         * confirmado, la fila de cancelación no debe seguir. */
        if respuesta.get("Status").and_then(Value::as_i64) == Some(3) {
            return Err(format!(
                "Cancelación bloqueada: la comanda {order_id} ya está facturada en BDP"
            ));
        }
        return Err(format!(
            "Cancelación bloqueada: BDP no devolvió la comanda {order_id}"
        ));
    };

    let pagado = pagado_remoto(order);
    if pagado > 0.0 {
        return Err(format!(
            "Cancelación bloqueada: la comanda {order_id} tiene pagos en BDP ({pagado:.2} €). \
             Retira los pagos en el TPV antes de anularla."
        ));
    }
    /* Sin pagos, una comanda abierta sí se cancela: es la operación normal.
     * Solo se bloquea si BDP ya la facturó (el `Order` sigue llegando). */
    let estado = respuesta
        .get("Status")
        .or_else(|| order.get("Status"))
        .and_then(Value::as_i64);
    if estado == Some(3) {
        return Err(format!(
            "Cancelación bloqueada: la comanda {order_id} ya está facturada en BDP"
        ));
    }
    Ok(())
}

fn order_id_del_payload(payload: &Value) -> Option<i64> {
    let identificador = payload.get("OrderIdentifier")?;
    identificador
        .get("OrderId")
        .or_else(|| identificador.get("OrderID"))
        .and_then(Value::as_i64)
        .or_else(|| {
            identificador
                .get("OrderId")
                .and_then(Value::as_str)?
                .trim()
                .parse()
                .ok()
        })
}

/// Suma de `Payments[].Amount`. Un importe ilegible cuenta como cobro.
fn pagado_remoto(order: &Value) -> f64 {
    let Some(payments) = order.get("Payments").and_then(Value::as_array) else {
        return 0.0;
    };
    payments
        .iter()
        .map(|pago| {
            let amount = pago.get("Amount");
            amount
                .and_then(|v| v.as_f64().or_else(|| v.as_str()?.trim().parse().ok()))
                .unwrap_or(f64::MAX)
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn order_id_sale_del_payload() {
        assert_eq!(
            order_id_del_payload(&json!({"PosId": 1, "OrderIdentifier": {"OrderId": 121}})),
            Some(121)
        );
        assert_eq!(
            order_id_del_payload(&json!({"OrderIdentifier": {"OrderId": "131"}})),
            Some(131)
        );
        assert_eq!(order_id_del_payload(&json!({"PosId": 1})), None);
    }

    #[test]
    fn pagado_remoto_cuenta_los_pagos() {
        let order = json!({"Payments": [{"Amount": 2.0}, {"Amount": "2.20"}]});
        assert!((pagado_remoto(&order) - 4.2).abs() < 1e-9);
        assert!(pagado_remoto(&json!({"Total": 3.15})).abs() < 1e-9);
    }

    #[test]
    fn importe_ilegible_bloquea() {
        assert_eq!(
            pagado_remoto(&json!({"Payments": [{"Amount": "x"}]})),
            f64::MAX
        );
    }
}
