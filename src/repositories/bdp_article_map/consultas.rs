// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* Consultas de lectura del mapeo BDP y asignación de códigos (D3). */

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::BdpArticleMap;

use super::tipos::BdpArticleMapRepository;

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

    /// [198A-1/D6] Resuelve los códigos BDP numéricos de una lista de códigos
    /// Glory. Solo devuelve pares con código BDP numérico (los artículos
    /// locales puros se omiten; el inventario no puede regularizarlos en BDP).
    pub async fn codigos_bdp_para_glory(
        pool: &PgPool,
        user_id: Uuid,
        codigos: &[String],
    ) -> Result<Vec<(String, i64)>, sqlx::Error> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT articulo_glory_codigo, articulo_bdp_codigo FROM bdp_article_map \
             WHERE user_id = $1 AND articulo_glory_codigo = ANY($2)",
        )
        .bind(user_id)
        .bind(codigos)
        .fetch_all(pool)
        .await?;
        Ok(rows
            .into_iter()
            .filter_map(|(glory, bdp)| bdp.trim().parse::<i64>().ok().map(|code| (glory, code)))
            .collect())
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

    /* [198A-1/D3] Siguiente código libre del rango reservado (default
     * 90 000 000). Solo considera códigos numéricos >= rango inicial; si no
     * hay ninguno, devuelve el rango inicial. La subconsulta filtra por regex
     * antes del cast para no fallar con códigos alfanuméricos. */
    pub async fn siguiente_codigo_reservado(
        pool: &PgPool,
        user_id: Uuid,
        rango_inicial: i64,
    ) -> Result<i64, sqlx::Error> {
        let next: Option<i64> = sqlx::query_scalar(
            "SELECT COALESCE(MAX(c), $2 - 1) + 1 \
             FROM ( \
               SELECT articulo_bdp_codigo::bigint AS c FROM bdp_article_map \
               WHERE user_id = $1 AND articulo_bdp_codigo ~ '^[0-9]{1,13}$' \
             ) t WHERE t.c >= $2",
        )
        .bind(user_id)
        .bind(rango_inicial)
        .fetch_one(pool)
        .await?;
        Ok(next.unwrap_or(rango_inicial))
    }

    /// Asigna el código BDP a un artículo local puro (D3) y lo marca `local_dirty`.
    pub async fn asignar_codigo_bdp(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        codigo: i64,
    ) -> Result<Option<BdpArticleMap>, sqlx::Error> {
        sqlx::query_as::<_, BdpArticleMap>(
            "UPDATE bdp_article_map SET articulo_bdp_codigo = $3, origen = 'local', \
             local_dirty = true, updated_at = NOW() WHERE id = $1 AND user_id = $2 RETURNING *",
        )
        .bind(id)
        .bind(user_id)
        .bind(codigo.to_string())
        .fetch_optional(pool)
        .await
    }
}
