/* [039A-1/Q1] Reversión de la corrección de ancho. Solo aplicable si no hay
 * filas con `tipo` compuesto de más de 50 caracteres (snapshots parciales
 * multi-tipo). */

ALTER TABLE bdp_snapshots
    ALTER COLUMN tipo TYPE VARCHAR(50);