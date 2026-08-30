// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* 253A-5: Repositorio de gastos */

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{CategoriaGasto, Gasto};

/// Datos necesarios para insertar un gasto en BD
pub struct NuevoGasto<'a> {
    pub user_id: Uuid,
    pub fecha: chrono::NaiveDate,
    pub proveedor: &'a str,
    pub categoria_id: Option<Uuid>,
    pub tipo_documento: &'a str,
    pub metodo_pago: &'a str,
    pub numero_documento: &'a str,
    pub recurrente: bool,
    pub importe_base: rust_decimal::Decimal,
    pub importe_iva: rust_decimal::Decimal,
}

/* [283A-22] Datos para actualizar parcialmente un gasto.
 * Usa runtime queries (query_as con bind) para no requerir actualizar el cache .sqlx. */
pub struct ActualizarGastoData<'a> {
    pub id: Uuid,
    pub user_id: Uuid,
    pub fecha: Option<chrono::NaiveDate>,
    pub proveedor: Option<&'a str>,
    pub categoria_id: Option<Uuid>,
    pub tipo_documento: Option<&'a str>,
    pub metodo_pago: Option<&'a str>,
    pub numero_documento: Option<&'a str>,
    pub recurrente: Option<bool>,
    pub importe_base: Option<rust_decimal::Decimal>,
    pub importe_iva: Option<rust_decimal::Decimal>,
}

pub struct GastoRepository;

