// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* Upsert del mapeo desde el catálogo BDP enriquecido (F9.1, sync enriquecida). */

use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

use super::tipos::{BdpArticleMapRepository, BdpArticleUpsertData, BdpArticleUpsertStatus};

impl BdpArticleMapRepository {
    /* [157A-7] F9.1: upsert desde datos BDP con campos enriquecidos.
     * [128A-1/F2] Reglas M6/M7: si la fila existe con `local_dirty=true`, el
     * import NO la sobrescribe (OmitidoLocalDirty); si está desactivada
     * localmente y BDP la trae activa, tampoco se reactiva (OmitidoDesactivado).
     * Devuelve el estado del upsert (ver `BdpArticleUpsertStatus`). */
    pub async fn upsert_from_bdp(
        pool: &PgPool,
        user_id: Uuid,
        data: &BdpArticleUpsertData<'_>,
    ) -> Result<BdpArticleUpsertStatus, sqlx::Error> {
        /* Chequeo barato previo: omitir filas con ediciones locales o
         * desactivadas localmente sin pisar su versión local. */
        let existing: Option<(bool, bool)> = sqlx::query_as(
            "SELECT local_dirty, activo FROM bdp_article_map \
             WHERE user_id = $1 AND articulo_glory_codigo = $2",
        )
        .bind(user_id)
        .bind(data.bdp_code)
        .fetch_optional(pool)
        .await?;

        if let Some((local_dirty, _)) = existing {
            if local_dirty {
                return Ok(BdpArticleUpsertStatus::OmitidoLocalDirty);
            }
        }
        if let Some((_, activo_local)) = existing {
            if !activo_local && data.activo {
                return Ok(BdpArticleUpsertStatus::OmitidoDesactivado);
            }
        }
        let existia = existing.is_some();

        let result = sqlx::query(
            "INSERT INTO bdp_article_map \
                (id, user_id, articulo_glory_codigo, articulo_bdp_codigo, articulo_bdp_nombre, \
                 descripcion, precio_tarifa1, iva_pct, departamento, familia, subfamilia, \
                 activo, barcode, stock_actual, inventariable, origen, local_dirty, ultima_sync_at, created_at, updated_at) \
             VALUES ($1, $2, $3, $3, $4, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, 'bdp', false, NOW(), NOW(), NOW()) \
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
                /* Si BDP no envía la marca en una sincronización, se conserva la última conocida. */ \
                inventariable = COALESCE(EXCLUDED.inventariable, bdp_article_map.inventariable), \
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
                OR bdp_article_map.stock_actual IS DISTINCT FROM EXCLUDED.stock_actual \
                OR COALESCE(EXCLUDED.inventariable, bdp_article_map.inventariable) IS DISTINCT FROM bdp_article_map.inventariable",
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
        .bind(data.inventariable)
        .execute(pool)
        .await?;

        /* [247A-10/S2] Propagar stock agregado al almacén por defecto "0"/"General". */
        if let Err(e) = Self::upsert_stock(
            pool,
            user_id,
            data.bdp_code,
            "0",
            "General",
            data.stock_actual,
        )
        .await
        {
            warn!(
                "[247A-10/S2] Error propagando stock del artículo {} a bdp_article_stock: {e}",
                data.bdp_code
            );
        }

        /* El upsert nunca toca filas local_dirty (ya retornamos antes). El
         * INSERT de una fila nueva siempre afecta 1 fila → Creado; si existía
         * y cambió → Actualizado; si existía y era idéntica, el WHERE del
         * UPDATE evita el cambio → SinCambios. */
        Ok(if result.rows_affected() == 0 {
            BdpArticleUpsertStatus::SinCambios
        } else if existia {
            BdpArticleUpsertStatus::Actualizado
        } else {
            BdpArticleUpsertStatus::Creado
        })
    }
}
