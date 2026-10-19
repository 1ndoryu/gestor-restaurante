/* [F6.d] OrderEndType del CreateOrder configurable por restaurante.
 * 0 = comanda normal (facturable tras el pago); 1 = pendiente de validación
 * (BDP no la factura). Por defecto 0: con 1 BDP rechaza la factura (301300). */
ALTER TABLE configuracion_restaurante
    ADD COLUMN bdp_order_end_type INTEGER NOT NULL DEFAULT 0
        CHECK (bdp_order_end_type IN (0, 1));
