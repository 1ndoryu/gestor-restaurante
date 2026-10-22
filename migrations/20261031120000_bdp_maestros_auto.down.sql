ALTER TABLE configuracion_restaurante
    DROP CONSTRAINT IF EXISTS ck_bdp_maestros_auto,
    DROP COLUMN IF EXISTS bdp_maestros_ultima_lectura,
    DROP COLUMN IF EXISTS bdp_maestros_auto_segundos;
