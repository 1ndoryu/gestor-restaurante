/* [D25] Permiso explícito para CreateCustomer con Overwrite=true sobre clientes ya vinculados.
 * FALSE por defecto: sin aprobación de la usuaria el cliente vinculado no se sobrescribe.
 * Solo tiene efecto con BDP en loopback (BDP local). */
ALTER TABLE configuracion_restaurante
    ADD COLUMN bdp_overwrite_clientes BOOLEAN NOT NULL DEFAULT FALSE;
