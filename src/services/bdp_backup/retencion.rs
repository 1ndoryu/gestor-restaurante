// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
use sqlx::PgPool;
use uuid::Uuid;

use super::BdpBackupService;

impl BdpBackupService {
    // =========================================================================
    // LIMPIEZA DE SNAPSHOTS EXPIRADOS
    // =========================================================================

    /// Elimina snapshots expirados.
    /* [D3] No eliminar snapshots que aún están referenciados por auditorías
     * pendientes o ambiguas. Sin esta protección, una auditoría pendiente
     * podría perder su evidencia pre-escritura. */
    pub async fn limpiar_expirados(pool: &PgPool) -> Result<u64, String> {
        let result = sqlx::query(
            r"DELETE FROM bdp_snapshots
              WHERE expires_at IS NOT NULL AND expires_at < NOW()
                AND NOT EXISTS (
                    SELECT 1 FROM bdp_audit_log a
                    WHERE a.snapshot_pre_id = bdp_snapshots.id
                      AND a.resultado IN ('pendiente', 'ambiguo')
                )",
        )
        .execute(pool)
        .await
        .map_err(|e| format!("Error limpiando snapshots expirados: {e}"))?;

        Ok(result.rows_affected())
    }

    // =========================================================================
    // LECTURA DE CONFIGURACIÓN DE RESPALDO
    // =========================================================================

    /* Días de retención del usuario (0 = indefinido). Sin fila de configuración
     * se rechaza: no se inventa una retención. */
    pub(super) async fn get_retention_days(pool: &PgPool, user_id: Uuid) -> Result<i32, String> {
        sqlx::query_scalar::<_, i32>(
            r"SELECT COALESCE(bdp_backup_retention_days, 30)
            FROM configuracion_restaurante WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| format!("Error leyendo retención de snapshots BDP: {error}"))?
        .ok_or_else(|| "No existe configuración para retención de snapshots BDP".to_string())
    }

    /* Auto-backup pre-write del usuario. Sin fila de configuración se rechaza
     * la escritura en lugar de asumir un valor. */
    pub(super) async fn get_auto_backup(pool: &PgPool, user_id: Uuid) -> Result<bool, String> {
        sqlx::query_scalar::<_, bool>(
            r"SELECT COALESCE(bdp_auto_backup_before_write, true)
            FROM configuracion_restaurante WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| format!("Error leyendo auto-backup BDP: {error}"))?
        .ok_or_else(|| "No existe configuración para auto-backup BDP".to_string())
    }
}
