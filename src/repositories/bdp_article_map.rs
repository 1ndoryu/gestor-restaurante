/* [F1.4] Repositorio de mapeo artículos Glory → BDP.
 * CRUD completo + búsqueda por código para uso interno de bdp_sync.
 * [157A-7] F9.1: upsert_from_bdp() para sync enriquecida de catálogo. */

use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Transaction};
use tracing::warn;
use uuid::Uuid;

use crate::models::{ActualizarBdpArticleMapRequest, BdpArticleMap, CrearBdpArticleMapRequest};

/* [128A-1/F2] Resultado del upsert del import BDP (M6/M7).
 * - `Creado`: la fila no existía y se insertó.
 * - `Actualizado`: la fila existía y cambió.
 * - `SinCambios`: la fila existía y era idéntica.
 * - `OmitidoLocalDirty`: la fila tiene ediciones locales (`local_dirty`); el
 *   import NO la sobrescribe y el conflicto se reporta en el resultado.
 * - `OmitidoDesactivado`: la fila está desactivada localmente (`activo=false`
 *   local) y BDP la trae activa; el import NO la reactiva (M7). */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BdpArticleUpsertStatus {
    Creado,
    Actualizado,
    SinCambios,
    OmitidoLocalDirty,
    OmitidoDesactivado,
}

impl BdpArticleUpsertStatus {
    /// True si el import cambió o creó la fila (para stock y contadores).
    #[must_use]
    pub fn es_cambio(self) -> bool {
        matches!(self, Self::Creado | Self::Actualizado)
    }

    /// True si el import dejó la fila intacta por una regla local (M6/M7).
    #[must_use]
    pub fn es_omitido(self) -> bool {
        matches!(self, Self::OmitidoLocalDirty | Self::OmitidoDesactivado)
    }
}

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
        sqlx::query_as::<_, BdpArticleMap>(
            "INSERT INTO bdp_article_map \
                (id, user_id, articulo_glory_codigo, articulo_bdp_codigo, articulo_bdp_nombre, \
                 descripcion, precio_tarifa1, iva_pct, departamento, familia, subfamilia, \
                 activo, barcode, origen, local_dirty) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, false) \
             ON CONFLICT (user_id, articulo_glory_codigo) DO UPDATE SET \
                articulo_bdp_codigo = EXCLUDED.articulo_bdp_codigo, \
                articulo_bdp_nombre = EXCLUDED.articulo_bdp_nombre, \
                descripcion = COALESCE(EXCLUDED.descripcion, bdp_article_map.descripcion), \
                precio_tarifa1 = COALESCE(EXCLUDED.precio_tarifa1, bdp_article_map.precio_tarifa1), \
                iva_pct = COALESCE(EXCLUDED.iva_pct, bdp_article_map.iva_pct), \
                departamento = COALESCE(EXCLUDED.departamento, bdp_article_map.departamento), \
                familia = COALESCE(EXCLUDED.familia, bdp_article_map.familia), \
                subfamilia = COALESCE(EXCLUDED.subfamilia, bdp_article_map.subfamilia), \
                activo = COALESCE(EXCLUDED.activo, bdp_article_map.activo), \
                barcode = COALESCE(EXCLUDED.barcode, bdp_article_map.barcode), \
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
                 activo, barcode, stock_actual, origen, local_dirty, ultima_sync_at, created_at, updated_at) \
             VALUES ($1, $2, $3, $3, $4, $4, $5, $6, $7, $8, $9, $10, $11, $12, 'bdp', false, NOW(), NOW(), NOW()) \
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
     */
    pub async fn ajustar_stock(
        pool: &PgPool,
        user_id: Uuid,
        articulo_glory_codigo: &str,
        delta: Decimal,
        motivo: &str,
        warehouse_id: Option<&str>,
        idempotency_key: Option<&str>,
    ) -> Result<(crate::models::BdpArticleStock, Uuid, Option<String>), sqlx::Error> {
        let mut tx = pool.begin().await?;

        let warehouse = warehouse_id.unwrap_or("0");
        let audit_payload = serde_json::json!({
            "articulo_glory_codigo": articulo_glory_codigo,
            "delta": delta,
            "motivo": motivo,
            "warehouse_id": warehouse,
        });

        let maybe_audit_id: Option<Uuid> = sqlx::query_scalar(
            r"INSERT INTO bdp_audit_log
               (user_id, operacion, direccion, datos_enviados, resultado,
                target_entity_type, target_entity_id, authorization_reason, idempotency_key)
               VALUES ($1, 'stock_ajuste', 'internal', $2, 'exito', 'articulo', NULL, $3, $4)
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
                (id, user_id, articulo_glory_codigo, warehouse_id, warehouse_name, stock, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, 'General', $5, NOW(), NOW()) \
             ON CONFLICT (user_id, articulo_glory_codigo, warehouse_id) DO UPDATE SET \
                stock = bdp_article_stock.stock + EXCLUDED.stock, \
                warehouse_name = EXCLUDED.warehouse_name, \
                updated_at = NOW() \
             RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(articulo_glory_codigo)
        .bind(warehouse)
        .bind(delta)
        .fetch_one(&mut *tx)
        .await?;

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
