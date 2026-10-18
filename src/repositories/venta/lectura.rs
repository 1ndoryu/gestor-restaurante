// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* Lectura de ventas: consultas por id, por reserva, totales y pendientes de BDP. */

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Venta;

use super::tipos::VentaRepository;

impl VentaRepository {
    pub async fn find_by_id(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Venta>, sqlx::Error> {
        sqlx::query_as::<_, Venta>("SELECT * FROM ventas WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .fetch_optional(pool)
            .await
    }

    /* [094A-1] Buscar venta asociada a una reserva — para evitar duplicados.
     * Retorna true si ya existe al menos una venta con este reserva_id. */
    pub async fn exists_by_reserva_id(
        pool: &PgPool,
        reserva_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let rec = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM ventas WHERE reserva_id = $1)",
        )
        .bind(reserva_id)
        .fetch_one(pool)
        .await?;
        Ok(rec)
    }

    /// Suma de `importe_base` de ventas en un rango de fechas
    pub async fn total_periodo(
        pool: &PgPool,
        user_id: Uuid,
        desde: chrono::NaiveDate,
        hasta: chrono::NaiveDate,
        excluir_anuladas: bool,
    ) -> Result<rust_decimal::Decimal, sqlx::Error> {
        /* [128A-1/F4][F4-4] M10 parametrizado por modalidad: en
         * `credito_completo` el resumen excluye las anuladas (reversión
         * idempotente del IVA); en `estado_solo` NO hay reversión contable,
         * así que las anuladas siguen contando. El servicio (dashboard)
         * decide según `config.anulacion_modalidad`. */
        let excluye_anuladas_sql = if excluir_anuladas {
            " AND anulada = false"
        } else {
            ""
        };
        let query = format!(
            "SELECT COALESCE(SUM(importe_base), 0) as total FROM ventas \
             WHERE user_id = $1 AND fecha >= $2 AND fecha <= $3{excluye_anuladas_sql}"
        );
        let rec = sqlx::query_scalar::<_, Option<rust_decimal::Decimal>>(&query)
            .bind(user_id)
            .bind(desde)
            .bind(hasta)
            .fetch_one(pool)
            .await?;
        Ok(rec.unwrap_or_default())
    }

    /* [276A-4.2] Ventas pendientes de polling BDP:
     * bdp_synced=true, bdp_order_status no final ('invoiced' ni 'error'). */
    pub async fn list_bdp_pending(pool: &PgPool, user_id: Uuid) -> Result<Vec<Venta>, sqlx::Error> {
        sqlx::query_as::<_, Venta>(
            "SELECT * FROM ventas \
             WHERE user_id = $1 \
               AND bdp_synced = TRUE \
               AND anulada = FALSE \
               AND (bdp_order_status IS NULL OR bdp_order_status NOT IN ('invoiced', 'cancelled', 'error')) \
             ORDER BY created_at ASC LIMIT 100",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }

    /* [AUDIT-2.11b] Ventas huérfanas: la comanda puede existir en BDP pero
     * Glory no recibió confirmación (proceso murió entre HTTP y UPDATE local).
     * Estas ventas tienen bdp_synced=false + bdp_order_id no nulo + auditoría
     * 'pendiente' o 'ambiguo' para la operación 'create_order'.
     * El polling normal NO las detecta porque filtra bdp_synced=TRUE. */
    pub async fn list_bdp_orphaned(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<Venta>, sqlx::Error> {
        sqlx::query_as::<_, Venta>(
            "SELECT v.* FROM ventas v \
             WHERE v.user_id = $1 \
               AND v.bdp_synced = FALSE \
               AND v.bdp_order_id IS NOT NULL \
               AND EXISTS ( \
                 SELECT 1 FROM bdp_audit_log a \
                 WHERE a.user_id = $1 \
                   AND a.target_entity_type = 'venta' \
                   AND a.target_entity_id = v.id \
                   AND a.operacion = 'create_order' \
                   AND a.resultado IN ('pendiente', 'ambiguo') \
               ) \
             ORDER BY v.created_at ASC LIMIT 50",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }

    /* [AUDIT-N2] Clientes BDP huérfanos: tienen bdp_synced=true pero auditoría
     * pendiente/ambiguo para 'create_customer'. El proceso murió entre
     * update_bdp_sync y actualizar_resultado (antes del fix N1).
     * El polling cierra la auditoría consultando ExportCustomers en BDP. */
    pub async fn list_bdp_orphaned_customers(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::Cliente>, sqlx::Error> {
        sqlx::query_as::<_, crate::models::Cliente>(
            "SELECT c.* FROM clientes c \
             WHERE c.user_id = $1 \
               AND c.bdp_synced = TRUE \
               AND c.bdp_customer_code IS NOT NULL \
               AND EXISTS ( \
                 SELECT 1 FROM bdp_audit_log a \
                 WHERE a.user_id = $1 \
                   AND a.target_entity_type = 'cliente' \
                   AND a.target_entity_id = c.id \
                   AND a.operacion = 'create_customer' \
                   AND a.resultado IN ('pendiente', 'ambiguo') \
               ) \
             ORDER BY c.created_at ASC LIMIT 50",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }
}
