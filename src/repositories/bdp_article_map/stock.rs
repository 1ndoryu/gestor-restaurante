// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* Stock local por almacén: lectura, upsert de sync y ajuste manual. */

use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::tipos::{AjusteStockError, BdpArticleMapRepository};

impl BdpArticleMapRepository {
    /// Lista el stock de un usuario opcionalmente filtrado por almacén.
    pub async fn listar_stock(
        pool: &PgPool,
        user_id: Uuid,
        warehouse_id: Option<&str>,
    ) -> Result<Vec<crate::models::BdpArticleStock>, sqlx::Error> {
        let rows = if let Some(wid) = warehouse_id {
            sqlx::query_as::<_, crate::models::BdpArticleStock>(
                "SELECT * FROM bdp_article_stock WHERE user_id = $1 AND warehouse_id = $2 ORDER BY articulo_glory_codigo",
            )
            .bind(user_id)
            .bind(wid)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, crate::models::BdpArticleStock>(
                "SELECT * FROM bdp_article_stock WHERE user_id = $1 ORDER BY articulo_glory_codigo",
            )
            .bind(user_id)
            .fetch_all(pool)
            .await?
        };
        Ok(rows)
    }

    /// Upsert del stock por almacén. El almacén lo elige quien llama
    /// (`warehouse_id`/`warehouse_name`): el sync agregado de `ExportArticles`
    /// pasa "0"/"General" y el stock real por almacén pasa el id del almacén.
    /// [247A-10/S2] Se usa desde `sync_catalog` para guardar el stock agregado
    /// de `ExportArticles` mientras BDP no devuelva desglose por almacén.
    /// [128A-1/F3] No sobrescribe filas ajustadas localmente: el UPDATE se
    /// condiciona a `NOT ajustado_local` para que el sync nunca pise la
    /// fuente de verdad editable (`bdp_article_stock`).
    pub async fn upsert_stock(
        pool: &PgPool,
        user_id: Uuid,
        articulo_glory_codigo: &str,
        warehouse_id: &str,
        warehouse_name: &str,
        stock: Decimal,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO bdp_article_stock \
                (id, user_id, articulo_glory_codigo, warehouse_id, warehouse_name, stock, ultima_sync_at, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW(), NOW()) \
             ON CONFLICT (user_id, articulo_glory_codigo, warehouse_id) DO UPDATE SET \
                stock = EXCLUDED.stock, \
                warehouse_name = EXCLUDED.warehouse_name, \
                ultima_sync_at = NOW(), \
                updated_at = NOW() \
             WHERE NOT bdp_article_stock.ajustado_local \
               AND bdp_article_stock.stock IS DISTINCT FROM EXCLUDED.stock",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(articulo_glory_codigo)
        .bind(warehouse_id)
        .bind(warehouse_name)
        .bind(stock)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /* [128A-1/F3] Ajuste manual de stock local (entrada/salida) con auditoría.
     * Fuente de verdad del stock local: `bdp_article_stock` (por almacén).
     * `bdp_article_map.stock_actual` sigue siendo el snapshot BDP de solo
     * lectura y NO se toca aquí.
     *
     * Idempotencia (decisión F3, patrón C1): si viene `idempotency_key`, el
     * INSERT de auditoría usa ON CONFLICT ... DO NOTHING. Si ya existía una
     * entrada con esa clave, se devuelve (fila_existente, resultado_previo)
     * sin aplicar el delta de nuevo; el handler decide entre éxito idempotente
     * (resultado == 'exito') o 409 Conflicto. Los delta e insert de stock se
     * ejecutan SOLO cuando la auditoría se inserta por primera vez.
     *
     * Retorna (BdpArticleStock, audit_id, resultado_previo).
     * [128A-1/F3] Guard de stock negativo: si el resultado quedara < 0 se
     * revierte la transacción y se devuelve `AjusteStockError::StockNegativo`.
     */
    pub async fn ajustar_stock(
        pool: &PgPool,
        user_id: Uuid,
        articulo_glory_codigo: &str,
        delta: Decimal,
        motivo: &str,
        warehouse_id: Option<&str>,
        idempotency_key: Option<&str>,
    ) -> Result<(crate::models::BdpArticleStock, Uuid, Option<String>), AjusteStockError> {
        let mut tx = pool.begin().await?;

        let warehouse = warehouse_id.unwrap_or("0");
        /* [128A-1/F3] Etiqueta coherente con el id: 'General' solo para el
         * almacén por defecto "0"; si no, se usa el id como nombre. */
        let warehouse_name = if warehouse == "0" {
            "General"
        } else {
            warehouse
        };
        let audit_payload = serde_json::json!({
            "articulo_glory_codigo": articulo_glory_codigo,
            "delta": delta,
            "motivo": motivo,
            "warehouse_id": warehouse,
        });

        let maybe_audit_id: Option<Uuid> = sqlx::query_scalar(
            r"INSERT INTO bdp_audit_log
               (user_id, operacion, direccion, datos_enviados, resultado, origen_operacion,
                target_entity_type, target_entity_id, authorization_reason, idempotency_key)
               VALUES ($1, 'stock_ajuste', 'internal', $2, 'exito', 'local', 'articulo', NULL, $3, $4)
               ON CONFLICT (user_id, idempotency_key) WHERE idempotency_key IS NOT NULL DO NOTHING
               RETURNING id",
        )
        .bind(user_id)
        .bind(audit_payload)
        .bind(format!(
            "Ajuste manual de stock local ({motivo}) — operación interna, no requiere autorización BDP"
        ))
        .bind(idempotency_key)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(audit_id) = maybe_audit_id else {
            let (existing_id, resultado): (Uuid, String) = sqlx::query_as(
                "SELECT id, resultado FROM bdp_audit_log WHERE user_id = $1 AND idempotency_key = $2",
            )
            .bind(user_id)
            .bind(idempotency_key.unwrap_or_default())
            .fetch_one(&mut *tx)
            .await?;
            let stock = Self::get_stock_tx(&mut tx, user_id, articulo_glory_codigo, warehouse)
                .await?
                .ok_or(sqlx::Error::RowNotFound)?;
            return Ok((stock, existing_id, Some(resultado)));
        };

        /* INSERT ... ON CONFLICT DO UPDATE con EXCLUDED.delta como valor:
         * en el ON CONFLICT de PostgreSQL no se puede referenciar la fila
         * objetivo dentro de VALUES, así que se pasa el delta en la columna
         * stock y la suma se hace contra la fila existente. */
        let stock = sqlx::query_as::<_, crate::models::BdpArticleStock>(
            "INSERT INTO bdp_article_stock \
                (id, user_id, articulo_glory_codigo, warehouse_id, warehouse_name, stock, ajustado_local, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, true, NOW(), NOW()) \
             ON CONFLICT (user_id, articulo_glory_codigo, warehouse_id) DO UPDATE SET \
                stock = bdp_article_stock.stock + EXCLUDED.stock, \
                warehouse_name = EXCLUDED.warehouse_name, \
                ajustado_local = true, \
                updated_at = NOW() \
             RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(articulo_glory_codigo)
        .bind(warehouse)
        .bind(warehouse_name)
        .bind(delta)
        .fetch_one(&mut *tx)
        .await?;

        /* [128A-1/F3] Guard de stock negativo: inventario negativo es un
         * estado inválido (un error de tipeo no puede dejar la fuente de
         * verdad local en negativo). Se revierte la transacción completa. */
        if stock.stock < Decimal::ZERO {
            let _ = tx.rollback().await;
            return Err(AjusteStockError::StockNegativo(format!(
                "el stock del artículo {articulo_glory_codigo} quedaría en {}",
                stock.stock
            )));
        }

        tx.commit().await?;

        Ok((stock, audit_id, None))
    }

    async fn get_stock_tx(
        tx: &mut Transaction<'_, Postgres>,
        user_id: Uuid,
        articulo_glory_codigo: &str,
        warehouse_id: &str,
    ) -> Result<Option<crate::models::BdpArticleStock>, sqlx::Error> {
        sqlx::query_as::<_, crate::models::BdpArticleStock>(
            "SELECT * FROM bdp_article_stock \
             WHERE user_id = $1 AND articulo_glory_codigo = $2 AND warehouse_id = $3",
        )
        .bind(user_id)
        .bind(articulo_glory_codigo)
        .bind(warehouse_id)
        .fetch_optional(&mut **tx)
        .await
    }
}
