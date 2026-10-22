/* [D24-auto] Lectura programada del catálogo BDP (artículos, precios y stock
 * agregado). Antes solo se hacía pulsando «Importar del BDP». `0` = no leer
 * sola: encenderla por defecto cambiaría el comportamiento de un restaurante
 * ya en marcha sin que nadie lo pida, así que se enciende explícitamente y la
 * cadencia recomendada es 900 s (15 min). */
ALTER TABLE configuracion_restaurante
    ADD COLUMN IF NOT EXISTS bdp_maestros_auto_segundos INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS bdp_maestros_ultima_lectura TIMESTAMPTZ;
ALTER TABLE configuracion_restaurante
    DROP CONSTRAINT IF EXISTS ck_bdp_maestros_auto;
ALTER TABLE configuracion_restaurante
    ADD CONSTRAINT ck_bdp_maestros_auto CHECK (bdp_maestros_auto_segundos >= 0);
