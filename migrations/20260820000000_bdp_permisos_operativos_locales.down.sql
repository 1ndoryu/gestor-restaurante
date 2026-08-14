/* [128A-1/F8-1] Reversa: quita los permisos de operaciones locales. */
ALTER TABLE configuracion_restaurante
    DROP COLUMN IF EXISTS permisos_pagos_locales,
    DROP COLUMN IF EXISTS permisos_facturacion_local;
