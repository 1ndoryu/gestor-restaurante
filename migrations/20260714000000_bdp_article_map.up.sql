/* [F1.1] Tabla de mapeo artículos Glory → BDP.
 * Permite al usuario mapear códigos de artículo del POS BDP a conceptos Glory.
 * UNIQUE(user_id, articulo_glory_codigo) previene duplicados por usuario. */

CREATE TABLE IF NOT EXISTS bdp_article_map (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    articulo_glory_codigo TEXT NOT NULL,
    articulo_bdp_codigo TEXT NOT NULL,
    articulo_bdp_nombre TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, articulo_glory_codigo)
);

CREATE INDEX IF NOT EXISTS idx_bdp_article_map_user ON bdp_article_map(user_id);
