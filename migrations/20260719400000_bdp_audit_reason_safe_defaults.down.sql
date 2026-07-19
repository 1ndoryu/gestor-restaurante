ALTER TABLE bdp_audit_log
    DROP COLUMN IF EXISTS authorization_reason;

ALTER TABLE configuracion_restaurante
    ALTER COLUMN bdp_default_article_code SET DEFAULT 'GLORY';

ALTER TABLE configuracion_restaurante
    DROP COLUMN IF EXISTS bdp_env_bootstrap_applied_at;
