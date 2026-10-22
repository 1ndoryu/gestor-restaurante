ALTER TABLE configuracion_restaurante
    DROP CONSTRAINT IF EXISTS ck_bdp_documentos_auto,
    DROP COLUMN IF EXISTS bdp_documentos_perfil,
    DROP COLUMN IF EXISTS bdp_documentos_auto_segundos;
