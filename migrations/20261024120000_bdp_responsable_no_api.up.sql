-- [D32] Responsable de las operaciones que BDP no expone por API (se gestionan en el TPV).
-- Vacío hasta que el restaurante lo rellene; el mensaje lo muestra tal cual.
ALTER TABLE configuracion_restaurante
    ADD COLUMN bdp_responsable_no_api TEXT NOT NULL DEFAULT '';
