// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* Alta, edición y baja del mapeo Glory → BDP. */

use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{ActualizarBdpArticleMapRequest, BdpArticleMap, CrearBdpArticleMapRequest};

use super::tipos::BdpArticleMapRepository;

impl BdpArticleMapRepository {
    /// Crea un nuevo mapeo (upsert: si ya existe el código Glory, actualiza)
    /// [128A-1/F2] Si llegan campos locales, el registro pasa a `origen='local'`;
    /// si la fila existía como BDP, se marca `local_dirty` (edición local de un
    /// artículo importado — M6). Si no llegan campos locales, conserva el
    /// comportamiento clásico de mapeo (`origen`/`local_dirty` intactos).
    pub async fn crear(
        pool: &PgPool,
        user_id: Uuid,
        req: &CrearBdpArticleMapRequest,
    ) -> Result<BdpArticleMap, sqlx::Error> {
        let id = Uuid::new_v4();
        let tiene_campos_locales = req.descripcion.is_some()
            || req.precio_tarifa1.is_some()
            || req.iva_pct.is_some()
            || req.departamento.is_some()
            || req.familia.is_some()
            || req.subfamilia.is_some()
            || req.activo.is_some()
            || req.barcode.is_some();
        /* [128A-1/F2][M7] El toggle de `activo` es estado de disponibilidad
         * local, no edición de datos: cambia `origen` pero NO marca
         * `local_dirty` para que el import siga sincronizando precios/datos y
         * solo respete la desactivación (M7). */
        let marca_dirty = req.descripcion.is_some()
            || req.precio_tarifa1.is_some()
            || req.iva_pct.is_some()
            || req.departamento.is_some()
            || req.familia.is_some()
            || req.subfamilia.is_some()
            || req.barcode.is_some();
        let bdp_codigo = req.articulo_bdp_codigo.as_deref().unwrap_or("");
        let descripcion = req.descripcion.as_deref().unwrap_or("");
        let precio = req.precio_tarifa1.unwrap_or(Decimal::ZERO);
        let iva = req.iva_pct.unwrap_or(Decimal::ZERO);
        let departamento = req.departamento.unwrap_or(0);
        let familia = req.familia.unwrap_or(0);
        let subfamilia = req.subfamilia.unwrap_or(0);
        let activo = req.activo.unwrap_or(true);
        let barcode = req.barcode.as_deref().unwrap_or("");
        /* [128A-1/F2] Los campos del DO UPDATE se ligan como Option: si el
         * POST no los trae, se conserva el valor existente (COALESCE contra la
         * fila objetivo). El INSERT mantiene los defaults clásicos para filas
         * nuevas. Así un mapeo puro sobre un glory code existente ya no vacía
         * descripcion/precio/iva ni reactiva un artículo desactivado (M7). */
        sqlx::query_as::<_, BdpArticleMap>(
            "INSERT INTO bdp_article_map \
                (id, user_id, articulo_glory_codigo, articulo_bdp_codigo, articulo_bdp_nombre, \
                 descripcion, precio_tarifa1, iva_pct, departamento, familia, subfamilia, \
                 activo, barcode, origen, local_dirty) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15) \
             ON CONFLICT (user_id, articulo_glory_codigo) DO UPDATE SET \
                articulo_bdp_codigo = COALESCE($17, bdp_article_map.articulo_bdp_codigo), \
                articulo_bdp_nombre = COALESCE($18, bdp_article_map.articulo_bdp_nombre), \
                descripcion = COALESCE($19, bdp_article_map.descripcion), \
                precio_tarifa1 = COALESCE($20, bdp_article_map.precio_tarifa1), \
                iva_pct = COALESCE($21, bdp_article_map.iva_pct), \
                departamento = COALESCE($22, bdp_article_map.departamento), \
                familia = COALESCE($23, bdp_article_map.familia), \
                subfamilia = COALESCE($24, bdp_article_map.subfamilia), \
                activo = COALESCE($25, bdp_article_map.activo), \
                barcode = COALESCE($26, bdp_article_map.barcode), \
                origen = CASE WHEN $15 THEN 'local' ELSE bdp_article_map.origen END, \
                local_dirty = CASE \
                    WHEN $16 AND bdp_article_map.origen = 'bdp' THEN true \
                    ELSE bdp_article_map.local_dirty \
                END, \
                updated_at = NOW() \
             RETURNING *",
        )
        .bind(id)
        .bind(user_id)
        .bind(&req.articulo_glory_codigo)
        .bind(bdp_codigo)
        .bind(req.articulo_bdp_nombre.as_deref().unwrap_or(""))
        .bind(descripcion)
        .bind(precio)
        .bind(iva)
        .bind(departamento)
        .bind(familia)
        .bind(subfamilia)
        .bind(activo)
        .bind(barcode)
        .bind(if tiene_campos_locales { "local" } else { "bdp" })
        .bind(tiene_campos_locales)
        .bind(marca_dirty)
        .bind(req.articulo_bdp_codigo.as_deref())
        .bind(req.articulo_bdp_nombre.as_deref())
        .bind(req.descripcion.as_deref())
        .bind(req.precio_tarifa1)
        .bind(req.iva_pct)
        .bind(req.departamento)
        .bind(req.familia)
        .bind(req.subfamilia)
        .bind(req.activo)
        .bind(req.barcode.as_deref())
        .fetch_one(pool)
        .await
    }

