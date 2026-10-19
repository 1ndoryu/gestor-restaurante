ALTER TABLE configuracion_restaurante DROP COLUMN IF EXISTS bdp_billing_details;

ALTER TABLE clientes
    DROP COLUMN IF EXISTS poblacion,
    DROP COLUMN IF EXISTS codigo_postal,
    DROP COLUMN IF EXISTS direccion_fiscal,
    DROP COLUMN IF EXISTS nif;
