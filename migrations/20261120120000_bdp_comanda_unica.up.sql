/* [18AB-5] Una comanda de BDP no puede reclamarla dos entidades de Glory.
 * `MarketId` + `MarketplaceOrderId` son la identidad que BDP usa para deduplicar
 * (punto 1 de la respuesta de Guillermo del 18/11); el índice de 20261017 era
 * solo de consulta y permitía dos filas con el mismo par. Parcial: las filas
 * del mapeo que no son de comanda no llevan marketplace. */
CREATE UNIQUE INDEX IF NOT EXISTS uq_bdp_id_map_marketplace
    ON bdp_id_map (user_id, entidad, market_id, marketplace_order_id)
    WHERE market_id IS NOT NULL AND marketplace_order_id IS NOT NULL;

/* El índice de consulta queda cubierto por el único (mismo prefijo). */
DROP INDEX IF EXISTS idx_bdp_id_map_marketplace;

/* Una venta solo puede tener una intención de `create_order` viva a la vez: si
 * dos procesos llegan juntos, el segundo choca aquí en vez de mandar la
 * comanda otra vez. Parcial sobre `pendiente`/`ambiguo` porque las cerradas se
 * acumulan con el tiempo. */
CREATE UNIQUE INDEX IF NOT EXISTS uq_bdp_audit_create_vivo
    ON bdp_audit_log (user_id, target_entity_type, target_entity_id)
    WHERE operacion = 'create_order' AND resultado IN ('pendiente', 'ambiguo');
