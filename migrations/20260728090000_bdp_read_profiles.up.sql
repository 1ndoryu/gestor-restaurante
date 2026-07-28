/* [287A-5] Parámetros de lectura configurables desde Glory.
 * No habilitan escrituras en BDP: solo seleccionan tarifa de catálogo y
 * plantilla ExportPurchaseNotes. NULL significa que Compras sigue sin configurar. */
ALTER TABLE configuracion_restaurante
    ADD COLUMN IF NOT EXISTS bdp_catalog_price_type INTEGER NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS bdp_purchase_notes_profile_id INTEGER;

ALTER TABLE configuracion_restaurante
    ADD CONSTRAINT configuracion_bdp_catalog_price_type_range
    CHECK (bdp_catalog_price_type BETWEEN 1 AND 5),
    ADD CONSTRAINT configuracion_bdp_purchase_profile_positive
    CHECK (bdp_purchase_notes_profile_id IS NULL OR bdp_purchase_notes_profile_id > 0);
