/* [D24-auto-2] Cadencias de los tres maestros que seguían sin bucle: clientes,
 * plano de sala y departamentos. La lectura de artículos ya tenía la suya
 * (`bdp_maestros_auto_segundos`); estas tres tablas del alcance prometen
 * «automático» y hasta ahora solo se refrescaban pulsando un botón.
 *
 * Todas nacen en 0 (apagadas): encender una lectura periódica cambia el ritmo
 * de llamadas a BDP de un restaurante que ya está en marcha, y eso lo decide
 * quien lo administra, no una migración. La cadencia recomendada es la misma
 * que los artículos (900 s) para que el conjunto se comporte como una unidad. */
ALTER TABLE configuracion_restaurante
    ADD COLUMN IF NOT EXISTS bdp_clientes_auto_segundos INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS bdp_clientes_ultima_lectura TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS bdp_plano_auto_segundos INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS bdp_plano_ultima_lectura TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS bdp_departamentos_auto_segundos INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS bdp_departamentos_ultima_lectura TIMESTAMPTZ;

/* Un valor negativo no significa nada: el barrido lo leería como «vencido
 * siempre» y llamaría a BDP en cada pasada. */
ALTER TABLE configuracion_restaurante
    DROP CONSTRAINT IF EXISTS ck_bdp_cadencias_maestros;
ALTER TABLE configuracion_restaurante
    ADD CONSTRAINT ck_bdp_cadencias_maestros CHECK (
        bdp_clientes_auto_segundos >= 0
        AND bdp_plano_auto_segundos >= 0
        AND bdp_departamentos_auto_segundos >= 0
    );
