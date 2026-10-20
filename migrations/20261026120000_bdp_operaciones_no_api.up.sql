/* [D32] Responsable y procedimiento por operación que BDP no expone por API.
 * Sin fila, la operación usa el responsable global de la configuración. */
CREATE TABLE IF NOT EXISTS bdp_operaciones_no_api (
    user_id UUID NOT NULL,
    codigo TEXT NOT NULL CHECK (codigo <> ''),
    responsable TEXT NOT NULL DEFAULT '',
    procedimiento TEXT NOT NULL DEFAULT '',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, codigo)
);
