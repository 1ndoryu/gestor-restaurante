/* [XT2-1] Feature flags por restaurante para funcionalidades BDP.
 * Permiten activar/desactivar features de forma granular sin redeploy.
 * Todos los flags nacen desactivados por defecto. */
ALTER TABLE configuracion_restaurante
    ADD COLUMN ff_bdp_auto_arm BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN ff_bdp_partial_payments BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN ff_bdp_cancel_order BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN ff_bdp_purchase_notes_read BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN ff_bdp_purchase_notes_draft BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN ff_bdp_purchase_notes_receive BOOLEAN NOT NULL DEFAULT FALSE;
