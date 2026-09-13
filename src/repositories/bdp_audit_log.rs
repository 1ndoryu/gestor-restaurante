// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [267A-7] Repositorio del ledger de auditoría BDP (`bdp_audit_log`).
 * Centraliza las lecturas/escrituras que antes vivían en handlers y en
 * `BdpBackupService::actualizar_resultado` con `&PgPool`.
 * La variante transaccional existe para el commit atómico post-create_customer
 * ([AUDIT-N1] en `bdp_customer_sync`): si el proceso muere después del HTTP,
 * no queda `bdp_synced=true` sin auditoría cerrada (o viceversa). */

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub struct BdpAuditLogRepository;

impl BdpAuditLogRepository {
    /// Cierra una auditoría como éxito dentro de una transacción ya abierta.
    /// SQL idéntico al que usaba el handler (resultado + respuesta + limpieza de error).
    pub async fn marcar_exito_tx(
        tx: &mut Transaction<'_, Postgres>,
        audit_id: Uuid,
        respuesta: Option<&serde_json::Value>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r"UPDATE bdp_audit_log
            SET resultado = 'exito', datos_respuesta = $2, error_mensaje = NULL, updated_at = NOW()
            WHERE id = $1",
        )
        .bind(audit_id)
        .bind(respuesta)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Recupera el `InvoiceNumber` de la última auditoría `invoice` exitosa
    /// ([C1-6]: idempotencia — si la operación ya fue exitosa, se devuelve el
    /// estado actual en lugar de re-facturar).
    pub async fn ultimo_invoice_number_exitoso(
        pool: &PgPool,
        user_id: Uuid,
        venta_id: Uuid,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar(
            r"SELECT datos_respuesta ->> 'InvoiceNumber'
               FROM bdp_audit_log
               WHERE user_id = $1
                 AND operacion = 'invoice'
                 AND target_entity_type = 'venta'
                 AND target_entity_id = $2
                 AND resultado = 'exito'
               ORDER BY created_at DESC
               LIMIT 1",
        )
        .bind(user_id)
        .bind(venta_id)
        .fetch_optional(pool)
        .await
    }
}
