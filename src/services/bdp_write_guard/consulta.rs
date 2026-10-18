// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
use sqlx::PgPool;
use uuid::Uuid;

use super::BdpWriteGuard;

impl BdpWriteGuard {
    /// Modos de `bdp_sync_mode` que permiten escrituras Glory → BDP:
    /// `unidirectional` (con armado por operación) y `automatic` (autorización
    /// permanente, siempre auditada). `read_only` y cualquier otro valor
    /// bloquean.
    #[allow(clippy::must_use_candidate)]
    pub fn modo_permite_escritura(mode: &str) -> bool {
        matches!(mode, "unidirectional" | "automatic")
    }

    /// Un registro pendiente o ambiguo puede representar una operación remota
    /// aplicada sin confirmación local. Se bloquea cualquier nueva escritura
    /// sobre la misma entidad hasta reconciliación manual.
    /// Verifica si ya existe un registro de auditoría con la misma clave de
    /// `idempotency_key` para este usuario. Devuelve el `audit_id` y el resultado si
    /// se encuentra, o `None` si no existe.
    pub async fn check_idempotency(
        pool: &PgPool,
        user_id: Uuid,
        idempotency_key: &str,
    ) -> Result<Option<(Uuid, String)>, String> {
        let row: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT id, resultado FROM bdp_audit_log WHERE user_id = $1 AND idempotency_key = $2",
        )
        .bind(user_id)
        .bind(idempotency_key)
        .fetch_optional(pool)
        .await
        .map_err(|error| format!("No se pudo verificar idempotencia BDP: {error}"))?;
        Ok(row)
    }

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
}
