/* [198A-1/F1] Escrituras BDP completas (independencia intacta).
 * Configuracion:
 *   - push_modalidad: disparador del push ('automatico' default | 'manual').
 *   - bdp_tav_map: mapeo IVA local (%) -> TAVCode BDP (JSONB), por M13.
 *   - bdp_almacen_default / bdp_codreg_default: almacén y motivo de
 *     regularización/traspaso (D5), defaults Store=1, CodReg=1.
 *   - bdp_articulo_rango_inicial: rango reservado de códigos de artículo (D3),
 *     default 90 000 000, <=13 dígitos.
 * Cola unidireccional Glory -> BDP (M18 aditiva, M19 concurrencia):
 *   UNIQUE parcial sobre las filas activas para una sola fila pendiente por
 *   (dominio, entidad, operacion); las históricas ('sincronizado'/'descartado')
 *   no bloquean nuevos reintentos. */

ALTER TABLE configuracion_restaurante
    ADD COLUMN IF NOT EXISTS push_modalidad VARCHAR(20) NOT NULL DEFAULT 'automatico'
        CHECK (push_modalidad IN ('automatico', 'manual')),
    ADD COLUMN IF NOT EXISTS bdp_tav_map JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS bdp_almacen_default INTEGER NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS bdp_codreg_default INTEGER NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS bdp_articulo_rango_inicial BIGINT NOT NULL DEFAULT 90000000;

CREATE TABLE IF NOT EXISTS bdp_push_pendientes (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    dominio VARCHAR(20) NOT NULL
        CHECK (dominio IN ('articulo', 'stock', 'departamento', 'familia', 'venta', 'cliente_puntos', 'propina')),
    /* Identificador local de la entidad: articulo_glory_codigo (TEXT), id de
     * venta (UUID como texto), código de cliente, etc. No se usa BIGINT porque
     * las entidades de Glory mezclan UUID y códigos alfanuméricos. */
    entidad_id TEXT NOT NULL,
    operacion VARCHAR(20) NOT NULL
        CHECK (operacion IN ('crear', 'modificar', 'precios', 'regularizar', 'traspasar', 'inventario', 'cancelar', 'puntos', 'propina')),
    payload_json JSONB NOT NULL,
    estado VARCHAR(20) NOT NULL DEFAULT 'pendiente'
        CHECK (estado IN ('pendiente', 'pendiente_suscripcion', 'error', 'sincronizado', 'descartado')),
    reintentos INTEGER NOT NULL DEFAULT 0,
    ultimo_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_bdp_push_pendientes_activos
    ON bdp_push_pendientes (user_id, dominio, entidad_id, operacion)
    WHERE estado IN ('pendiente', 'pendiente_suscripcion', 'error');

CREATE INDEX IF NOT EXISTS idx_bdp_push_pendientes_user_estado
    ON bdp_push_pendientes (user_id, estado);
