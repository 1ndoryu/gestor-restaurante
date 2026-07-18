DROP INDEX IF EXISTS idx_bdp_audit_target;
DROP INDEX IF EXISTS idx_bdp_snapshots_target;

ALTER TABLE bdp_write_arming
    DROP CONSTRAINT IF EXISTS bdp_write_arming_scopes_safe;

ALTER TABLE bdp_audit_log
    DROP COLUMN IF EXISTS updated_at,
    DROP COLUMN IF EXISTS target_entity_id,
    DROP COLUMN IF EXISTS target_entity_type,
    DROP COLUMN IF EXISTS target_base_url;

ALTER TABLE bdp_write_arming
    DROP COLUMN IF EXISTS connection_fingerprint,
    DROP COLUMN IF EXISTS snapshot_id;

ALTER TABLE bdp_snapshots
    DROP COLUMN IF EXISTS connection_fingerprint,
    DROP COLUMN IF EXISTS target_base_url;
