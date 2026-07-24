/* [237A-4] Campo stock_actual en bdp_article_map.
 * Se rellena durante sync-catalog si ExportArticles devuelve CurrentStock
 * en la respuesta de PricesTableDataType. Solo lectura — no se puede
 * modificar stock desde Glory. */

ALTER TABLE bdp_article_map
    ADD COLUMN IF NOT EXISTS stock_actual NUMERIC(14,4) NOT NULL DEFAULT 0;
