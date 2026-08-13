/* [128A-1/F6] Auditoría local + factura local mínima (A11, A7/D9).
 *
 * `bdp_audit_log.origen_operacion` distingue operaciones locales puras
 * ('local': anulaciones, ajustes de stock, CRUD local, pagos/facturas
 * locales) de las que implican al BDP ('bdp', default — no altera filas
 * existentes). El Historial puede mostrarse sin BDP con origen 'local'.
 *
 * Factura local mínima (D9, default implementar): estado `facturada` sobre
 * `ventas` mediante `facturada_local` (final, transición única) + numeración
 * local secuencial `F-{año}-{n}` en `factura_numero` + `factura_fecha`.
 * El índice parcial UNIQUE(user_id, factura_numero) evita números duplicados
 * por usuario en carreras concurrentes.
 */

ALTER TABLE bdp_audit_log
    ADD COLUMN IF NOT EXISTS origen_operacion VARCHAR(10) NOT NULL DEFAULT 'bdp'
        CHECK (origen_operacion IN ('local', 'bdp'));

CREATE INDEX IF NOT EXISTS idx_bdp_audit_user_origen
    ON bdp_audit_log(user_id, origen_operacion, created_at DESC);

ALTER TABLE ventas
    ADD COLUMN IF NOT EXISTS facturada_local BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS factura_numero VARCHAR(50),
    ADD COLUMN IF NOT EXISTS factura_fecha TIMESTAMPTZ;

CREATE UNIQUE INDEX IF NOT EXISTS uq_ventas_user_factura_numero
    ON ventas(user_id, factura_numero)
    WHERE factura_numero IS NOT NULL;
