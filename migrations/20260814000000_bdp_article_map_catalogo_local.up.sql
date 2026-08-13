/* [128A-1/F2] Catálogo local — C4: migración aditiva mínima sobre bdp_article_map.
 * Semántica (M5): la tabla pasa a ser "artículos del catálogo + mapeo Glory↔BDP".
 * UNIQUE(user_id, articulo_glory_codigo) se mantiene como identidad local.
 *
 * `origen` indica la procedencia del registro:
 *   - 'bdp'    (default): importado/sincronizado desde BDP (filas existentes).
 *   - 'local'  : creado o editado localmente en la Aplicación Web.
 * `local_dirty` marca filas editadas localmente: el import BDP (M6) no las
 * sobrescribe y reporta el conflicto; el desactivado local (M7) se conserva.
 */

ALTER TABLE bdp_article_map
    ADD COLUMN IF NOT EXISTS origen VARCHAR(10) NOT NULL DEFAULT 'bdp'
        CHECK (origen IN ('local', 'bdp')),
    ADD COLUMN IF NOT EXISTS local_dirty BOOLEAN NOT NULL DEFAULT false;

CREATE INDEX IF NOT EXISTS idx_bdp_article_map_user_origen
    ON bdp_article_map(user_id, origen);
