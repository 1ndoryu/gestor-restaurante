-- [Fase 7.3+7.4] Campos BDP para sync bidireccional de clientes.
-- bdp_customer_code: código del cliente en BDP (entero, asignado por BDP o por import).
-- bdp_synced: indica si el cliente fue sincronizado con BDP.
-- bdp_synced_at: fecha de última sincronización.
-- bdp_sync_error: último error de sincronización (si aplica).

ALTER TABLE clientes
    ADD COLUMN IF NOT EXISTS bdp_customer_code INTEGER,
    ADD COLUMN IF NOT EXISTS bdp_synced BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS bdp_synced_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS bdp_sync_error TEXT;

-- Índice para buscar clientes por código BDP (import/sync)
CREATE INDEX IF NOT EXISTS idx_clientes_bdp_customer_code
    ON clientes (bdp_customer_code)
    WHERE bdp_customer_code IS NOT NULL;
