/* [157A-7] F9.1: Campos enriquecidos en bdp_article_map para sync completa de catálogo.
 * Estos campos almacenan datos del ExportArticles de BDP para evitar
 * llamadas repetidas al resolver artículos. */

ALTER TABLE bdp_article_map
    ADD COLUMN IF NOT EXISTS descripcion TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS precio_tarifa1 NUMERIC(12,4) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS iva_pct NUMERIC(6,2) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS departamento INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS familia INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS subfamilia INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS activo BOOLEAN NOT NULL DEFAULT true,
    ADD COLUMN IF NOT EXISTS barcode TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS ultima_sync_at TIMESTAMPTZ;
