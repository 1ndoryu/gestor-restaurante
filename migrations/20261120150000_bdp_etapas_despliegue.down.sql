/* [21AB-5] Vuelta atrás: fuera el índice de modificaciones y las categorías
 * nuevas. Las filas de `modificaciones`/`descuentos`/`lectura` se borran antes
 * de reponer el CHECK, que si no las rechaza. */
DROP INDEX IF EXISTS uq_bdp_audit_modify_vivo;

DELETE FROM bdp_permiso_automatico_historial
 WHERE categoria IN ('lectura', 'modificaciones', 'descuentos');
DELETE FROM bdp_permiso_automatico
 WHERE categoria IN ('lectura', 'modificaciones', 'descuentos');

ALTER TABLE bdp_permiso_automatico
    DROP CONSTRAINT IF EXISTS bdp_permiso_automatico_categoria_check;

ALTER TABLE bdp_permiso_automatico
    ADD CONSTRAINT bdp_permiso_automatico_categoria_check CHECK (categoria IN (
        'comandas', 'articulos_clientes', 'propinas_puntos',
        'pagos', 'facturas', 'stock', 'camarero'
    ));
