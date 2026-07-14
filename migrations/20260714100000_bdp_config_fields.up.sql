/* [F1.2] Campos nuevos en configuración para mapeos BDP.
 * bdp_tender_map: mapeo método_pago Glory → código tend BDP (JSONB).
 * bdp_order_type_map: mapeo canal_venta Glory → código tipo pedido BDP (JSONB).
 * bdp_default_customer_code: código cliente BDP por defecto (para ventas sin cliente). */

ALTER TABLE configuracion_restaurante
    ADD COLUMN IF NOT EXISTS bdp_tender_map JSONB NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS bdp_order_type_map JSONB NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS bdp_default_customer_code TEXT NOT NULL DEFAULT '';