    /// Actualiza parcialmente un mapeo existente
    /// [128A-1/F2] Al editar campos locales, el registro pasa a `origen='local'`
    /// y se marca `local_dirty=true` (M6). Un PATCH que solo toca códigos BDP
    /// (mapeo) o solo `activo` (M7) no marca dirty.
    pub async fn actualizar(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        req: &ActualizarBdpArticleMapRequest,
    ) -> Result<Option<BdpArticleMap>, sqlx::Error> {
        let tiene_campos_locales = req.descripcion.is_some()
            || req.precio_tarifa1.is_some()
            || req.iva_pct.is_some()
            || req.departamento.is_some()
            || req.familia.is_some()
            || req.subfamilia.is_some()
            || req.activo.is_some()
            || req.barcode.is_some();
        /* [128A-1/F2][M7] Igual que en `crear`: `activo` no marca dirty. */
        let marca_dirty = req.descripcion.is_some()
            || req.precio_tarifa1.is_some()
            || req.iva_pct.is_some()
            || req.departamento.is_some()
            || req.familia.is_some()
            || req.subfamilia.is_some()
            || req.barcode.is_some();
        sqlx::query_as::<_, BdpArticleMap>(
            "UPDATE bdp_article_map SET \
                articulo_bdp_codigo = COALESCE($3, articulo_bdp_codigo), \
                articulo_bdp_nombre = COALESCE($4, articulo_bdp_nombre), \
                descripcion = COALESCE($5, descripcion), \
                precio_tarifa1 = COALESCE($6, precio_tarifa1), \
                iva_pct = COALESCE($7, iva_pct), \
                departamento = COALESCE($8, departamento), \
                familia = COALESCE($9, familia), \
                subfamilia = COALESCE($10, subfamilia), \
                activo = COALESCE($11, activo), \
                barcode = COALESCE($12, barcode), \
                origen = CASE WHEN $13 THEN 'local' ELSE origen END, \
                local_dirty = CASE \
                    WHEN $14 AND origen = 'bdp' THEN true \
                    ELSE local_dirty \
                END, \
                updated_at = NOW() \
             WHERE id = $1 AND user_id = $2 \
             RETURNING *",
        )
        .bind(id)
        .bind(user_id)
        .bind(req.articulo_bdp_codigo.as_deref())
        .bind(req.articulo_bdp_nombre.as_deref())
        .bind(req.descripcion.as_deref())
        .bind(req.precio_tarifa1)
        .bind(req.iva_pct)
        .bind(req.departamento)
        .bind(req.familia)
        .bind(req.subfamilia)
        .bind(req.activo)
        .bind(req.barcode.as_deref())
        .bind(tiene_campos_locales)
        .bind(marca_dirty)
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
}