impl GastoRepository {
    pub async fn create<'e, E>(executor: E, data: &NuevoGasto<'_>) -> Result<Gasto, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            Gasto,
            "INSERT INTO gastos (id, user_id, fecha, proveedor, categoria_id, tipo_documento, \
             metodo_pago, numero_documento, recurrente, importe_base, importe_iva) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
             RETURNING *",
            id,
            data.user_id,
            data.fecha,
            data.proveedor,
            data.categoria_id,
            data.tipo_documento,
            data.metodo_pago,
            data.numero_documento,
            data.recurrente,
            data.importe_base,
            data.importe_iva
        )
        .fetch_one(executor)
        .await
    }

    pub async fn find_by_id<'e, E>(
        executor: E,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Gasto>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        sqlx::query_as!(
            Gasto,
            "SELECT * FROM gastos WHERE id = $1 AND user_id = $2",
            id,
            user_id
        )
        .fetch_optional(executor)
        .await
    }

    /* [044A-8+9] Whitelist de columnas — previene SQL injection.
     * [064A-3] Añadidos filtros por columna: tipo_documento, metodo_pago (multi-valor separado por coma). */
    #[allow(clippy::too_many_arguments)]
    pub async fn list(
        pool: &PgPool,
        user_id: Uuid,
        page: i64,
        per_page: i64,
        desde: Option<chrono::NaiveDate>,
        hasta: Option<chrono::NaiveDate>,
        categoria_id: Option<Uuid>,
        busqueda: Option<&str>,
        tipo_documento: Option<&str>,
        metodo_pago: Option<&str>,
        sort_by: Option<&str>,
        sort_order: Option<&str>,
    ) -> Result<(Vec<Gasto>, i64), sqlx::Error> {
        let offset = (page - 1) * per_page;

        /* Whitelist de columnas — previene SQL injection */
        let order_col = match sort_by {
            Some("proveedor") => "proveedor",
            Some("importe_base") => "importe_base",
            Some("tipo_documento") => "tipo_documento",
            Some("metodo_pago") => "metodo_pago",
            _ => "fecha",
        };
        let order_dir = if matches!(sort_order, Some("asc")) {
            "ASC"
        } else {
            "DESC"
        };

        let busqueda_pattern = busqueda.filter(|b| !b.is_empty()).map(|b| format!("%{b}%"));

        /* Normalizar filtros vacíos a None */
        let tipo_doc_filter = tipo_documento.filter(|t| !t.is_empty());
        let metodo_filter = metodo_pago.filter(|m| !m.is_empty());

        let query_str = format!(
            "SELECT * FROM gastos WHERE user_id = $1 \
             AND ($4::DATE IS NULL OR fecha >= $4) \
             AND ($5::DATE IS NULL OR fecha <= $5) \
             AND ($6::UUID IS NULL OR categoria_id = $6) \
             AND ($7::TEXT IS NULL \
                  OR proveedor ILIKE $7 \
                  OR tipo_documento ILIKE $7 \
                  OR numero_documento ILIKE $7) \
             AND ($8::TEXT IS NULL OR tipo_documento = ANY(string_to_array($8, ','))) \
             AND ($9::TEXT IS NULL OR metodo_pago = ANY(string_to_array($9, ','))) \
             ORDER BY {order_col} {order_dir}, created_at DESC \
             LIMIT $2 OFFSET $3"
        );

        let items = sqlx::query_as::<_, Gasto>(&query_str)
            .bind(user_id)
            .bind(per_page)
            .bind(offset)
            .bind(desde)
            .bind(hasta)
            .bind(categoria_id)
            .bind(busqueda_pattern.as_deref())
            .bind(tipo_doc_filter)
            .bind(metodo_filter)
            .fetch_all(pool)
            .await?;

        /* COUNT con los mismos filtros */
        let has_text_filter = busqueda_pattern.is_some();
        let has_column_filters = tipo_doc_filter.is_some() || metodo_filter.is_some();

        let count = if has_text_filter || has_column_filters {
            let rec = sqlx::query_scalar::<_, Option<i64>>(
                "SELECT COUNT(*) FROM gastos WHERE user_id = $1 \
                 AND ($2::DATE IS NULL OR fecha >= $2) \
                 AND ($3::DATE IS NULL OR fecha <= $3) \
                 AND ($4::UUID IS NULL OR categoria_id = $4) \
                 AND ($5::TEXT IS NULL \
                      OR proveedor ILIKE $5 \
                      OR tipo_documento ILIKE $5 \
                      OR numero_documento ILIKE $5) \
                 AND ($6::TEXT IS NULL OR tipo_documento = ANY(string_to_array($6, ','))) \
                 AND ($7::TEXT IS NULL OR metodo_pago = ANY(string_to_array($7, ',')))",
            )
            .bind(user_id)
            .bind(desde)
            .bind(hasta)
            .bind(categoria_id)
            .bind(busqueda_pattern.as_deref())
            .bind(tipo_doc_filter)
            .bind(metodo_filter)
            .fetch_one(pool)
            .await?;
            rec.unwrap_or(0)
        } else {
            let rec = sqlx::query_scalar::<_, Option<i64>>(
                "SELECT COUNT(*) FROM gastos WHERE user_id = $1 \
                 AND ($2::DATE IS NULL OR fecha >= $2) \
                 AND ($3::DATE IS NULL OR fecha <= $3) \
                 AND ($4::UUID IS NULL OR categoria_id = $4)",
            )
            .bind(user_id)
            .bind(desde)
            .bind(hasta)
            .bind(categoria_id)
            .fetch_one(pool)
            .await?;
            rec.unwrap_or(0)
        };

        Ok((items, count))
    }

    pub async fn delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM gastos WHERE id = $1 AND user_id = $2",
            id,
            user_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /* [283A-22] Actualizar parcialmente un gasto — COALESCE mantiene valores existentes
     * cuando el campo no se envía (None).
     * [014A-11] Convertido a query_as! para verificación SQL en compilación. */
    pub async fn update(
        pool: &PgPool,
        data: &ActualizarGastoData<'_>,
    ) -> Result<Option<Gasto>, sqlx::Error> {
        sqlx::query_as!(
            Gasto,
            "UPDATE gastos SET \
             fecha = COALESCE($3, fecha), \
             proveedor = COALESCE($4, proveedor), \
             categoria_id = COALESCE($5, categoria_id), \
             tipo_documento = COALESCE($6, tipo_documento), \
             metodo_pago = COALESCE($7, metodo_pago), \
             numero_documento = COALESCE($8, numero_documento), \
             recurrente = COALESCE($9, recurrente), \
             importe_base = COALESCE($10, importe_base), \
             importe_iva = COALESCE($11, importe_iva), \
             updated_at = NOW() \
             WHERE id = $1 AND user_id = $2 \
             RETURNING *",
            data.id,
            data.user_id,
            data.fecha,
            data.proveedor,
            data.categoria_id,
            data.tipo_documento,
            data.metodo_pago,
            data.numero_documento,
            data.recurrente,
            data.importe_base,
            data.importe_iva
        )
        .fetch_optional(pool)
        .await
    }

    /* [044A-10] Proveedores únicos para autocomplete.
     * Filtra por ILIKE si se proporciona búsqueda. Máximo 20 resultados. */
    pub async fn proveedores_unicos(
        pool: &PgPool,
        user_id: Uuid,
        busqueda: Option<&str>,
    ) -> Result<Vec<String>, sqlx::Error> {
        let pattern = busqueda.map(|b| format!("%{b}%"));
        sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT proveedor FROM gastos \
             WHERE user_id = $1 \
             AND proveedor != '' \
             AND ($2::TEXT IS NULL OR proveedor ILIKE $2) \
             ORDER BY proveedor ASC \
             LIMIT 20",
        )
        .bind(user_id)
        .bind(pattern.as_deref())
        .fetch_all(pool)
        .await
    }

    /// Suma de `importe_base` de gastos en un período
    pub async fn total_periodo(
        pool: &PgPool,
        user_id: Uuid,
        desde: chrono::NaiveDate,
        hasta: chrono::NaiveDate,
    ) -> Result<rust_decimal::Decimal, sqlx::Error> {
        let rec = sqlx::query!(
            "SELECT COALESCE(SUM(importe_base), 0) as total FROM gastos \
             WHERE user_id = $1 AND fecha >= $2 AND fecha <= $3",
            user_id,
            desde,
            hasta
        )
        .fetch_one(pool)
        .await?;
        Ok(rec.total.unwrap_or_default())
    }
}

pub struct CategoriaGastoRepository;

impl CategoriaGastoRepository {
    pub async fn list_all(pool: &PgPool) -> Result<Vec<CategoriaGasto>, sqlx::Error> {
        sqlx::query_as!(
            CategoriaGasto,
            "SELECT * FROM categorias_gasto ORDER BY nombre ASC"
        )
        .fetch_all(pool)
        .await
    }
}
