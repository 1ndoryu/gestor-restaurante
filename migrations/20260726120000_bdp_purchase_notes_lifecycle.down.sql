/* [247A-12] Rollback: eliminar columnas de ciclo de vida de compras BDP. */

DROP INDEX IF EXISTS idx_bdp_purchase_notes_gasto;
DROP INDEX IF EXISTS idx_bdp_purchase_notes_estado;

ALTER TABLE bdp_purchase_notes
DROP COLUMN IF EXISTS estado,
DROP COLUMN IF EXISTS gasto_id;
