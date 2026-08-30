/* [198A-1/F1] Reversión de la corrección de ancho. Solo aplicable si no hay
 * filas con estados de más de 20 caracteres ('pendiente_suscripcion'). */

ALTER TABLE bdp_push_pendientes
    ALTER COLUMN estado TYPE VARCHAR(20);
