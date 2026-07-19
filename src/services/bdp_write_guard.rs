use sqlx::PgPool;
use uuid::Uuid;

use crate::models::ConfiguracionRestaurante;
use crate::services::{BdpBackupService, BdpWeblinkClient};

pub struct BdpWriteGuard;

impl BdpWriteGuard {
    /// Un registro pendiente o ambiguo puede representar una operación remota
    /// aplicada sin confirmación local. Se bloquea cualquier nueva escritura
    /// sobre la misma entidad hasta reconciliación manual.
    pub async fn ensure_no_unresolved(
        pool: &PgPool,
        user_id: Uuid,
        entity_field: &str,
        entity_id: Uuid,
        operations: &[&str],
    ) -> Result<(), String> {
        let exists: bool = sqlx::query_scalar(
            r"SELECT EXISTS (
                 SELECT 1 FROM bdp_audit_log
                 WHERE user_id = $1
                   AND datos_enviados ->> $2 = $3
                   AND operacion = ANY($4)
                   AND resultado IN ('pendiente', 'ambiguo')
               )",
        )
        .bind(user_id)
        .bind(entity_field)
        .bind(entity_id.to_string())
        .bind(operations)
        .fetch_one(pool)
        .await
        .map_err(|error| format!("No se pudo verificar estado ambiguo BDP: {error}"))?;
        if exists {
            return Err(format!(
                "Escritura BDP bloqueada: {entity_field}={entity_id} tiene una operación pendiente o ambigua; debe reconciliarse antes"
            ));
        }
        Ok(())
    }

    /// Convierte un armado temporal en una única intención auditable. La misma
    /// transacción consume el cupo, registra la intención y devuelve el modo a
    /// solo lectura ANTES de cualquier HTTP de escritura.
    #[allow(clippy::too_many_arguments)]
    /* [187A-1] Esta transacción es deliberadamente lineal: lock, bloqueo por
     * ambigüedad, consumo, auditoría y kill switch deben ser indivisibles. */
    #[allow(clippy::too_many_lines)]
    pub async fn authorize(
        pool: &PgPool,
        user_id: Uuid,
        config: &ConfiguracionRestaurante,
        scope: &str,
        target_entity_type: &str,
        target_entity_id: Uuid,
        entity_json_field: &str,
        datos_enviados: &serde_json::Value,
        snapshot_pre_id: Option<Uuid>,
    ) -> Result<Uuid, String> {
        /* La allowlist se valida antes de tocar auditoría o autorización. El
         * cliente HTTP repite esta comprobación justo antes del envío. */
        BdpWeblinkClient::new(config)
            .ensure_write_target_allowed()
            .map_err(|error| error.to_string())?;
        let base = BdpBackupService::canonical_target(config)?;
        let fingerprint = BdpBackupService::connection_fingerprint(config)?;

        let mut tx = pool
            .begin()
            .await
            .map_err(|error| format!("No se pudo iniciar autorización BDP: {error}"))?;

        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!(
                "bdp-write:{user_id}:{target_entity_type}:{target_entity_id}:{scope}"
            ))
            .execute(&mut *tx)
            .await
            .map_err(|error| format!("No se pudo bloquear la intención BDP: {error}"))?;

        let unresolved: bool = sqlx::query_scalar(
            r"SELECT EXISTS (
                 SELECT 1 FROM bdp_audit_log
                 WHERE user_id = $1
                   AND resultado IN ('pendiente', 'ambiguo')
                   AND (
                     (target_entity_type = $2 AND target_entity_id = $3)
                     OR datos_enviados ->> $4 = $3::TEXT
                   )
               )",
        )
        .bind(user_id)
        .bind(target_entity_type)
        .bind(target_entity_id)
        .bind(entity_json_field)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| format!("No se pudo verificar intención previa BDP: {error}"))?;
        if unresolved {
            return Err(format!(
                "Escritura BDP bloqueada: {target_entity_type}={target_entity_id} tiene una operación pendiente o ambigua"
            ));
        }

        let consumed: Option<(Option<Uuid>, String)> = sqlx::query_as(
            r"UPDATE bdp_write_arming
               SET remaining_operations = remaining_operations - 1
               WHERE user_id = $1
                 AND base_url = $2
                 AND $3 = ANY(scopes)
                 AND target_entity_type = $4
                 AND target_entity_id = $5
                 AND connection_fingerprint = $6
                 AND snapshot_id IS NOT NULL
                 AND expires_at > NOW()
                 AND remaining_operations > 0
                 AND EXISTS (
                   SELECT 1 FROM configuracion_restaurante c
                   WHERE c.user_id = $1
                     AND TRIM(TRAILING '/' FROM TRIM(c.bdp_base_url)) = $2
                     AND c.bdp_login = $7
                     AND c.bdp_password = $8
                     AND c.bdp_integrator_code = $9
                     AND c.bdp_pos_id = $10
                     AND c.bdp_employee_id = $11
                     AND c.bdp_items_profile_id = $12
                     AND c.bdp_sync_mode = 'unidirectional'
                 )
               RETURNING snapshot_id, reason",
        )
        .bind(user_id)
        .bind(&base)
        .bind(scope)
        .bind(target_entity_type)
        .bind(target_entity_id)
        .bind(&fingerprint)
        .bind(&config.bdp_login)
        .bind(&config.bdp_password)
        .bind(&config.bdp_integrator_code)
        .bind(config.bdp_pos_id)
        .bind(config.bdp_employee_id)
        .bind(config.bdp_items_profile_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| format!("No se pudo verificar el armado BDP: {error}"))?;

        let (arming_snapshot_id, authorization_reason) = consumed.ok_or_else(|| {
            format!(
                "Escritura BDP bloqueada: no existe armado vigente para alcance {scope}, objetivo {target_entity_type}={target_entity_id}, destino exacto y cupo disponible"
            )
        })?;

        let audit_snapshot_id = snapshot_pre_id.or(arming_snapshot_id);

        let audit_id: Uuid = sqlx::query_scalar(
            r"INSERT INTO bdp_audit_log
               (user_id, operacion, direccion, snapshot_pre_id, datos_enviados,
                resultado, target_base_url, target_entity_type, target_entity_id,
                authorization_reason)
               VALUES ($1, $2, 'glory_to_bdp', $3, $4, 'pendiente', $5, $6, $7, $8)
               RETURNING id",
        )
        .bind(user_id)
        .bind(scope)
        .bind(audit_snapshot_id)
        .bind(datos_enviados)
        .bind(&base)
        .bind(target_entity_type)
        .bind(target_entity_id)
        .bind(authorization_reason)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| format!("No se pudo registrar intención BDP: {error}"))?;

        let mode_updated = sqlx::query(
            "UPDATE configuracion_restaurante SET bdp_sync_mode = 'read_only', updated_at = NOW() WHERE user_id = $1",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("No se pudo cerrar modo escritura BDP: {error}"))?;
        if mode_updated.rows_affected() != 1 {
            return Err(
                "No se pudo cerrar modo escritura BDP: configuración inexistente".to_string(),
            );
        }

        sqlx::query("DELETE FROM bdp_write_arming WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|error| format!("No se pudo eliminar armado BDP consumido: {error}"))?;

        tx.commit()
            .await
            .map_err(|error| format!("No se pudo confirmar autorización BDP: {error}"))?;
        Ok(audit_id)
    }
}
