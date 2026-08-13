/* [128A-1/F6] Reversa: quita factura local y el origen de operación. */
DROP INDEX IF EXISTS uq_ventas_user_factura_numero;

ALTER TABLE ventas
    DROP COLUMN IF EXISTS factura_fecha,
    DROP COLUMN IF EXISTS factura_numero,
    DROP COLUMN IF EXISTS facturada_local;

DROP INDEX IF EXISTS idx_bdp_audit_user_origen;

ALTER TABLE bdp_audit_log
    DROP COLUMN IF EXISTS origen_operacion;
