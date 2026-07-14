ALTER TABLE configuracion_restaurante
    DROP COLUMN IF EXISTS bdp_tender_map,
    DROP COLUMN IF EXISTS bdp_order_type_map,
    DROP COLUMN IF EXISTS bdp_default_customer_code;
