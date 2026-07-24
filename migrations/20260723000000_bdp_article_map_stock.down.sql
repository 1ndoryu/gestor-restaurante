/* [237A-4] Revertir campo stock_actual. */
ALTER TABLE bdp_article_map
    DROP COLUMN IF EXISTS stock_actual;
