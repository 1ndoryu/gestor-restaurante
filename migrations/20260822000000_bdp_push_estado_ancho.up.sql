/* [198A-1/F1] Corrección aditiva: 'pendiente_suscripcion' mide 21 caracteres
 * y la columna `estado` original era VARCHAR(20), por lo que marcar el bloqueo
 * por suscripción fallaba con "valor demasiado largo". Se ensancha sin tocar la
 * migración ya aplicada (inmutabilidad M18). */

ALTER TABLE bdp_push_pendientes
    ALTER COLUMN estado TYPE VARCHAR(30);
