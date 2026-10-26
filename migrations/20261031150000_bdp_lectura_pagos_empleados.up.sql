/* [26AA-2] Lectura programada de formas de pago y empleados del BDP.
 *
 * Hasta ahora solo se consultaban a mano al abrir la pantalla. Aquí se guardan
 * los registros leídos (relación por identificador BDP, nunca por nombre) y la
 * cadencia por cuenta. Nace apagada (segundos = 0): encenderla cambia el ritmo
 * de llamadas al TPV y lo decide quien administra la cuenta.
 *
 * Es solo lectura sobre el TPV. Un registro que BDP deja de publicar pasa a
 * inactivo; no se borra ni se toca ningún mapeo (`bdp_tender_map`, empleado). */
CREATE TABLE IF NOT EXISTS bdp_lectura_programada (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    maestro TEXT NOT NULL CHECK (maestro IN ('formas_pago', 'empleados')),
    segundos INTEGER NOT NULL DEFAULT 0 CHECK (segundos >= 0),
    ultima_lectura TIMESTAMPTZ,
    ultima_ok TIMESTAMPTZ,
    ultimo_error TEXT,
    resumen TEXT,
    PRIMARY KEY (user_id, maestro)
);

CREATE TABLE IF NOT EXISTS bdp_formas_pago (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    bdp_id BIGINT NOT NULL,
    nombre TEXT NOT NULL DEFAULT '',
    tipo BIGINT,
    activo BOOLEAN NOT NULL DEFAULT TRUE,
    visto_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, bdp_id)
);

CREATE TABLE IF NOT EXISTS bdp_empleados (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    bdp_id BIGINT NOT NULL,
    nombre TEXT NOT NULL DEFAULT '',
    tipo BIGINT,
    activo BOOLEAN NOT NULL DEFAULT TRUE,
    visto_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, bdp_id)
);
