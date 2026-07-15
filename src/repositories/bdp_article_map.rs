/* [F1.4] Repositorio de mapeo artículos Glory → BDP.
 * CRUD completo + búsqueda por código para uso interno de bdp_sync. */

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    ActualizarBdpArticleMapRequest, BdpArticleMap, CrearBdpArticleMapRequest,
};

pub struct BdpArticleMapRepository;

impl BdpArticleMapRepository {
    /// Lista todos los mapeos del usuario
    pub async fn listar(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<BdpArticleMap>, sqlx::Error> {
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
    pub async fn eliminar(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM bdp_article_map WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
