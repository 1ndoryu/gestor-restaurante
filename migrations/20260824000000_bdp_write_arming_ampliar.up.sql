/* [198A-1/F1] Amplía las CHECK de `bdp_write_arming` para admitir las escrituras
 * Glory -> BDP nuevas del plan de escrituras completas.
 *
 * Las CHECK originales (20260718000000 / 20260718300000) solo permitían
 *   - target_entity_type IN ('venta', 'cliente')
 *   - scopes <@ ARRAY['create_order', 'create_customer', 'add_payment', 'invoice']
 * El worker de push (`BdpPushFlushService` -> `BdpWriteGuard::armar_push`) usa
 * `target_entity_type` = dominio ('articulo', 'stock', 'departamento', 'familia',
 * 'propina', 'cliente_puntos') y scopes ('create_article', 'modify_article', ...),
 * que violaban esas restricciones y bloqueaban el arming del camino feliz.
 *
 * Migración aditiva: se amplían ambas listas sin tocar los datos existentes. */

ALTER TABLE bdp_write_arming
    DROP CONSTRAINT IF EXISTS bdp_write_arming_scopes_safe;

ALTER TABLE bdp_write_arming
    ADD CONSTRAINT bdp_write_arming_scopes_safe CHECK (
        cardinality(scopes) = 1
        AND scopes <@ ARRAY[
            'create_order', 'create_customer', 'add_payment', 'invoice',
            'create_article', 'modify_article', 'modify_prices',
            'create_department', 'create_family',
            'regularize_stock', 'transfer_stock', 'inventory',
            'cancel_order', 'add_tip', 'add_points'
        ]::TEXT[]
    );

ALTER TABLE bdp_write_arming
    DROP CONSTRAINT IF EXISTS bdp_write_arming_target_entity_type_check;

ALTER TABLE bdp_write_arming
    ADD CONSTRAINT bdp_write_arming_target_entity_type_check CHECK (
        target_entity_type IN (
            'venta', 'cliente', 'articulo', 'stock', 'departamento',
            'familia', 'propina', 'cliente_puntos'
        )
    );
