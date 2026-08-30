// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
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

/// [128A-1/F3] Errores de dominio del ajuste de stock local.
#[derive(Debug, thiserror::Error)]
pub enum AjusteStockError {
    #[error("Stock negativo inválido: {0}")]
    StockNegativo(String),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
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
    /// [128A-1/F3] No sobrescribe filas ajustadas localmente: el UPDATE se
    /// condiciona a `NOT ajustado_local` para que el sync nunca pise la
    /// fuente de verdad editable (`bdp_article_stock`).
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
             WHERE NOT bdp_article_stock.ajustado_local \
               AND bdp_article_stock.stock IS DISTINCT FROM EXCLUDED.stock",
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

    /* [208A-2/C3] Conteos de inventario persistidos (decisiones D3/D4). */

    /// Lista las cabeceras de los conteos del usuario, con total de líneas.
    pub async fn listar_conteos(
        pool: &PgPool,
        user_id: Uuid,
        limite: i64,
    ) -> Result<Vec<crate::models::BdpConteoInventario>, sqlx::Error> {
        sqlx::query_as::<_, crate::models::BdpConteoInventario>(
            r"SELECT c.id, c.fecha, c.observaciones, c.estado, c.creado_el,
                      (SELECT COUNT(*)::bigint FROM bdp_conteos_inventario_lineas l
                        WHERE l.conteo_id = c.id) AS total_lineas
               FROM bdp_conteos_inventario c
               WHERE c.user_id = $1
               ORDER BY c.creado_el DESC
               LIMIT $2",
        )
        .bind(user_id)
        .bind(limite)
        .fetch_all(pool)
        .await
    }

    /// Detalle de un conteo con sus líneas (para retomar/recontar).
    pub async fn obtener_conteo(
        pool: &PgPool,
        user_id: Uuid,
        conteo_id: Uuid,
    ) -> Result<
        Option<(crate::models::BdpConteoInventario, Vec<crate::models::BdpConteoInventarioLinea>)>,
        sqlx::Error,
    > {
        let conteo = sqlx::query_as::<_, crate::models::BdpConteoInventario>(
            r"SELECT c.id, c.fecha, c.observaciones, c.estado, c.creado_el,
                      (SELECT COUNT(*)::bigint FROM bdp_conteos_inventario_lineas l
                        WHERE l.conteo_id = c.id) AS total_lineas
               FROM bdp_conteos_inventario c
               WHERE c.user_id = $1 AND c.id = $2",
        )
        .bind(user_id)
        .bind(conteo_id)
        .fetch_optional(pool)
        .await?;
        let Some(conteo) = conteo else {
            return Ok(None);
        };
        let lineas = sqlx::query_as::<_, crate::models::BdpConteoInventarioLinea>(
            r"SELECT id, articulo_glory_codigo, esperado, contado, diferencia, aplicado_al_stock
               FROM bdp_conteos_inventario_lineas
               WHERE conteo_id = $1
               ORDER BY articulo_glory_codigo",
        )
        .bind(conteo_id)
        .fetch_all(pool)
        .await?;
        Ok(Some((conteo, lineas)))
    }

