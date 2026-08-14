/* [128A-1/F8-1] Permisos operativos para las variantes LOCALES de F6
 * (pagos parciales y facturación local): con el default 'admin' un Trabajador
 * no puede registrar pagos parciales ni emitir facturas locales (operaciones
 * monetarias) sin permiso explícito. Misma semántica que la migración F8:
 * 'admin' (default) | 'admin_trabajador' | 'todos', con CHECK en BD y
 * defaults aditivos (M15) que no alteran filas existentes. */

ALTER TABLE configuracion_restaurante
    ADD COLUMN IF NOT EXISTS permisos_pagos_locales VARCHAR(20) NOT NULL DEFAULT 'admin'
        CHECK (permisos_pagos_locales IN ('admin', 'admin_trabajador', 'todos')),
    ADD COLUMN IF NOT EXISTS permisos_facturacion_local VARCHAR(20) NOT NULL DEFAULT 'admin'
        CHECK (permisos_facturacion_local IN ('admin', 'admin_trabajador', 'todos'));
