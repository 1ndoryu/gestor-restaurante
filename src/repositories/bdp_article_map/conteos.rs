// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* Conteos de inventario persistidos (D3/D4): lectura y aplicación al stock. */

use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::tipos::{AjusteStockError, BdpArticleMapRepository};

impl BdpArticleMapRepository {
    /* [208A-2/C3] Conteos de inventario persistidos (decisiones D3/D4). */

    /// Lista las cabeceras de los conteos del usuario, con total de líneas.
    pub async fn listar_conteos(
        pool: &PgPool,
        user_id: Uuid,
        warehouse_id: Option<&str>,
        limite: i64,
    ) -> Result<Vec<crate::models::BdpConteoInventario>, sqlx::Error> {
        /* [L7] `warehouse_id` nulo lista todos (el historial histórico de "0"
         * incluido); con valor, solo ese almacén. */
        sqlx::query_as::<_, crate::models::BdpConteoInventario>(
            r"SELECT c.id, c.fecha, c.observaciones, c.estado, c.warehouse_id, c.creado_el,
                      (SELECT COUNT(*)::bigint FROM bdp_conteos_inventario_lineas l
                        WHERE l.conteo_id = c.id) AS total_lineas
               FROM bdp_conteos_inventario c
               WHERE c.user_id = $1
                 AND ($2::text IS NULL OR c.warehouse_id = $2)
               ORDER BY c.creado_el DESC
               LIMIT $3",
        )
        .bind(user_id)
        .bind(warehouse_id)
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
        Option<(
            crate::models::BdpConteoInventario,
            Vec<crate::models::BdpConteoInventarioLinea>,
        )>,
        sqlx::Error,
    > {
        let conteo = sqlx::query_as::<_, crate::models::BdpConteoInventario>(
            r"SELECT c.id, c.fecha, c.observaciones, c.estado, c.warehouse_id, c.creado_el,
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
    // `total_lineas` no puede envolver i64 en la práctica (conteo de líneas en memoria).
    #[allow(clippy::too_many_lines, clippy::cast_possible_wrap)]
    pub async fn crear_conteo(
        pool: &PgPool,
        user_id: Uuid,
        observaciones: &str,
        idempotency_key: Option<&str>,
        articulos: &[(String, Decimal)],
        warehouse_id: Option<&str>,
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
        /* [L7] El almacén es parte del conteo, no un global: se guarda con la
         * cabecera y manda en las tres cosas que antes eran fijas — el esperado
         * que se lee, el payload de auditoría y la fila de stock que se ajusta. */
        let almacen = warehouse_id.map_or("", str::trim);
        let almacen = if almacen.is_empty() { "0" } else { almacen };
        /* Nombre coherente con el id, igual que en el ajuste manual: 'General'
         * solo para el agregado local "0"; el resto usa su id como nombre. */
        let nombre_almacen = if almacen == "0" { "General" } else { almacen };
        /* Idempotencia (D4): con la misma clave, la inserción no hace nada y
         * devolvemos el conteo ya existente sin volver a aplicar. Atómico: el
         * índice único parcial + ON CONFLICT evita la doble aplicación incluso
         * con dos POSTs concurrentes de la misma clave. */
        let insertadas = sqlx::query(
            r"INSERT INTO bdp_conteos_inventario
                (id, user_id, observaciones, idempotency_key, warehouse_id)
              VALUES ($1, $2, $3, $4, $5)
              ON CONFLICT (user_id, idempotency_key) WHERE idempotency_key IS NOT NULL DO NOTHING",
        )
        .bind(conteo_id)
        .bind(user_id)
        .bind(observaciones)
        .bind(idempotency_key)
        .bind(almacen)
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
            let esperado = esperado_conteo(&mut tx, user_id, codigo, almacen).await?;
            let diferencia = contado - esperado;
            let linea_id = Uuid::new_v4();
            registrar_linea_conteo(
                &mut tx, linea_id, conteo_id, codigo, esperado, *contado, diferencia,
            )
            .await?;

            if diferencia != Decimal::ZERO {
                let stock_resultante = ajustar_stock_conteo(
                    &mut tx,
                    user_id,
                    conteo_id,
                    codigo,
                    esperado,
                    diferencia,
                    (almacen, nombre_almacen),
                )
                .await?;
                if stock_resultante < Decimal::ZERO {
                    let _ = tx.rollback().await;
                    return Err(AjusteStockError::StockNegativo(format!(
                        "el stock del artículo {codigo} quedaría en {stock_resultante} tras el conteo"
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
            warehouse_id: almacen.to_string(),
            creado_el: chrono::Utc::now(),
            total_lineas: lineas.len() as i64,
        };
        tx.commit().await?;
        Ok((conteo, lineas, false, aplicadas))
    }
}

/// Esperado = stock local (fuente de verdad) con fallback al snapshot BDP,
/// igual que en la UI de Inventario.
async fn esperado_conteo(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    codigo: &str,
    warehouse_id: &str,
) -> Result<Decimal, sqlx::Error> {
    sqlx::query_scalar(
        r"SELECT COALESCE(
                (SELECT stock FROM bdp_article_stock
                  WHERE user_id = $1 AND articulo_glory_codigo = $2 AND warehouse_id = $3),
                (SELECT stock_actual FROM bdp_article_map
                  WHERE user_id = $1 AND articulo_glory_codigo = $2),
                0)::numeric",
    )
    .bind(user_id)
    .bind(codigo)
    .bind(warehouse_id)
    .fetch_one(&mut **tx)
    .await
}

/// Inserta la línea del conteo con su esperado, contado y diferencia.
async fn registrar_linea_conteo(
    tx: &mut Transaction<'_, Postgres>,
    linea_id: Uuid,
    conteo_id: Uuid,
    codigo: &str,
    esperado: Decimal,
    contado: Decimal,
    diferencia: Decimal,
) -> Result<(), sqlx::Error> {
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
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Registra la auditoría idempotente (`conteo:{id}:{codigo}`) del ajuste de una línea.
async fn registrar_auditoria_conteo(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    conteo_id: Uuid,
    codigo: &str,
    diferencia: Decimal,
    warehouse_id: &str,
) -> Result<(), sqlx::Error> {
    /* Auditoría idempotente por conteo+línea. */
    let audit_payload = serde_json::json!({
        "articulo_glory_codigo": codigo,
        "delta": diferencia,
        "motivo": "conteo",
        "warehouse_id": warehouse_id,
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
    .execute(&mut **tx)
    .await?;

    Ok(())
}

/// Aplica la diferencia de una línea al stock local con auditoría idempotente.
/// Devuelve el stock resultante; el caller decide si revierte.
/// El almacén viaja como par (id, nombre) para no arrastrar ocho parámetros.
async fn ajustar_stock_conteo(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    conteo_id: Uuid,
    codigo: &str,
    esperado: Decimal,
    diferencia: Decimal,
    almacen: (&str, &str),
) -> Result<Decimal, sqlx::Error> {
    registrar_auditoria_conteo(tx, user_id, conteo_id, codigo, diferencia, almacen.0).await?;
    aplicar_delta_stock(tx, user_id, codigo, esperado, diferencia, almacen).await
}

/// Crea la fila base si falta y suma el delta. Las dos escrituras van en orden
/// dentro de la misma transacción: no son independientes.
async fn aplicar_delta_stock(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    codigo: &str,
    esperado: Decimal,
    diferencia: Decimal,
    almacen: (&str, &str),
) -> Result<Decimal, sqlx::Error> {
    let (warehouse_id, warehouse_name) = almacen;
    /* Base: si no hay fila local, se crea con el esperado para que
     * el delta deje el stock = contado (no un valor derivado). */
    sqlx::query(
        r"INSERT INTO bdp_article_stock
            (id, user_id, articulo_glory_codigo, warehouse_id, warehouse_name, stock,
             ajustado_local, created_at, updated_at)
          VALUES ($1, $2, $3, $4, $5, $6, true, NOW(), NOW())
          ON CONFLICT (user_id, articulo_glory_codigo, warehouse_id) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(codigo)
    .bind(warehouse_id)
    .bind(warehouse_name)
    .bind(esperado)
    .execute(&mut **tx)
    .await?;

    let stock = sqlx::query_as::<_, crate::models::BdpArticleStock>(
        r"INSERT INTO bdp_article_stock
            (id, user_id, articulo_glory_codigo, warehouse_id, warehouse_name, stock,
             ajustado_local, created_at, updated_at)
          VALUES ($1, $2, $3, $4, $5, $6, true, NOW(), NOW())
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
    .bind(warehouse_id)
    .bind(warehouse_name)
    .bind(diferencia)
    .fetch_one(&mut **tx)
    .await?;
    Ok(stock.stock)
}
