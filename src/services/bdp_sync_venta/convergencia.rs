/* [21AB-12] Aceptar el estado del TPV (punto 3 de Guillermo, 25/11): cuando una
 * comanda se modifica desde el TPV, la web lo detecta y se niega a reescribirse
 * sola (02AB-2, 28AA-6); hasta ahora la única salida era entrar en BDP o
 * depender de nosotros. Aquí la web relee `GetOrder` y adopta lo que dice el
 * TPV, con confirmación explícita y sin tocar el TPV.
 *
 * No es una escritura a BDP: la fuente es el propio TPV. Por eso no pasa por
 * `BdpWriteGuard` ni por auditoría de escritura; sí exige modo BDP, la etapa de
 * lectura y el permiso de la clase de escritura sensible sobre ventas. */

use serde_json::Value;
use sqlx::PgPool;

use crate::models::Venta;
use crate::repositories::{VentaLineaRepository, VentaRepository};
use crate::services::bdp_order_poller::BdpOrderPollerService;
use crate::services::bdp_sync::BdpSyncService;
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{BdpGetOrderRequest, BdpOrderIdentifier};

/// Resultado de una convergencia: lo que había, lo que hay y cómo quedó la venta.
pub struct ConvergenciaBdp {
    pub venta: Venta,
    pub lineas: Vec<crate::models::VentaLinea>,
    /// Total que la web tenía antes de aceptar el del TPV.
    pub total_antes: f64,
    /// Total que el TPV tiene para esa comanda.
    pub total_bdp: f64,
}

impl BdpSyncService {
    /// ¿La comanda está cerrada para cambios de líneas? Anulada, facturada en
    /// BDP o facturada en local: aceptar el estado del TPV reescribiría el
    /// documento que ya salió, así que se corta antes de leer.
    fn motivo_para_no_converger(venta: &Venta) -> Option<String> {
        if venta.anulada {
            return Some(
                "La venta está anulada: no se adopta el estado de una comanda anulada.".into(),
            );
        }
        if venta.bdp_invoiced || venta.facturada_local {
            return Some(
                "La venta ya está facturada: sus líneas no se pueden sustituir por las del TPV."
                    .into(),
            );
        }
        None
    }

    /* Adopta en la venta lo que el TPV tiene ahora mismo para su comanda:
     * relee `GetOrder`, sustituye las líneas, recalcula la cabecera y limpia el
     * aviso de conflicto. Una sola transacción: o queda todo del TPV o nada.
     *
     * Una comanda que BDP ya no devuelve no se puede adoptar: sin `Items` no hay
     * estado que aceptar y reescribir con cero líneas borraría la venta. */
    pub async fn aceptar_estado_tpv(
        pool: &PgPool,
        venta: &Venta,
        config: &crate::models::ConfiguracionRestaurante,
    ) -> Result<ConvergenciaBdp, String> {
        if let Some(motivo) = Self::motivo_para_no_converger(venta) {
            return Err(motivo);
        }
        let order_id = venta.bdp_order_id.filter(|id| *id > 0).ok_or_else(|| {
            "La venta no tiene comanda en el TPV: no hay estado que aceptar.".to_string()
        })?;

        let client = BdpWeblinkClient::new(config);
        let respuesta = client
            .get_order(&BdpGetOrderRequest {
                order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            })
            .await
            .map_err(|e| format!("No se pudo leer la comanda {order_id} en el TPV: {e}"))?;
        let pedido = respuesta.get("Order").unwrap_or(&respuesta);
        /* `GetOrder` de una comanda inexistente contesta sin `Items`: aceptar
         * eso dejaría la venta sin líneas. Se exige que el TPV la devuelva. */
        let Some(items) = pedido.get("Items").and_then(Value::as_array) else {
            return Err(format!(
                "El TPV no devolvió el detalle de la comanda {order_id}: no se adopta nada."
            ));
        };
        if items.is_empty() {
            return Err(format!(
                "La comanda {order_id} está vacía en el TPV: no se sustituyen las líneas de la venta por nada."
            ));
        }

        let lineas_antes = VentaLineaRepository::listar_por_venta(pool, venta.id)
            .await
            .map_err(|e| format!("No se pudieron leer las líneas de la venta: {e}"))?;
        let total_antes = BdpSyncService::total_pedido(
            venta,
            (!lineas_antes.is_empty()).then_some(&lineas_antes[..]),
        );
        let total_bdp = pedido
            .get("Total")
            .and_then(Value::as_f64)
            .unwrap_or(total_antes);

        let nuevas = BdpOrderPollerService::lineas_de_comanda(pool, venta.user_id, pedido).await?;
        if nuevas.is_empty() {
            return Err(format!(
                "La comanda {order_id} del TPV no tiene líneas convertibles: no se adopta nada."
            ));
        }

        let estado = BdpOrderPollerService::map_status(
            BdpOrderPollerService::parse_status(&respuesta).unwrap_or(0),
        );

        let mut tx = pool
            .begin()
            .await
            .map_err(|e| format!("No se pudo abrir la transacción: {e}"))?;
        /* `&mut tx` y no `&mut *tx`: la coerción a `&mut PgConnection` la hace
         * el compilador y `explicit_auto_deref` prohíbe el desreferenciado. */
        let lineas = VentaLineaRepository::reemplazar_conn(&mut tx, venta.id, &nuevas)
            .await
            .map_err(|e| format!("No se pudieron sustituir las líneas: {e}"))?;
        let actualizada = VentaRepository::recalcular_importes(&mut *tx, venta.id, venta.user_id)
            .await
            .map_err(|e| format!("No se pudo recalcular el importe: {e}"))?
            .ok_or_else(|| {
                "La venta desapareció mientras se adoptaba el estado del TPV.".to_string()
            })?;
        VentaRepository::update_bdp_order_status(&mut *tx, venta.id, &estado)
            .await
            .map_err(|e| format!("No se pudo guardar el estado de la comanda: {e}"))?;
        tx.commit()
            .await
            .map_err(|e| format!("No se pudo confirmar la adopción del estado del TPV: {e}"))?;

        /* Fuera de la transacción: el aviso de conflicto solo se limpia si
         * empieza por «Conflicto:», así que no puede pisar otro error. */
        VentaRepository::marcar_bdp_conflicto(pool, venta.id, None)
            .await
            .map_err(|e| format!("No se pudo limpiar el aviso de conflicto: {e}"))?;

        Ok(ConvergenciaBdp {
            venta: actualizada,
            lineas,
            total_antes,
            total_bdp,
        })
    }
}
