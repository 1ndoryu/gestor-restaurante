/* [187A-1] Vincula cada autorización y evidencia BDP con el destino/configuración
 * exactos. Los registros legacy quedan deliberadamente inelegibles para escritura. */

ALTER TABLE bdp_snapshots
    ADD COLUMN IF NOT EXISTS target_base_url TEXT,
    ADD COLUMN IF NOT EXISTS connection_fingerprint TEXT;

ALTER TABLE bdp_write_arming
    ADD COLUMN IF NOT EXISTS snapshot_id UUID REFERENCES bdp_snapshots(id),
    ADD COLUMN IF NOT EXISTS connection_fingerprint TEXT;

ALTER TABLE bdp_audit_log
    ADD COLUMN IF NOT EXISTS target_base_url TEXT,
    ADD COLUMN IF NOT EXISTS target_entity_type TEXT,
    ADD COLUMN IF NOT EXISTS target_entity_id UUID,
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

ALTER TABLE bdp_write_arming
    DROP CONSTRAINT IF EXISTS bdp_write_arming_scopes_safe;

ALTER TABLE bdp_write_arming
    ADD CONSTRAINT bdp_write_arming_scopes_safe CHECK (
        cardinality(scopes) = 1
        AND scopes <@ ARRAY['create_order', 'create_customer', 'add_payment', 'invoice']::TEXT[]
    );

CREATE INDEX IF NOT EXISTS idx_bdp_snapshots_target
    ON bdp_snapshots(user_id, target_base_url, connection_fingerprint, created_at DESC)
    WHERE direccion = 'bdp';

CREATE INDEX IF NOT EXISTS idx_bdp_audit_target
    ON bdp_audit_log(user_id, target_entity_type, target_entity_id, operacion, created_at DESC);
