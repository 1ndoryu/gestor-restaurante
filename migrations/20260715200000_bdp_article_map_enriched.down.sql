/* Revertir campos enriquecidos de bdp_article_map */

ALTER TABLE bdp_article_map
    DROP COLUMN IF EXISTS descripcion,
    DROP COLUMN IF EXISTS precio_tarifa1,
    DROP COLUMN IF EXISTS iva_pct,
    DROP COLUMN IF EXISTS departamento,
    DROP COLUMN IF EXISTS familia,
    DROP COLUMN IF EXISTS subfamilia,
    DROP COLUMN IF EXISTS activo,
    DROP COLUMN IF EXISTS barcode,
    DROP COLUMN IF EXISTS ultima_sync_at;
