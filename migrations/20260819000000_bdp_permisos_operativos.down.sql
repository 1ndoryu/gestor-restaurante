/* [128A-1/F8] Reversa: quita los permisos operativos por acción. */
ALTER TABLE configuracion_restaurante
    DROP COLUMN IF EXISTS permisos_catalogo_edicion,
    DROP COLUMN IF EXISTS permisos_stock_ajuste,
    DROP COLUMN IF EXISTS permisos_albaranes_gestion,
    DROP COLUMN IF EXISTS permisos_anulacion_ventas;
