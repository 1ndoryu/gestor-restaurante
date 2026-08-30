/* [198A-1/F1] Rollback aditivo de las escrituras BDP: se eliminan la cola y
 * las columnas de configuración añadidas por la migración `.up`. No altera
 * ninguna tabla preexistente ni sus datos. */

DROP TABLE IF EXISTS bdp_push_pendientes;

ALTER TABLE configuracion_restaurante
    DROP COLUMN IF EXISTS push_modalidad,
    DROP COLUMN IF EXISTS bdp_tav_map,
    DROP COLUMN IF EXISTS bdp_almacen_default,
    DROP COLUMN IF EXISTS bdp_codreg_default,
    DROP COLUMN IF EXISTS bdp_articulo_rango_inicial;
