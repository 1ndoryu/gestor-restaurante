/* [D28-auto-R/D32] La lectura automática de documentos de venta deja de vivir en
 * el entorno. Antes solo arrancaba con `BDP_DOCUMENTOS_AUTO_SEGUNDOS` y
 * `BDP_DOCUMENTOS_PERFIL`, que nadie puede tocar desde la web: en la práctica
 * quedaba siempre apagada, sin perfil y sin interfaz. Ahora la cadencia y el
 * perfil son configuración del restaurante. `0` significa «no leer sola»; un
 * perfil a `NULL` la mantiene apagada aunque la cadencia sea mayor que 0. */
ALTER TABLE configuracion_restaurante
    ADD COLUMN IF NOT EXISTS bdp_documentos_auto_segundos INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS bdp_documentos_perfil INTEGER;

/* Sin perfil válido el disparo no puede hacer nada: se descartan los valores de
 * relleno que dejara un guardado incompleto en vez de arrancar con un perfil 0. */
ALTER TABLE configuracion_restaurante
    DROP CONSTRAINT IF EXISTS ck_bdp_documentos_auto;
ALTER TABLE configuracion_restaurante
    ADD CONSTRAINT ck_bdp_documentos_auto CHECK (
        bdp_documentos_auto_segundos >= 0
        AND (bdp_documentos_perfil IS NULL OR bdp_documentos_perfil > 0));
