/* [065A-5] Rollback: eliminar campos BDP sync */
DROP INDEX IF EXISTS idx_ventas_bdp_synced;
ALTER TABLE ventas DROP COLUMN IF EXISTS bdp_order_id;
ALTER TABLE ventas DROP COLUMN IF EXISTS bdp_sync_error;
ALTER TABLE ventas DROP COLUMN IF EXISTS bdp_synced_at;
ALTER TABLE ventas DROP COLUMN IF EXISTS bdp_synced;
ALTER TABLE configuracion_restaurante DROP COLUMN IF EXISTS bdp_default_article_name;
ALTER TABLE configuracion_restaurante DROP COLUMN IF EXISTS bdp_default_article_code;
