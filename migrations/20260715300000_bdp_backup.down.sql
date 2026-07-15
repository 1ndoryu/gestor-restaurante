/* Revertir sistema de backup BDP */
ALTER TABLE configuracion_restaurante DROP COLUMN IF EXISTS bdp_auto_backup_before_write;
ALTER TABLE configuracion_restaurante DROP COLUMN IF EXISTS bdp_backup_retention_days;
ALTER TABLE configuracion_restaurante DROP COLUMN IF EXISTS bdp_sync_mode;

DROP TABLE IF EXISTS bdp_audit_log;
DROP TABLE IF EXISTS bdp_snapshots;
