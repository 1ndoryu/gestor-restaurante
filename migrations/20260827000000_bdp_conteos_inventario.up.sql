/* [208A-2/C3] Persistencia local del conteo de inventario (decisiones D3/D4).
 * Aditiva (M15): solo crea tablas nuevas con defaults; no toca columnas
 * existentes.
 *   - bdp_conteos_inventario: un conteo fechado por sesión de recuento.
 *   - bdp_conteos_inventario_lineas: esperado/contado/diferencia por artículo,
 *     con el flag aplicado_al_stock para auditar el efecto local (D4).
 * La diferencia se aplica al stock local (bdp_article_stock) en la misma
 * transacción del guardado, con motivo 'conteo' y auditoría idempotente
 * (clave conteo:{id}:{codigo} en bdp_audit_log). */

CREATE TABLE IF NOT EXISTS bdp_conteos_inventario (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    fecha DATE NOT NULL DEFAULT CURRENT_DATE,
    observaciones TEXT NOT NULL DEFAULT '',
    estado VARCHAR(20) NOT NULL DEFAULT 'aplicado'
        CHECK (estado IN ('aplicado')),
    /* Clave de idempotencia: un mismo guardado no aplica dos veces (D4).
     * El cliente la genera por sesión de conteo y la reutiliza en reintentos. */
    idempotency_key TEXT,
    creado_el TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS bdp_conteos_inventario_user_key_idx
    ON bdp_conteos_inventario (user_id, idempotency_key)
    WHERE idempotency_key IS NOT NULL;

CREATE TABLE IF NOT EXISTS bdp_conteos_inventario_lineas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conteo_id UUID NOT NULL REFERENCES bdp_conteos_inventario(id) ON DELETE CASCADE,
    articulo_glory_codigo TEXT NOT NULL,
    esperado NUMERIC NOT NULL DEFAULT 0,
    contado NUMERIC NOT NULL DEFAULT 0,
    diferencia NUMERIC NOT NULL DEFAULT 0,
    aplicado_al_stock BOOLEAN NOT NULL DEFAULT false,
    UNIQUE (conteo_id, articulo_glory_codigo)
);

CREATE INDEX IF NOT EXISTS bdp_conteos_inventario_user_fecha_idx
    ON bdp_conteos_inventario (user_id, fecha DESC, creado_el DESC);
