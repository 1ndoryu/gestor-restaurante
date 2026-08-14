/* [128A-1/F3] Marca de stock ajustado localmente en bdp_article_stock.
 * El sync BDP (upsert_stock) NO sobrescribe filas con ajustado_local=true:
 * la fuente de verdad editable nunca se pisa por un import posterior. */

ALTER TABLE bdp_article_stock
    ADD COLUMN IF NOT EXISTS ajustado_local BOOLEAN NOT NULL DEFAULT false;
