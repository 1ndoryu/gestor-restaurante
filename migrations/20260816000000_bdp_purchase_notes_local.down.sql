DROP INDEX IF EXISTS idx_bdp_purchase_notes_user_origen;
ALTER TABLE bdp_purchase_notes DROP COLUMN IF EXISTS origen;
