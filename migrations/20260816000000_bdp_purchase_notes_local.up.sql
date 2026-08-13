/* [128A-1/F5] Compras locales — origen y series reservadas (M18).
 * `origen` indica la procedencia del albarán:
 *   - 'bdp'   (default): importado/sincronizado desde BDP (filas existentes).
 *   - 'local' : creado/administrado en la Aplicación Web sin BDP.
 * Las líneas de los albaranes locales se guardan en `datos_bdp` (JSONB) bajo
 * la clave "lineas" para no añadir tablas nuevas. Las series locales usan el
 * prefijo reservado `L-...` para no chocar con el UNIQUE(user_id, serie, numero)
 * de los albaranes importados de BDP (M18).
 */

ALTER TABLE bdp_purchase_notes
    ADD COLUMN IF NOT EXISTS origen VARCHAR(10) NOT NULL DEFAULT 'bdp'
        CHECK (origen IN ('local', 'bdp'));

CREATE INDEX IF NOT EXISTS idx_bdp_purchase_notes_user_origen
    ON bdp_purchase_notes(user_id, origen);
