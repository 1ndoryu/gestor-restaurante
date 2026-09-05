-- [049A-1/S5] Nuevo estado 'rechazado': rechazo definitivo del BDP (HTTP 4xx,
-- payload inválido o conflicto). A diferencia de 'error' (transitorio), una fila
-- rechazada NO se reintenta automáticamente: solo reintento manual, igual que
-- 'pendiente_suscripcion'. La auditoría lo registra como 'error' (definitivo),
-- no como 'ambiguo'.
ALTER TABLE bdp_push_pendientes DROP CONSTRAINT bdp_push_pendientes_estado_check;
ALTER TABLE bdp_push_pendientes
    ADD CONSTRAINT bdp_push_pendientes_estado_check
    CHECK (estado IN ('pendiente', 'pendiente_suscripcion', 'error', 'rechazado', 'sincronizado', 'descartado'));