DROP INDEX IF EXISTS idx_bdp_audit_idempotency;
ALTER TABLE bdp_audit_log DROP COLUMN IF EXISTS idempotency_key;
