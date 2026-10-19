// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* Escritura de ventas: alta, edición parcial, propina y borrados. */

use sqlx::{Executor, PgPool, Postgres};
use uuid::Uuid;

use crate::models::Venta;

use super::tipos::{ActualizarVentaData, NuevaVenta, VentaRepository};

impl VentaRepository {
    pub async fn create(pool: &PgPool, data: &NuevaVenta<'_>) -> Result<Venta, sqlx::Error> {
        Self::create_with(pool, data).await
    }

    pub async fn create_with<'e, E>(
        executor: E,
        data: &NuevaVenta<'_>,
    ) -> Result<Venta, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, Venta>(
            "INSERT INTO ventas (id, user_id, fecha, comensales, descripcion, iva_porcentaje, \
             turno, canal, metodo_pago, importe_base, importe_iva, reserva_id, cliente_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) \
             RETURNING *",
        )
        .bind(id)
        .bind(data.user_id)
        .bind(data.fecha)
        .bind(data.comensales)
        .bind(data.descripcion)
        .bind(data.iva_porcentaje)
        .bind(data.turno)
        .bind(data.canal)
        .bind(data.metodo_pago)
        .bind(data.importe_base)
        .bind(data.importe_iva)
        .bind(data.reserva_id)
        .bind(data.cliente_id)
        .fetch_one(executor)
        .await
    }

    /* [198A-1/D8] Guarda la propina local de una venta. Fuente de verdad local
     * para mostrar/editar la propina sin BDP; el push AddOrderTip se encola en
     * el handler. `add_tip` suma al valor actual o lo sustituye, igual que en
     * BDP, y se resuelve en el mismo UPDATE para no leer y escribir por separado. */
    pub async fn actualizar_propina(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        importe: rust_decimal::Decimal,
        add_tip: bool,
    ) -> Result<Option<Venta>, sqlx::Error> {
        sqlx::query_as::<_, Venta>(
            "UPDATE ventas SET propina = CASE WHEN $4 THEN propina + $3 ELSE $3 END, \
             updated_at = NOW() WHERE id = $1 AND user_id = $2 RETURNING *",
        )
        .bind(id)
        .bind(user_id)
        .bind(importe)
        .bind(add_tip)
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        /* [128A-1/F4/D5] Las ventas anuladas nunca se borran físicamente:
         * son histórico con motivo. Guard en el DELETE para impedirlo. */
        /* [128A-1/F4] query() dinámico (no macro): la cache offline .sqlx/
         * no contiene las columnas nuevas de la migración F4. */
        let result =
            sqlx::query("DELETE FROM ventas WHERE id = $1 AND user_id = $2 AND anulada = false")
                .bind(id)
                .bind(user_id)
                .execute(pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    /* [094A-1] Eliminar ventas asociadas a una reserva al descompletar.
     * Solo elimina ventas auto-generadas (importe_base = 0) para no perder datos manuales. */
    pub async fn delete_by_reserva_id(
        pool: &PgPool,
        reserva_id: Uuid,
        user_id: Uuid,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM ventas WHERE reserva_id = $1 AND user_id = $2 AND importe_base = 0",
        )
        .bind(reserva_id)
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /* [283A-22] Actualizar parcialmente una venta — COALESCE mantiene valores existentes
     * cuando el campo no se envía (None).
     * [014A-11] Convertido a query_as! para verificación SQL en compilación. */
    pub async fn update(
        pool: &PgPool,
        data: &ActualizarVentaData<'_>,
    ) -> Result<Option<Venta>, sqlx::Error> {
        Self::update_with(pool, data).await
    }

    pub async fn update_with<'e, E>(
        executor: E,
        data: &ActualizarVentaData<'_>,
    ) -> Result<Option<Venta>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, Venta>(
            "UPDATE ventas SET \
             fecha = COALESCE($3, fecha), \
             comensales = COALESCE($4, comensales), \
             descripcion = COALESCE($5, descripcion), \
             iva_porcentaje = COALESCE($6, iva_porcentaje), \
             turno = COALESCE($7, turno), \
             canal = COALESCE($8, canal), \
             metodo_pago = COALESCE($9, metodo_pago), \
             importe_base = COALESCE($10, importe_base), \
             importe_iva = COALESCE($11, importe_iva), \
             updated_at = NOW() \
             WHERE id = $1 AND user_id = $2 \
             RETURNING *",
        )
        .bind(data.id)
        .bind(data.user_id)
        .bind(data.fecha)
        .bind(data.comensales)
        .bind(data.descripcion)
        .bind(data.iva_porcentaje)
        .bind(data.turno)
        .bind(data.canal)
        .bind(data.metodo_pago)
        .bind(data.importe_base)
        .bind(data.importe_iva)
        .fetch_optional(executor)
        .await
    }
}
