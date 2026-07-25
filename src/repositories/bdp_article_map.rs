/* [F1.4] Repositorio de mapeo artículos Glory → BDP.
 * CRUD completo + búsqueda por código para uso interno de bdp_sync.
 * [157A-7] F9.1: upsert_from_bdp() para sync enriquecida de catálogo. */

use rust_decimal::Decimal;
use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

use crate::models::{ActualizarBdpArticleMapRequest, BdpArticleMap, CrearBdpArticleMapRequest};

pub struct BdpArticleMapRepository;

impl BdpArticleMapRepository {
    /// Lista todos los mapeos del usuario
    pub async fn listar(pool: &PgPool, user_id: Uuid) -> Result<Vec<BdpArticleMap>, sqlx::Error> {
        sqlx::query_as::<_, BdpArticleMap>(
            "SELECT * FROM bdp_article_map WHERE user_id = $1 ORDER BY articulo_glory_codigo",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }

    /// Obtiene un mapeo por ID (validando que pertenezca al usuario)
    pub async fn obtener(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<BdpArticleMap>, sqlx::Error> {
        sqlx::query_as::<_, BdpArticleMap>(
            "SELECT * FROM bdp_article_map WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
    }

    /// Busca un mapeo por código Glory (usado por `bdp_sync::resolve_article`)
    pub async fn buscar_por_codigo(
        pool: &PgPool,
        user_id: Uuid,
        articulo_glory_codigo: &str,
    ) -> Result<Option<BdpArticleMap>, sqlx::Error> {
        sqlx::query_as::<_, BdpArticleMap>(
            "SELECT * FROM bdp_article_map WHERE user_id = $1 AND articulo_glory_codigo = $2",
        )
        .bind(user_id)
        .bind(articulo_glory_codigo)
        .fetch_optional(pool)
        .await
    }

    /// Crea un nuevo mapeo (upsert: si ya existe el código Glory, actualiza)
    pub async fn crear(
        pool: &PgPool,
        user_id: Uuid,
        req: &CrearBdpArticleMapRequest,
    ) -> Result<BdpArticleMap, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, BdpArticleMap>(
            "INSERT INTO bdp_article_map (id, user_id, articulo_glory_codigo, articulo_bdp_codigo, articulo_bdp_nombre) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (user_id, articulo_glory_codigo) DO UPDATE SET \
                articulo_bdp_codigo = EXCLUDED.articulo_bdp_codigo, \
                articulo_bdp_nombre = EXCLUDED.articulo_bdp_nombre, \
                updated_at = NOW() \
             RETURNING *",
        )
        .bind(id)
        .bind(user_id)
        .bind(&req.articulo_glory_codigo)
        .bind(&req.articulo_bdp_codigo)
        .bind(req.articulo_bdp_nombre.as_deref().unwrap_or(""))
        .fetch_one(pool)
        .await
    }

    /// Actualiza parcialmente un mapeo existente
    pub async fn actualizar(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        req: &ActualizarBdpArticleMapRequest,
    ) -> Result<Option<BdpArticleMap>, sqlx::Error> {
        sqlx::query_as::<_, BdpArticleMap>(
            "UPDATE bdp_article_map SET \
                articulo_bdp_codigo = COALESCE($3, articulo_bdp_codigo), \
                articulo_bdp_nombre = COALESCE($4, articulo_bdp_nombre), \
                updated_at = NOW() \
             WHERE id = $1 AND user_id = $2 \
             RETURNING *",
        )
        .bind(id)
        .bind(user_id)
        .bind(req.articulo_bdp_codigo.as_deref())
        .bind(req.articulo_bdp_nombre.as_deref())
        .fetch_optional(pool)
        .await
    }

    /// Elimina un mapeo
    pub async fn eliminar(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM bdp_article_map WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /* [157A-7] F9.1: upsert desde datos BDP con campos enriquecidos.
     * Usado por BdpSyncService::sync_catalog() para sincronizar catálogo completo.
     * Upsert por (user_id, articulo_glory_codigo) que es el código BDP string.
     * Devuelve true si se creó o actualizó, false si no hubo cambios. */
    pub async fn upsert_from_bdp(
        pool: &PgPool,
        user_id: Uuid,
        data: &BdpArticleUpsertData<'_>,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO bdp_article_map \
                (id, user_id, articulo_glory_codigo, articulo_bdp_codigo, articulo_bdp_nombre, \
                 descripcion, precio_tarifa1, iva_pct, departamento, familia, subfamilia, \
                 activo, barcode, stock_actual, ultima_sync_at, created_at, updated_at) \
             VALUES ($1, $2, $3, $3, $4, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW(), NOW(), NOW()) \
             ON CONFLICT (user_id, articulo_glory_codigo) DO UPDATE SET \
                articulo_bdp_nombre = EXCLUDED.articulo_bdp_nombre, \
                descripcion = EXCLUDED.descripcion, \
                precio_tarifa1 = EXCLUDED.precio_tarifa1, \
                iva_pct = EXCLUDED.iva_pct, \
                departamento = EXCLUDED.departamento, \
                familia = EXCLUDED.familia, \
                subfamilia = EXCLUDED.subfamilia, \
                activo = EXCLUDED.activo, \
                barcode = EXCLUDED.barcode, \
                stock_actual = EXCLUDED.stock_actual, \
                ultima_sync_at = NOW(), \
                updated_at = NOW() \
             WHERE \
                bdp_article_map.descripcion IS DISTINCT FROM EXCLUDED.descripcion \
                OR bdp_article_map.precio_tarifa1 IS DISTINCT FROM EXCLUDED.precio_tarifa1 \
                OR bdp_article_map.iva_pct IS DISTINCT FROM EXCLUDED.iva_pct \
                OR bdp_article_map.departamento IS DISTINCT FROM EXCLUDED.departamento \
                OR bdp_article_map.familia IS DISTINCT FROM EXCLUDED.familia \
                OR bdp_article_map.subfamilia IS DISTINCT FROM EXCLUDED.subfamilia \
                OR bdp_article_map.activo IS DISTINCT FROM EXCLUDED.activo \
                OR bdp_article_map.barcode IS DISTINCT FROM EXCLUDED.barcode \
                OR bdp_article_map.stock_actual IS DISTINCT FROM EXCLUDED.stock_actual",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(data.bdp_code)
        .bind(data.descripcion)
        .bind(data.precio_tarifa1)
        .bind(data.iva_pct)
        .bind(data.departamento)
        .bind(data.familia)
        .bind(data.subfamilia)
        .bind(data.activo)
        .bind(data.barcode)
        .bind(data.stock_actual)
        .execute(pool)
        .await?;

        /* [247A-10/S2] Propagar stock agregado al almacén por defecto. */
        if let Err(e) = Self::upsert_stock(pool, user_id, data.bdp_code, data.stock_actual).await {
            warn!(
                "[247A-10/S2] Error propagando stock del artículo {} a bdp_article_stock: {e}",
                data.bdp_code
            );
        }

        Ok(result.rows_affected() > 0)
    }

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

    /// Upsert del stock por almacén. Por defecto `warehouse_id` "0" / "General".
    /// [247A-10/S2] Se usa desde `sync_catalog` para guardar el stock agregado
    /// de `ExportArticles` mientras BDP no devuelva desglose por almacén.
    pub async fn upsert_stock(
        pool: &PgPool,
        user_id: Uuid,
        articulo_glory_codigo: &str,
        stock: Decimal,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO bdp_article_stock \
                (id, user_id, articulo_glory_codigo, warehouse_id, warehouse_name, stock, ultima_sync_at, created_at, updated_at) \
             VALUES ($1, $2, $3, '0', 'General', $4, NOW(), NOW(), NOW()) \
             ON CONFLICT (user_id, articulo_glory_codigo, warehouse_id) DO UPDATE SET \
                stock = EXCLUDED.stock, \
                warehouse_name = EXCLUDED.warehouse_name, \
                ultima_sync_at = NOW(), \
                updated_at = NOW() \
             WHERE bdp_article_stock.stock IS DISTINCT FROM EXCLUDED.stock",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(articulo_glory_codigo)
        .bind(stock)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}

/* [157A-7] F9.1: Datos para upsert de artículo BDP enriquecido.
 * [237A-4] Añadido stock_actual. */
pub struct BdpArticleUpsertData<'a> {
    pub bdp_code: &'a str,
    pub descripcion: &'a str,
    pub precio_tarifa1: Decimal,
    pub iva_pct: Decimal,
    pub departamento: i32,
    pub familia: i32,
    pub subfamilia: i32,
    pub activo: bool,
    pub barcode: &'a str,
    /* [237A-4] Stock actual del artículo en BDP */
    pub stock_actual: Decimal,
}
