/* [10AA-3/D20] Número de factura de BDP en la propia venta.
 *
 * Hasta ahora el `InvoiceNumber` que devuelve BDP solo vivía en
 * `bdp_audit_log.datos_respuesta`, así que el worker de avisos no tenía contra
 * qué cruzar el aviso `finalizar` (que trae `Serie`+`Numero` del documento
 * emitido en caja) y D20 no se podía comprobar. Persistirlo aquí es lo que
 * convierte el aviso y el documento en una sola venta.
 *
 * Nace vacío: no migra datos del ledger de auditoría, que sigue siendo la
 * fuente de la traza de la llamada. */

ALTER TABLE ventas ADD COLUMN IF NOT EXISTS bdp_invoice_number TEXT;

/* D18 (número único): dos ventas no pueden reclamar el mismo documento de BDP.
 * Parcial porque la mayoría de ventas no se factura y en PostgreSQL los NULL no
 * chocan entre sí, pero el índice explícito deja la intención escrita. */
CREATE UNIQUE INDEX IF NOT EXISTS uq_ventas_bdp_invoice_number
    ON ventas (user_id, bdp_invoice_number)
    WHERE bdp_invoice_number IS NOT NULL;
