/* Revertir: restaura las CHECK restrictivas originales de bdp_write_arming. */

ALTER TABLE bdp_write_arming
    DROP CONSTRAINT IF EXISTS bdp_write_arming_scopes_safe;

ALTER TABLE bdp_write_arming
    ADD CONSTRAINT bdp_write_arming_scopes_safe CHECK (
        cardinality(scopes) = 1
        AND scopes <@ ARRAY['create_order', 'create_customer', 'add_payment', 'invoice']::TEXT[]
    );

ALTER TABLE bdp_write_arming
    DROP CONSTRAINT IF EXISTS bdp_write_arming_target_entity_type_check;

ALTER TABLE bdp_write_arming
    ADD CONSTRAINT bdp_write_arming_target_entity_type_check CHECK (
        target_entity_type IN ('venta', 'cliente')
    );
