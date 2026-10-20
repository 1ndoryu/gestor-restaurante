/* [D27] Guarda de stock por venta: una venta que ya está en BDP (enviada o nacida allí)
 * ya movió su stock; escribirlo otra vez duplicaría el movimiento.
 * No hay columna `origen`: el criterio usa `bdp_synced` y `bdp_order_id`.
 * Sin escritor local todavía; la usará el primer escritor de stock por venta. */

use super::BdpWriteGuard;

impl BdpWriteGuard {
    /// Devuelve `true` solo si la venta puede generar un movimiento de stock en BDP.
    #[must_use]
    pub fn venta_puede_mover_stock(bdp_synced: bool, bdp_order_id: Option<i64>) -> bool {
        !bdp_synced && bdp_order_id.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::BdpWriteGuard;

    /* Venta local sin rastro en BDP: sí puede mover stock. */
    #[test]
    fn venta_local_sin_rastro_puede_mover_stock() {
        assert!(BdpWriteGuard::venta_puede_mover_stock(false, None));
    }

    /* Venta ya sincronizada: bloquea aunque falte el id de pedido. */
    #[test]
    fn venta_sincronizada_no_mueve_stock() {
        assert!(!BdpWriteGuard::venta_puede_mover_stock(true, None));
    }

    /* Venta con id de pedido en BDP (origen TPV o ya enviada): bloquea aunque no esté marcada. */
    #[test]
    fn venta_con_pedido_bdp_no_mueve_stock() {
        assert!(!BdpWriteGuard::venta_puede_mover_stock(false, Some(42)));
    }
}
