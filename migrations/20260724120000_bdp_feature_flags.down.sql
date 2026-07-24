ALTER TABLE configuracion_restaurante
    DROP COLUMN IF EXISTS ff_bdp_auto_arm,
    DROP COLUMN IF EXISTS ff_bdp_partial_payments,
    DROP COLUMN IF EXISTS ff_bdp_cancel_order,
    DROP COLUMN IF EXISTS ff_bdp_purchase_notes_read,
    DROP COLUMN IF EXISTS ff_bdp_purchase_notes_draft,
    DROP COLUMN IF EXISTS ff_bdp_purchase_notes_receive;
