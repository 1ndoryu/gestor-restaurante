/* [128A-1/F4] Reversa: quita campos de anulación y el índice parcial. */
ALTER TABLE configuracion_restaurante
    DROP COLUMN IF EXISTS anulacion_modalidad;

DROP INDEX IF EXISTS idx_ventas_anulada;

ALTER TABLE ventas
    DROP COLUMN IF EXISTS anulacion_usuario,
    DROP COLUMN IF EXISTS anulacion_motivo,
    DROP COLUMN IF EXISTS anulada_at,
    DROP COLUMN IF EXISTS anulada;
