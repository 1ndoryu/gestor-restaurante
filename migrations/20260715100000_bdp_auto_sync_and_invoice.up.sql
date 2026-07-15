/* [F7.5] Flag para auto-sync de clientes Glory→BDP al crear ventas.
 * Default FALSE — el usuario debe habilitarlo explícitamente (requiere autorización). */
ALTER TABLE configuracion_restaurante ADD COLUMN bdp_auto_sync_customers BOOLEAN NOT NULL DEFAULT FALSE;

/* [F8.4] Flag para marcar ventas facturadas en BDP. */
ALTER TABLE ventas ADD COLUMN bdp_invoiced BOOLEAN NOT NULL DEFAULT FALSE;
