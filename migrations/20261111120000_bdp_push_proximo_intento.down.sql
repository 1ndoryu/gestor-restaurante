DROP INDEX IF EXISTS idx_bdp_push_pendientes_espera;
ALTER TABLE bdp_push_pendientes DROP COLUMN IF EXISTS proximo_intento_at;
