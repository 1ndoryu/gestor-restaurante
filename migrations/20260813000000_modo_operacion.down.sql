/* [128A-1/F1] Rollback del conmutador de modo operativo. */
ALTER TABLE configuracion_restaurante
    DROP COLUMN IF EXISTS modo_operacion;
