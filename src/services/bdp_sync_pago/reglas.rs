/* Reglas puras del pago BDP: clasificación del error, tope por comanda y
 * lectura del número de factura. Sin red ni BD, para poder probarlas sin TPV.
 * Extraído de `bdp_sync_pago.rs` para no pasar el umbral de god-object. */

use serde_json::Value;

use crate::services::bdp_weblink::BdpWeblinkError;

/* [21AB-1] ¿El fallo deja el pago en el aire (pudo aplicarse en BDP)? Los de
 * transporte, HTTP y throttling sí: la petición pudo llegar. `RespuestaPerdida`
 * es el caso exacto que pide Guillermo —BDP cobró y la web no leyó la
 * respuesta—: clasificarlo `error` lo dejaría fuera de
 * `reconciliar_pagos_ambiguos` y el reintento reenviaría el cobro. Los de
 * negocio (`Remote`) y configuración no escriben nada en BDP. */
pub(super) fn pago_queda_ambiguo(e: &BdpWeblinkError) -> bool {
    matches!(
        e,
        BdpWeblinkError::Http(_)
            | BdpWeblinkError::Api { .. }
            | BdpWeblinkError::Throttled(_)
            | BdpWeblinkError::RespuestaPerdida(_)
    )
}

/* BDP devuelve "" en InvoiceNumber cuando el pago no factura: eso no es un número
 * de factura y no debe marcar la venta como facturada. */
pub(super) fn numero_factura(response: &Value) -> Option<String> {
    response
        .get("InvoiceNumber")
        .and_then(Value::as_str)
        .filter(|numero| !numero.trim().is_empty())
        .map(String::from)
}

/* [D17] Máximo de pagos BDP por venta (anexo B8, D17). */
pub(super) const MAX_PAGOS_VENTA: usize = 3;

pub(super) fn tope_pagos_alcanzado(pagos_remotos: usize) -> bool {
    pagos_remotos >= MAX_PAGOS_VENTA
}

#[cfg(test)]
mod tests {
    use super::{numero_factura, pago_queda_ambiguo, tope_pagos_alcanzado};
    use crate::services::bdp_weblink::BdpWeblinkError;
    use serde_json::json;

    /* [21AB-1] Una respuesta perdida en `AddOrderPayment` pudo cobrar en BDP:
     * tiene que ir a `ambiguo` para que el reintento lo mire con `GetOrder`. */
    #[test]
    fn respuesta_perdida_queda_ambigua() {
        assert!(pago_queda_ambiguo(&BdpWeblinkError::RespuestaPerdida(
            "corte a media respuesta".into()
        )));
        assert!(pago_queda_ambiguo(&BdpWeblinkError::Http("timeout".into())));
        assert!(pago_queda_ambiguo(&BdpWeblinkError::Api {
            status: 500,
            body: "sin cuerpo".into()
        }));
        assert!(pago_queda_ambiguo(&BdpWeblinkError::Throttled(
            "2 de 2".into()
        )));
    }

    /* Un rechazo de negocio no escribió nada: reintentarlo es seguro. */
    #[test]
    fn rechazo_de_negocio_no_es_ambiguo() {
        assert!(!pago_queda_ambiguo(&BdpWeblinkError::Remote(
            "[300036]-IMPORTE".into()
        )));
        assert!(!pago_queda_ambiguo(&BdpWeblinkError::NotConfigured));
    }

    #[test]
    fn tres_pagos_bloquean_el_cuarto() {
        assert!(!tope_pagos_alcanzado(2));
        assert!(tope_pagos_alcanzado(3));
        assert!(tope_pagos_alcanzado(4));
    }

    #[test]
    fn respuesta_vacia_o_ausente_no_es_factura() {
        assert_eq!(numero_factura(&json!({"InvoiceNumber": ""})), None);
        assert_eq!(numero_factura(&json!({"InvoiceNumber": "  "})), None);
        assert_eq!(numero_factura(&json!({"ErrorMessage": ""})), None);
    }

    #[test]
    fn numero_real_se_conserva() {
        assert_eq!(
            numero_factura(&json!({"InvoiceNumber": "F-2026-0001"})),
            Some("F-2026-0001".to_string())
        );
    }
}
