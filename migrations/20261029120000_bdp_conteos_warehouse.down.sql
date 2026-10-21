/* [L7] Rollback: el conteo vuelve a no tener almacén propio. */

ALTER TABLE bdp_conteos_inventario
    DROP COLUMN IF EXISTS warehouse_id;
