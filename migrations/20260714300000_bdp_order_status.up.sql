/* [F4.1] Estado del pedido BDP en ventas.
 * Valores: 'pending' (enviado, esperando), 'confirmed' (BDP aceptó), 
 * 'invoiced' (facturada en TPV), 'error' (falló).
 * NULL = no se ha intentado sync o no aplica. */

ALTER TABLE ventas
    ADD COLUMN IF NOT EXISTS bdp_order_status TEXT;

/* [F4.5] Intervalo de polling para consultar estado de comandas BDP. */
ALTER TABLE configuracion_restaurante
    ADD COLUMN IF NOT EXISTS bdp_poll_interval_secs INTEGER NOT NULL DEFAULT 60;

/* Índice para el poller: ventas sincronizadas con estado no-final */
CREATE INDEX IF NOT EXISTS idx_ventas_bdp_poll 
    ON ventas(bdp_synced, bdp_order_status) 
    WHERE bdp_synced = TRUE AND bdp_order_status NOT IN ('invoiced', 'error');
