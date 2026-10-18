// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* Anulación local de ventas (128A-1/F4): transacción con auditoría idempotente. */

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Venta;

use super::tipos::VentaRepository;

impl VentaRepository {
    /* [128A-1/F4] Anulación local de ventas (D4, M9-M11).
     *
     * Transición de estado única: solo `anulada=false` puede pasar a
     * `anulada=true` (guard M10). Bloquea ventas facturadas (M9) y ventas
     * ya anuladas (idempotencia por guard, no por UPDATE: si ya está
     * anulada devuelve la fila sin modificarla y `resultado_previo` indica
     * el resultado de la auditoría previa).
     *
     * Auditoría obligatoria (C1): INSERT en `bdp_audit_log` con
     * `operacion='anular_venta'`, `direccion='internal'`, `resultado='exito'`,
     * `target_entity_type='venta'`, `target_entity_id`, `authorization_reason`
     * (motivo) e `idempotency_key`. Con `ON CONFLICT (user_id, idempotency_key)
     * WHERE idempotency_key IS NOT NULL DO NOTHING` se logra idempotencia
     * (doble click seguro): si la clave ya se usó, no se vuelve a anular ni a
     * auditar; se devuelve el resultado previo para que el handler decida.
     *
     * Retorna (Venta, audit_id, resultado_previo, ya_anulada).
     */
    pub async fn anular(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        motivo: Option<&str>,
        anulacion_usuario: Option<Uuid>,
        idempotency_key: Option<&str>,
    ) -> Result<(Venta, Uuid, Option<String>, bool), sqlx::Error> {
        let mut tx = pool.begin().await?;

        let venta_actual: Option<Venta> =
            sqlx::query_as::<_, Venta>("SELECT * FROM ventas WHERE id = $1 AND user_id = $2")
                .bind(id)
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;

        let Some(venta) = venta_actual else {
            return Err(sqlx::Error::RowNotFound);
        };

        /* M9: solo ventas no facturadas (ni en BDP ni factura local F6). */
        if venta.facturada_local
            || venta.bdp_invoiced
            || venta.bdp_order_status.as_deref() == Some("invoiced")
        {
            return Err(sqlx::Error::Protocol(
                "venta_facturada_no_anulable".to_string(),
            ));
        }

        let audit_payload = serde_json::json!({
            "venta_id": id,
            "motivo": motivo,
            "importe_base": venta.importe_base,
            "importe_iva": venta.importe_iva,
            "bdp_synced": venta.bdp_synced,
            "bdp_order_id": venta.bdp_order_id,
        });

        /* Idempotencia C1: si la clave ya existe, no se anula dos veces. */
        let maybe_audit_id: Option<Uuid> = sqlx::query_scalar(
            r"INSERT INTO bdp_audit_log
               (user_id, operacion, direccion, datos_enviados, resultado, origen_operacion,
                target_entity_type, target_entity_id, authorization_reason, idempotency_key)
               VALUES ($1, 'anular_venta', 'internal', $2, 'exito', 'local', 'venta', $3, $4, $5)
               ON CONFLICT (user_id, idempotency_key) WHERE idempotency_key IS NOT NULL DO NOTHING
               RETURNING id",
        )
        .bind(user_id)
        .bind(audit_payload)
        .bind(id)
        .bind(format!(
            "Anulación local de venta {} ({}) — operación interna, no requiere autorización BDP",
            id,
            motivo.unwrap_or("sin motivo")
        ))
        .bind(idempotency_key)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(audit_id) = maybe_audit_id else {
            /* [128A-1/F4][F4-5] La clave de idempotencia está scoped por venta:
             * si la fila previa apunta a OTRA venta, no es un reintento de esta
             * anulación sino una clave reutilizada → conflicto, nunca éxito
             * idempotente falso. */
            let (existing_id, resultado, target_entity_id): (Uuid, String, Uuid) = sqlx::query_as(
                "SELECT id, resultado, target_entity_id \
                 FROM bdp_audit_log \
                 WHERE user_id = $1 AND idempotency_key = $2",
            )
            .bind(user_id)
            .bind(idempotency_key.unwrap_or_default())
            .fetch_one(&mut *tx)
            .await?;
            if target_entity_id != id {
                tx.rollback().await?;
                return Err(sqlx::Error::Protocol(
                    "idempotency_key_otra_venta".to_string(),
                ));
            }
            /* Si ya estaba anulada, devolver la fila actual (estado consistente). */
            let venta_actual =
                sqlx::query_as::<_, Venta>("SELECT * FROM ventas WHERE id = $1 AND user_id = $2")
                    .bind(id)
                    .bind(user_id)
                    .fetch_one(&mut *tx)
                    .await?;
            tx.commit().await?;
            let ya_anulada = venta_actual.anulada;
            return Ok((venta_actual, existing_id, Some(resultado), ya_anulada));
        };

        /* Transición única con guard: solo si aún no está anulada. */
        let venta_anulada: Option<Venta> = sqlx::query_as::<_, Venta>(
            "UPDATE ventas SET anulada = true, anulada_at = NOW(), \
             anulacion_motivo = COALESCE($3, anulacion_motivo), \
             anulacion_usuario = COALESCE($4, anulacion_usuario), \
             updated_at = NOW() \
             WHERE id = $1 AND user_id = $2 AND anulada = false \
             RETURNING *",
        )
        .bind(id)
        .bind(user_id)
        .bind(motivo)
        .bind(anulacion_usuario)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(venta_anulada) = venta_anulada else {
            /* Ya estaba anulada (carrera): la auditoría de esta clave se
             * insertó recién pero el guard impidió el UPDATE; revertir la
             * auditoría para no inventar un éxito sin efecto. */
            tx.rollback().await?;
            return Err(sqlx::Error::Protocol("venta_ya_anulada".to_string()));
        };

        tx.commit().await?;

        Ok((venta_anulada, audit_id, None, false))
    }
}
