/* [287A-5] La unicidad general (user_id, bdp_order_id) ya impide duplicados
 * facturados. El índice parcial adicional es redundante y se elimina en una
 * migración nueva para no alterar el checksum de una migración aplicada. */
DROP INDEX IF EXISTS idx_ventas_bdp_invoiced_order_unique;
