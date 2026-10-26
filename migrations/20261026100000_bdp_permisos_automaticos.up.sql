/* [26AA-3] Permisos del modo automático por categoría.
 *
 * El modo `automatic` escribía en el TPV todas las categorías a la vez. Aquí
 * cada categoría (comandas, artículos y clientes, propinas y puntos, pagos,
 * facturas, stock, camarero) tiene su propio interruptor. Nace apagado: sin
 * fila la categoría está apagada, así que una cuenta nueva no escribe nada.
 *
 * `bdp_permiso_automatico_historial` deja constancia de cada cambio con su fecha. */
CREATE TABLE IF NOT EXISTS bdp_permiso_automatico (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    categoria TEXT NOT NULL CHECK (categoria IN (
        'comandas', 'articulos_clientes', 'propinas_puntos',
        'pagos', 'facturas', 'stock', 'camarero'
    )),
    activo BOOLEAN NOT NULL DEFAULT FALSE,
    actualizado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, categoria)
);

CREATE TABLE IF NOT EXISTS bdp_permiso_automatico_historial (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    categoria TEXT NOT NULL,
    activo BOOLEAN NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_bdp_permiso_historial_user
    ON bdp_permiso_automatico_historial(user_id, created_at DESC);
