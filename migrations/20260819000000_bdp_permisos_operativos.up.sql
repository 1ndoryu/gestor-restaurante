/* [128A-1/F8] Permisos operativos por acción (D8, M17).
 *
 * Cada columna define quién puede ejecutar la acción sensible en backend:
 *   - 'admin' (default): solo el propietario (rol Admin).
 *   - 'admin_trabajador': Admin y Trabajador (todo el staff autenticado).
 *   - 'todos': cualquier usuario autenticado.
 *
 * Valores con CHECK para impedir configuraciones inválidas en la BD
 * (la API también valida en el handler). Migración aditiva (M15): con
 * defaults, las filas existentes quedan en 'admin' sin tocar datos previos. */

ALTER TABLE configuracion_restaurante
    ADD COLUMN IF NOT EXISTS permisos_catalogo_edicion VARCHAR(20) NOT NULL DEFAULT 'admin'
        CHECK (permisos_catalogo_edicion IN ('admin', 'admin_trabajador', 'todos')),
    ADD COLUMN IF NOT EXISTS permisos_stock_ajuste VARCHAR(20) NOT NULL DEFAULT 'admin'
        CHECK (permisos_stock_ajuste IN ('admin', 'admin_trabajador', 'todos')),
    ADD COLUMN IF NOT EXISTS permisos_albaranes_gestion VARCHAR(20) NOT NULL DEFAULT 'admin'
        CHECK (permisos_albaranes_gestion IN ('admin', 'admin_trabajador', 'todos')),
    ADD COLUMN IF NOT EXISTS permisos_anulacion_ventas VARCHAR(20) NOT NULL DEFAULT 'admin'
        CHECK (permisos_anulacion_ventas IN ('admin', 'admin_trabajador', 'todos'));
