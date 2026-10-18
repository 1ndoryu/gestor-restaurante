use crate::services::bdp_weblink::BdpWeblinkError;

use super::constantes::{ESTADO_ERROR, ESTADO_PENDIENTE_SUSCRIPCION, ESTADO_RECHAZADO};

/* `entidad_uuid` se movió a `bdp_write_guard.rs` [10AA-3-B]: el armado manual
 * tiene que derivar el mismo valor que el worker, y una sola copia lo garantiza. */

pub(super) fn es_transitorio(error: &BdpWeblinkError) -> bool {
    matches!(
        error,
        BdpWeblinkError::Http(_) | BdpWeblinkError::Throttled(_)
    ) || matches!(
        error,
        /* [049A-1/S5] 4xx es rechazo definitivo del BDP (payload inválido,
         * conflicto de negocio): reintentarlo en bucle es inútil y puede
         * enmascarar un dato local roto. Solo 5xx es transitorio. */
        BdpWeblinkError::Api { status, .. } if *status >= 500
    )
}

pub(super) fn clasificar_error(error: &BdpWeblinkError) -> (&'static str, bool) {
    if let BdpWeblinkError::Remote(mensaje) = error {
        /* [049A-1/H-W-3] Match normalizado (case-insensitive y ambas grafías
         * subscrip-/suscrip-) para no perder el bloqueo por suscripción si el
         * BDP varía el texto exacto; si se pierde, caería a reintento transitorio
         * (acotado a REINTENTOS_MAX, no desastre, pero sin la semántica manual). */
        let normalizado = mensaje.trim().to_lowercase();
        if normalizado.contains("subscripción no activada")
            || normalizado.contains("subscripcion no activada")
            || normalizado.contains("suscripción no activada")
            || normalizado.contains("suscripcion no activada")
        {
            return (ESTADO_PENDIENTE_SUSCRIPCION, false);
        }
    }
    if let BdpWeblinkError::Api { status, .. } = error {
        /* [049A-1/S5] Rechazo definitivo: 4xx no se reintenta en bucle. */
        if *status < 500 {
            return (ESTADO_RECHAZADO, false);
        }
    }
    if es_transitorio(error) {
        (ESTADO_ERROR, true)
    } else {
        (ESTADO_ERROR, false)
    }
}
