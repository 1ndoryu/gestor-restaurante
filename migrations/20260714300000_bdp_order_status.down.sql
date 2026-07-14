/* Revertir bdp_order_status y bdp_poll_interval_secs */
DROP INDEX IF EXISTS idx_ventas_bdp_poll;
ALTER TABLE configuracion_restaurante DROP COLUMN IF EXISTS bdp_poll_interval_secs;
ALTER TABLE ventas DROP COLUMN IF EXISTS bdp_order_status;
