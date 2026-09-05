-- Reversa de 20260905100000: quitar 'rechazado' del CHECK de estado.
ALTER TABLE bdp_push_pendientes DROP CONSTRAINT bdp_push_pendientes_estado_check;
ALTER TABLE bdp_push_pendientes
    ADD CONSTRAINT bdp_push_pendientes_estado_check
    CHECK (estado IN ('pendiente', 'pendiente_suscripcion', 'error', 'sincronizado', 'descartado'));