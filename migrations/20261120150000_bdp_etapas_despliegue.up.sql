/* [21AB-5] Etapas de la puesta en marcha, como las pidió Guillermo: lectura,
 * comandas, modificaciones, descuentos, pagos y tickets, cada una con su
 * interruptor.
 *
 * `lectura` y `descuentos` no tienen alcance de escritura propio (las lecturas
 * no pasan por el guard y los descuentos viajan en el payload de la comanda),
 * pero son etapas igual: la primera se consulta antes de tocar el TPV y la
 * segunda en la frontera del envío.
 *
 * Las filas existentes no se tocan: `comandas` sigue gobernando creación y
 * anulación, y `facturas` los tickets. Solo se añade lo nuevo, apagado. */
ALTER TABLE bdp_permiso_automatico
    DROP CONSTRAINT IF EXISTS bdp_permiso_automatico_categoria_check;

ALTER TABLE bdp_permiso_automatico
    ADD CONSTRAINT bdp_permiso_automatico_categoria_check CHECK (categoria IN (
        'lectura', 'comandas', 'modificaciones', 'descuentos',
        'pagos', 'facturas',
        'articulos_clientes', 'propinas_puntos', 'stock', 'camarero'
    ));

/* [21AB-5] La modificación deja de compartir alcance con la creación: su
 * auditoría va como `modify_order`. El único de «una creación viva por venta»
 * tiene que seguir cubriendo solo `create_order`; si incluyera `modify_order`,
 * modificar una comanda ya creada chocaría contra su propia creación cerrada
 * o contra otra modificación legítima de la misma venta. */
CREATE UNIQUE INDEX IF NOT EXISTS uq_bdp_audit_modify_vivo
    ON bdp_audit_log (user_id, target_entity_type, target_entity_id)
    WHERE operacion = 'modify_order' AND resultado IN ('pendiente', 'ambiguo');
