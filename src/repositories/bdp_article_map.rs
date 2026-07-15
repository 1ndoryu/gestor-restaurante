/* [F1.4] Repositorio de mapeo artículos Glory → BDP.
 * CRUD completo + búsqueda por código para uso interno de bdp_sync.
 * [157A-7] F9.1: upsert_from_bdp() para sync enriquecida de catálogo. */

use rust_decimal::Decimal;
use sqlx::PgPool;
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
                 activo, barcode, ultima_sync_at, created_at, updated_at) \
             VALUES ($1, $2, $3, $3, $4, $4, $5, $6, $7, $8, $9, $10, $11, NOW(), NOW(), NOW()) \
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
                ultima_sync_at = NOW(), \
                updated_at = NOW() \
             WHERE \
                bdp_article_map.descripcion IS DISTINCT FROM EXCLUDED.descripcion \
                OR bdp_article_map.precio_tarifa1 IS DISTINCT FROM EXCLUDED.precio_tarifa1 \
                OR bdp_article_map.iva_pct IS DISTINCT FROM EXCLUDED.iva_pct \
                OR bdp_article_map.departamento IS DISTINCT FROM EXCLUDED.departamento \
                OR bdp_article_map.activo IS DISTINCT FROM EXCLUDED.activo",
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
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}

/* [157A-7] F9.1: Datos para upsert de artículo BDP enriquecido. */
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
}
