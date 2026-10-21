// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* Sincronización de ventas con Haddock y BDP: estados de envío y de pedido. */

use sqlx::PgPool;
use uuid::Uuid;

use super::tipos::VentaRepository;

impl VentaRepository {
    /* [064A-6] Actualiza el estado de sincronización Haddock de una venta.
     * Llamado por HaddockService después de cada intento de sync. */
    pub async fn update_haddock_status(
        pool: &PgPool,
        id: Uuid,
        synced: bool,
        error_msg: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE ventas SET \
             haddock_synced = $2, \
             haddock_synced_at = CASE WHEN $2 THEN NOW() ELSE haddock_synced_at END, \
             haddock_sync_error = $3 \
             WHERE id = $1",
            id,
            synced,
            error_msg
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /* [065A-5] Actualiza el estado de sincronización BDP de una venta.
     * Patrón idéntico a update_haddock_status pero usando sqlx::query() sin macro
     * para evitar necesitar cargo sqlx prepare (nuevas columnas no están en cache). */
    pub async fn update_bdp_status(
        pool: &PgPool,
        id: Uuid,
        synced: bool,
        error_msg: Option<&str>,
        order_id: Option<i64>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE ventas SET bdp_synced = $2, bdp_synced_at = CASE WHEN $2 THEN NOW() ELSE bdp_synced_at END, bdp_sync_error = $3, bdp_order_id = CASE WHEN $2 THEN $4 ELSE bdp_order_id END WHERE id = $1",
        )
        .bind(id)
        .bind(synced)
        .bind(error_msg)
        .bind(order_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /* [276A-4.2] Actualiza bdp_order_status de una venta — polling/endpoint manual.
     * [F8.4] Si el status es "invoiced", también marca bdp_invoiced = true.
     * [D10] Si BDP dice `cancelled`, la venta queda anulada aquí también: el
     * polling ve el mismo hecho que el aviso y las dos vías no pueden contar
     * cosas distintas. La marca es única (`NOT anulada`) para no pisar un motivo
     * ya registrado. */
    pub async fn update_bdp_order_status<'e, E>(
        executor: E,
        id: Uuid,
        status: &str,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let invoiced = status == "invoiced";
        let cancelada = status == "cancelled";
        sqlx::query(
            "UPDATE ventas SET bdp_order_status = $2, bdp_invoiced = $3,
                 anulada = CASE WHEN $4 AND NOT anulada THEN TRUE ELSE anulada END,
                 anulada_at = CASE WHEN $4 AND NOT anulada THEN NOW() ELSE anulada_at END,
                 anulacion_motivo = CASE WHEN $4 AND NOT anulada
                                         THEN 'Anulada en el TPV (sondeo de BDP)'
                                         ELSE anulacion_motivo END,
                 updated_at = NOW()
               WHERE id = $1",
        )
        .bind(id)
        .bind(status)
        .bind(invoiced)
        .bind(cancelada)
        .execute(executor)
        .await?;
        Ok(())
    }
}
