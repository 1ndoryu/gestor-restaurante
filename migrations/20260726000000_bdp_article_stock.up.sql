/* [247A-10/S2] Stock por almacén (solo lectura).
 * La API actual de BDP (ExportArticles) solo devuelve un stock agregado por
 * artículo. Esta tabla prepara el modelo para cuando BDP exponga stock por
 * almacén, guardando por defecto un único almacén "General".
 * El stock sigue siendo solo lectura desde Glory. */

CREATE TABLE IF NOT EXISTS bdp_article_stock (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    articulo_glory_codigo TEXT NOT NULL,
    warehouse_id TEXT NOT NULL DEFAULT '0',
    warehouse_name TEXT NOT NULL DEFAULT 'General',
    stock NUMERIC(14,4) NOT NULL DEFAULT 0,
    ultima_sync_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, articulo_glory_codigo, warehouse_id)
);

CREATE INDEX IF NOT EXISTS idx_bdp_article_stock_user ON bdp_article_stock(user_id);
CREATE INDEX IF NOT EXISTS idx_bdp_article_stock_user_article ON bdp_article_stock(user_id, articulo_glory_codigo);
