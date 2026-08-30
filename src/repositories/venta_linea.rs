// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [F2.4] Repositorio de líneas de venta.
 * Batch insert para crear múltiples líneas junto con la venta.
 * Lectura por venta para poblar el pedido BDP multi-item. */

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::models::{CrearVentaLineaRequest, VentaLinea};

pub struct VentaLineaRepository;

impl VentaLineaRepository {
    /// Crea múltiples líneas para una venta (batch insert)
    pub async fn crear_batch(
        pool: &PgPool,
        venta_id: Uuid,
        lineas: &[CrearVentaLineaRequest],
    ) -> Result<Vec<VentaLinea>, sqlx::Error> {
        let mut resultado = Vec::with_capacity(lineas.len());
        for linea in lineas {
            let id = Uuid::new_v4();
            let linea_db = sqlx::query_as::<_, VentaLinea>(
                "INSERT INTO venta_lineas (id, venta_id, articulo_codigo, descripcion, cantidad, precio_unitario, iva_pct, descuento) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
                 RETURNING *",
            )
            .bind(id)
            .bind(venta_id)
            .bind(linea.articulo_codigo.as_deref().unwrap_or(""))
            .bind(&linea.descripcion)
            .bind(linea.cantidad.unwrap_or(rust_decimal::Decimal::ONE))
            .bind(linea.precio_unitario)
            .bind(linea.iva_pct.unwrap_or(rust_decimal::Decimal::ZERO))
            .bind(linea.descuento.unwrap_or(rust_decimal::Decimal::ZERO))
            .fetch_one(pool)
            .await?;
            resultado.push(linea_db);
        }
        Ok(resultado)
    }

    /// Variante para una transacción ya abierta. Todas las líneas comparten
    /// la misma conexión y cualquier error permite revertir también la venta.
    pub async fn crear_batch_conn(
        conn: &mut PgConnection,
        venta_id: Uuid,
        lineas: &[CrearVentaLineaRequest],
    ) -> Result<Vec<VentaLinea>, sqlx::Error> {
        let mut resultado = Vec::with_capacity(lineas.len());
        for linea in lineas {
            let linea_db = sqlx::query_as::<_, VentaLinea>(
                "INSERT INTO venta_lineas (id, venta_id, articulo_codigo, descripcion, cantidad, precio_unitario, iva_pct, descuento) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *",
            )
            .bind(Uuid::new_v4())
            .bind(venta_id)
            .bind(linea.articulo_codigo.as_deref().unwrap_or(""))
            .bind(&linea.descripcion)
            .bind(linea.cantidad.unwrap_or(rust_decimal::Decimal::ONE))
            .bind(linea.precio_unitario)
            .bind(linea.iva_pct.unwrap_or(rust_decimal::Decimal::ZERO))
            .bind(linea.descuento.unwrap_or(rust_decimal::Decimal::ZERO))
            .fetch_one(&mut *conn)
            .await?;
            resultado.push(linea_db);
        }
        Ok(resultado)
    }

    pub async fn reemplazar_conn(
        conn: &mut PgConnection,
        venta_id: Uuid,
        lineas: &[CrearVentaLineaRequest],
    ) -> Result<Vec<VentaLinea>, sqlx::Error> {
        sqlx::query("DELETE FROM venta_lineas WHERE venta_id = $1")
            .bind(venta_id)
            .execute(&mut *conn)
            .await?;
        Self::crear_batch_conn(conn, venta_id, lineas).await
    }

    /// Lista todas las líneas de una venta
    pub async fn listar_por_venta(
        pool: &PgPool,
        venta_id: Uuid,
    ) -> Result<Vec<VentaLinea>, sqlx::Error> {
        sqlx::query_as::<_, VentaLinea>(
            "SELECT * FROM venta_lineas WHERE venta_id = $1 ORDER BY created_at",
        )
        .bind(venta_id)
        .fetch_all(pool)
        .await
    }

    /// Elimina todas las líneas de una venta (usado antes de re-crear en actualización)
    pub async fn eliminar_por_venta(pool: &PgPool, venta_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM venta_lineas WHERE venta_id = $1")
            .bind(venta_id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected())
    }
}
