DROP INDEX IF EXISTS uq_bdp_audit_create_vivo;

CREATE INDEX IF NOT EXISTS idx_bdp_id_map_marketplace
    ON bdp_id_map (user_id, entidad, market_id, marketplace_order_id)
    WHERE market_id IS NOT NULL;

DROP INDEX IF EXISTS uq_bdp_id_map_marketplace;
