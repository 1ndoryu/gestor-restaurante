/* [C1-1] Claves de idempotencia para auto-arming BDP.
 * Permite deduplicar operaciones de escritura iniciadas desde la UI sin
 * depender de caché en memoria. */
ALTER TABLE bdp_audit_log
    ADD COLUMN idempotency_key VARCHAR(255) NULL;

CREATE UNIQUE INDEX idx_bdp_audit_idempotency
    ON bdp_audit_log(user_id, idempotency_key)
    WHERE idempotency_key IS NOT NULL;
