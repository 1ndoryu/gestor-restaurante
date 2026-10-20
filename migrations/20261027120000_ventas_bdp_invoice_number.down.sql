DROP INDEX IF EXISTS uq_ventas_bdp_invoice_number;
ALTER TABLE ventas DROP COLUMN IF EXISTS bdp_invoice_number;
