/* [D18d] Datos fiscales del cliente para InvoiceOrder (BillingDetails).
 * nif/direccion/codigo_postal/poblacion vacíos por defecto: sin NIF no se factura con datos fiscales.
 * bdp_billing_details = false por defecto: BDP factura con los datos que ya tenga la comanda. */
ALTER TABLE clientes
    ADD COLUMN nif VARCHAR(20) NOT NULL DEFAULT '',
    ADD COLUMN direccion_fiscal TEXT NOT NULL DEFAULT '',
    ADD COLUMN codigo_postal VARCHAR(10) NOT NULL DEFAULT '',
    ADD COLUMN poblacion VARCHAR(100) NOT NULL DEFAULT '';

ALTER TABLE configuracion_restaurante
    ADD COLUMN bdp_billing_details BOOLEAN NOT NULL DEFAULT FALSE;