    /// Guarda un conteo y aplica la diferencia al stock local en la misma
    /// transacción (D4, motivo 'conteo', auditoría idempotente por línea con
    /// clave `conteo:{id}:{codigo}`). Si una línea dejaría stock negativo se
    /// revierte todo el conteo con `AjusteStockError::StockNegativo`.
    pub async fn crear_conteo(
        pool: &PgPool,
        user_id: Uuid,
        observaciones: &str,
        idempotency_key: Option<&str>,
        articulos: &[(String, Decimal)],
    ) -> Result<
        (
            crate::models::BdpConteoInventario,
            Vec<crate::models::BdpConteoInventarioLinea>,
            bool,
            usize,
        ),
        AjusteStockError,
    > {
        let mut tx = pool.begin().await?;
        let conteo_id = Uuid::new_v4();
        /* Idempotencia (D4): con la misma clave, la inserción no hace nada y
         * devolvemos el conteo ya existente sin volver a aplicar. Atómico: el
         * índice único parcial + ON CONFLICT evita la doble aplicación incluso
         * con dos POSTs concurrentes de la misma clave. */
        let insertadas = sqlx::query(
            r"INSERT INTO bdp_conteos_inventario (id, user_id, observaciones, idempotency_key)
              VALUES ($1, $2, $3, $4)
              ON CONFLICT (user_id, idempotency_key) WHERE idempotency_key IS NOT NULL DO NOTHING",
        )
        .bind(conteo_id)
        .bind(user_id)
        .bind(observaciones)
        .bind(idempotency_key)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if insertadas == 0 {
            let existing_id: Uuid = sqlx::query_scalar(
                r"SELECT id FROM bdp_conteos_inventario
                  WHERE user_id = $1 AND idempotency_key = $2",
            )
            .bind(user_id)
            .bind(idempotency_key.unwrap_or_default())
            .fetch_one(&mut *tx)
            .await?;
            tx.commit().await?;
            let (conteo, lineas) = Self::obtener_conteo(pool, user_id, existing_id)
                .await?
                .ok_or(sqlx::Error::RowNotFound)?;
            return Ok((conteo, lineas, true, 0));
        }

        let mut lineas = Vec::new();
        let mut aplicadas = 0usize;
        for (codigo, contado) in articulos {
            /* Esperado = stock local (fuente de verdad) con fallback al
             * snapshot BDP, igual que en la UI de Inventario. */
            let esperado: Decimal = sqlx::query_scalar(
                r"SELECT COALESCE(
                        (SELECT stock FROM bdp_article_stock
                          WHERE user_id = $1 AND articulo_glory_codigo = $2 AND warehouse_id = '0'),
                        (SELECT stock_actual FROM bdp_article_map
                          WHERE user_id = $1 AND articulo_glory_codigo = $2),
                        0)::numeric",
            )
            .bind(user_id)
            .bind(codigo)
            .fetch_one(&mut *tx)
            .await?;
            let diferencia = contado - esperado;
            let linea_id = Uuid::new_v4();
            sqlx::query(
                r"INSERT INTO bdp_conteos_inventario_lineas
                    (id, conteo_id, articulo_glory_codigo, esperado, contado, diferencia, aplicado_al_stock)
                  VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(linea_id)
            .bind(conteo_id)
            .bind(codigo)
            .bind(esperado)
            .bind(contado)
            .bind(diferencia)
            .bind(diferencia != Decimal::ZERO)
            .execute(&mut *tx)
            .await?;

            if diferencia != Decimal::ZERO {
                /* Auditoría idempotente por conteo+línea. */
                let audit_payload = serde_json::json!({
                    "articulo_glory_codigo": codigo,
                    "delta": diferencia,
                    "motivo": "conteo",
                    "warehouse_id": "0",
                });
                let idem = format!("conteo:{conteo_id}:{codigo}");
                sqlx::query(
                    r"INSERT INTO bdp_audit_log
                        (user_id, operacion, direccion, datos_enviados, resultado, origen_operacion,
                         target_entity_type, target_entity_id, authorization_reason, idempotency_key)
                      VALUES ($1, 'stock_ajuste', 'internal', $2, 'exito', 'local', 'articulo', NULL, $3, $4)
                      ON CONFLICT (user_id, idempotency_key) WHERE idempotency_key IS NOT NULL DO NOTHING",
                )
                .bind(user_id)
                .bind(audit_payload)
                .bind("Ajuste por conteo de inventario — operación interna, no requiere autorización BDP")
                .bind(&idem)
                .execute(&mut *tx)
                .await?;

                /* Base: si no hay fila local, se crea con el esperado para que
                 * el delta deje el stock = contado (no un valor derivado). */
                sqlx::query(
                    r"INSERT INTO bdp_article_stock
                        (id, user_id, articulo_glory_codigo, warehouse_id, warehouse_name, stock,
                         ajustado_local, created_at, updated_at)
                      VALUES ($1, $2, $3, '0', 'General', $4, true, NOW(), NOW())
                      ON CONFLICT (user_id, articulo_glory_codigo, warehouse_id) DO NOTHING",
                )
                .bind(Uuid::new_v4())
                .bind(user_id)
                .bind(codigo)
                .bind(esperado)
                .execute(&mut *tx)
                .await?;

                let stock = sqlx::query_as::<_, crate::models::BdpArticleStock>(
                    r"INSERT INTO bdp_article_stock
                        (id, user_id, articulo_glory_codigo, warehouse_id, warehouse_name, stock,
                         ajustado_local, created_at, updated_at)
                      VALUES ($1, $2, $3, '0', 'General', $4, true, NOW(), NOW())
                      ON CONFLICT (user_id, articulo_glory_codigo, warehouse_id) DO UPDATE SET
                        stock = bdp_article_stock.stock + EXCLUDED.stock,
                        warehouse_name = EXCLUDED.warehouse_name,
                        ajustado_local = true,
                        updated_at = NOW()
                      RETURNING *",
                )
                .bind(Uuid::new_v4())
                .bind(user_id)
                .bind(codigo)
                .bind(diferencia)
                .fetch_one(&mut *tx)
                .await?;

                if stock.stock < Decimal::ZERO {
                    let _ = tx.rollback().await;
                    return Err(AjusteStockError::StockNegativo(format!(
                        "el stock del artículo {codigo} quedaría en {} tras el conteo",
                        stock.stock
                    )));
                }
                aplicadas += 1;
            }

            lineas.push(crate::models::BdpConteoInventarioLinea {
                id: linea_id,
                articulo_glory_codigo: codigo.clone(),
                esperado,
                contado: *contado,
                diferencia,
                aplicado_al_stock: diferencia != Decimal::ZERO,
            });
        }

        let conteo = crate::models::BdpConteoInventario {
            id: conteo_id,
            fecha: chrono::Utc::now().date_naive(),
            observaciones: observaciones.to_string(),
            estado: "aplicado".to_string(),
            creado_el: chrono::Utc::now(),
            total_lineas: lineas.len() as i64,
        };
        tx.commit().await?;
        Ok((conteo, lineas, false, aplicadas))
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
