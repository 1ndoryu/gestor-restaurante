/* Armado temporal y acotado de escrituras BDP. Una fila por usuario. */
CREATE TABLE IF NOT EXISTS bdp_write_arming (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    base_url TEXT NOT NULL,
    scopes TEXT[] NOT NULL,
    target_entity_type TEXT NOT NULL CHECK (target_entity_type IN ('venta', 'cliente')),
    target_entity_id UUID NOT NULL,
    reason TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    remaining_operations INTEGER NOT NULL CHECK (remaining_operations BETWEEN 0 AND 10),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (cardinality(scopes) > 0)
);

CREATE INDEX IF NOT EXISTS idx_bdp_write_arming_expiry
    ON bdp_write_arming(expires_at);

CREATE INDEX IF NOT EXISTS idx_bdp_write_arming_target
    ON bdp_write_arming(user_id, target_entity_type, target_entity_id);
