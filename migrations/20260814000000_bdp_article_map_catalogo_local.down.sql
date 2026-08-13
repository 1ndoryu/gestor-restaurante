-- [128A-1/F2] Reversa: quita origen/local_dirty e índice del catálogo local.
ALTER TABLE bdp_article_map DROP COLUMN IF EXISTS local_dirty;
ALTER TABLE bdp_article_map DROP COLUMN IF EXISTS origen;
DROP INDEX IF EXISTS idx_bdp_article_map_user_origen;
