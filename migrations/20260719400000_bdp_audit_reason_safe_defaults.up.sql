/* [197A-3] Conserva el motivo de cada autorización en la auditoría y elimina
 * el placeholder GLORY, incompatible con la validación numérica vigente. */
ALTER TABLE bdp_audit_log
    ADD COLUMN IF NOT EXISTS authorization_reason TEXT;

ALTER TABLE configuracion_restaurante
    ALTER COLUMN bdp_default_article_code SET DEFAULT '',
    ADD COLUMN IF NOT EXISTS bdp_env_bootstrap_applied_at TIMESTAMPTZ;

UPDATE configuracion_restaurante
SET bdp_default_article_code = '', updated_at = NOW()
WHERE UPPER(TRIM(bdp_default_article_code)) = 'GLORY';
