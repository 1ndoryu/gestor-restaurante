/* [198A-1/F3-F7] Handlers locales de edición que faltaban (D3, D7, D8, D9).
 * Todo es aditivo y 100% operativo sin BDP (independencia): estas tablas son
 * la fuente de verdad local; el push a BDP se encola y el worker decide según
 * el modo operativo. */

/* D7: departamento/familia locales con código numérico secuencial (BDP pide int). */
CREATE TABLE IF NOT EXISTS bdp_catalogo_clasificaciones (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    tipo VARCHAR(20) NOT NULL
        CHECK (tipo IN ('departamento', 'familia')),
    code INTEGER NOT NULL CHECK (code BETWEEN 1 AND 999),
    nombre VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_clasificacion_nombre UNIQUE (user_id, tipo, nombre),
    CONSTRAINT uq_clasificacion_code UNIQUE (user_id, tipo, code)
);

CREATE INDEX IF NOT EXISTS idx_clasificaciones_user_tipo
    ON bdp_catalogo_clasificaciones (user_id, tipo);

/* D8: propina por venta (migración aditiva; default 0 preserva filas existentes). */
ALTER TABLE ventas
    ADD COLUMN IF NOT EXISTS propina NUMERIC(12,2) NOT NULL DEFAULT 0;

/* D9: ledger local de puntos de fidelización (saldo BDP es la fuente remota;
 * este ledger permite operar y consultar el saldo local sin BDP). */
CREATE TABLE IF NOT EXISTS bdp_puntos_cliente (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    cliente_id UUID NOT NULL,
    bdp_customer_code INTEGER NOT NULL,
    points_added NUMERIC(12,2) NOT NULL,
    reason VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_puntos_cliente
    ON bdp_puntos_cliente (user_id, cliente_id, created_at);
