/* [198A-1/F3-F7] Reversión aditiva. */

DROP TABLE IF EXISTS bdp_puntos_cliente;
ALTER TABLE ventas DROP COLUMN IF EXISTS propina;
DROP TABLE IF EXISTS bdp_catalogo_clasificaciones;
