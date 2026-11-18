/* [18AB-4] Reversión de la migración de invitación por línea. */

ALTER TABLE venta_lineas
    DROP COLUMN IF EXISTS invitacion;

ALTER TABLE bdp_article_map
    DROP COLUMN IF EXISTS invitacion_prohibida;
