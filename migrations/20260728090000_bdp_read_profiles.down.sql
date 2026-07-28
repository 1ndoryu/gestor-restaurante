ALTER TABLE configuracion_restaurante
    DROP CONSTRAINT IF EXISTS configuracion_bdp_purchase_profile_positive,
    DROP CONSTRAINT IF EXISTS configuracion_bdp_catalog_price_type_range,
    DROP COLUMN IF EXISTS bdp_purchase_notes_profile_id,
    DROP COLUMN IF EXISTS bdp_catalog_price_type;
