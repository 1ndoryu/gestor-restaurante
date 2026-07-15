-- Revertir campos BDP de clientes
DROP INDEX IF EXISTS idx_clientes_bdp_customer_code;

ALTER TABLE clientes
    DROP COLUMN IF EXISTS bdp_customer_code,
    DROP COLUMN IF EXISTS bdp_synced,
    DROP COLUMN IF EXISTS bdp_synced_at,
    DROP COLUMN IF EXISTS bdp_sync_error;
