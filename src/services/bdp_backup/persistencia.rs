// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
use sqlx::PgPool;
use uuid::Uuid;

use super::tipos::NuevoSnapshot;
use super::{BdpAuditEntry, BdpBackupService, BdpSnapshot};

/* Acceso a las tablas bdp_snapshots y bdp_audit_log. Aquí solo hay SQL de
 * lectura, inserción y actualización; la lógica de cuándo escribir vive en
 * los módulos de snapshot y escritura. */
impl BdpBackupService {
    /* [187A-1] Helper interno de persistencia: los campos reflejan uno a uno el
     * registro inmutable de evidencia y todos los llamadores son locales. */
    pub(super) async fn insert_snapshot(
        pool: &PgPool,
        user_id: Uuid,
        nuevo: NuevoSnapshot<'_>,
    ) -> Result<BdpSnapshot, String> {
        sqlx::query_as::<_, BdpSnapshot>(
            r"INSERT INTO bdp_snapshots
               (user_id, tipo, direccion, trigger_tipo, datos, metadata,
                target_base_url, connection_fingerprint, expires_at, notas)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, user_id, tipo, direccion, trigger_tipo, datos, metadata,
                      target_base_url, connection_fingerprint, created_at, expires_at, notas",
        )
        .bind(user_id)
        .bind(nuevo.tipo)
        .bind(nuevo.direccion)
        .bind(nuevo.trigger_tipo)
        .bind(nuevo.datos)
        .bind(nuevo.metadata)
        .bind(nuevo.target_base_url)
        .bind(nuevo.connection_fingerprint)
        .bind(nuevo.expires_at)
        .bind(nuevo.notas)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Error insertando snapshot: {e}"))
    }

    /// Actualiza el resultado de una entrada de auditoría.
    pub async fn actualizar_resultado(
        pool: &PgPool,
        audit_id: Uuid,
        resultado: &str,
        datos_respuesta: Option<&serde_json::Value>,
        error_mensaje: Option<&str>,
    ) -> Result<(), String> {
        let result = sqlx::query(
            r"UPDATE bdp_audit_log
            SET resultado = $2, datos_respuesta = $3, error_mensaje = $4, updated_at = NOW()
            WHERE id = $1",
        )
        .bind(audit_id)
        .bind(resultado)
        .bind(datos_respuesta)
        .bind(error_mensaje)
        .execute(pool)
        .await
        .map_err(|e| format!("Error actualizando audit log: {e}"))?;

        if result.rows_affected() != 1 {
            return Err(format!(
                "No se actualizó la auditoría BDP {audit_id}: registro inexistente"
            ));
        }
        Ok(())
    }

    // =========================================================================
    // CONSULTAS
    // =========================================================================

    /// Lista snapshots del usuario, ordenados por fecha descendente.
    pub async fn listar_snapshots(
        pool: &PgPool,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<BdpSnapshot>, String> {
        sqlx::query_as::<_, BdpSnapshot>(
            r"SELECT id, user_id, tipo, direccion, trigger_tipo, datos, metadata,
                      target_base_url, connection_fingerprint, created_at, expires_at, notas
            FROM bdp_snapshots
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2",
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error listando snapshots: {e}"))
    }

    /// Obtiene un snapshot por ID.
    pub async fn obtener_snapshot(
        pool: &PgPool,
        snapshot_id: Uuid,
    ) -> Result<Option<BdpSnapshot>, String> {
        sqlx::query_as::<_, BdpSnapshot>(
            r"SELECT id, user_id, tipo, direccion, trigger_tipo, datos, metadata,
                      target_base_url, connection_fingerprint, created_at, expires_at, notas
            FROM bdp_snapshots
            WHERE id = $1",
        )
        .bind(snapshot_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Error obteniendo snapshot: {e}"))
    }

    /// Elimina un snapshot.
    pub async fn eliminar_snapshot(
        pool: &PgPool,
        snapshot_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, String> {
        let result = sqlx::query(r"DELETE FROM bdp_snapshots WHERE id = $1 AND user_id = $2")
            .bind(snapshot_id)
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| format!("Error eliminando snapshot: {e}"))?;

        Ok(result.rows_affected() > 0)
    }

    /// Lista entradas del audit log.
    pub async fn listar_audit(
        pool: &PgPool,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<BdpAuditEntry>, String> {
        sqlx::query_as::<_, BdpAuditEntry>(
            r"SELECT id, user_id, operacion, direccion, snapshot_pre_id,
                      datos_enviados, resultado, datos_respuesta, error_mensaje,
                      target_base_url, target_entity_type, target_entity_id,
                      authorization_reason, origen_operacion, created_at, updated_at
            FROM bdp_audit_log
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2",
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error listando audit log: {e}"))
    }
}
