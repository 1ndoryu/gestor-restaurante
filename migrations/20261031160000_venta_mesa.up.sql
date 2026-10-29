/* Mesa real de la venta web: la orden BDP sale como Type 1 (Mesa) con su salón
 * y su mesa. `num_mesa` y `num_salon` son la foto de los códigos BDP en el momento
 * de crear la venta; así el sync no depende del plano si la mesa cambia después. */
ALTER TABLE ventas
    ADD COLUMN IF NOT EXISTS mesa_id UUID REFERENCES mesas(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS num_mesa INTEGER,
    ADD COLUMN IF NOT EXISTS num_salon INTEGER;
