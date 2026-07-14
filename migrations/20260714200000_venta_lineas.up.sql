/* [F2.1] Tabla de líneas de venta (items individuales).
 * Permite desglosar una venta en múltiples artículos.
 * Si una venta no tiene líneas, se usa el comportamiento legacy (1 artículo genérico).
 * ON DELETE CASCADE: al borrar la venta, se borran sus líneas. */

CREATE TABLE IF NOT EXISTS venta_lineas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    venta_id UUID NOT NULL REFERENCES ventas(id) ON DELETE CASCADE,
    articulo_codigo TEXT NOT NULL DEFAULT '',
    descripcion TEXT NOT NULL,
    cantidad DECIMAL(10,3) NOT NULL DEFAULT 1,
    precio_unitario DECIMAL(10,2) NOT NULL DEFAULT 0,
    iva_pct DECIMAL(5,2) NOT NULL DEFAULT 0,
    descuento DECIMAL(10,2) NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_venta_lineas_venta ON venta_lineas(venta_id);
