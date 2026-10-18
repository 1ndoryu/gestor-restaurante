// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* Factura local mínima (128A-1/F6): numeración, guards M9 e idempotencia C1. */

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::models::Venta;

use super::tipos::VentaRepository;

impl VentaRepository {
    /* [128A-1/F6] Factura local mínima (A7/D9).
     *
     * Emite una factura local: estado `facturada_local=true` (final, transición
     * única), numeración local secuencial `F-{año}-{n:04}` por usuario y
     * auditoría obligatoria con `origen_operacion='local'`. El índice parcial
     * UNIQUE(user_id, factura_numero) protege la numeración en carreras; el
     * servicio reintenta si hay colisión de número (unique violation 23505).
     *
     * Guards (M9): no se factura una venta anulada, ya facturada (local o BDP),
     * ni con pagos parciales pendientes en el ledger (si el ledger tiene filas,
     * deben cubrir el total; una venta sin pagos parciales se considera pagada
     * por `metodo_pago`, diseño A6: pago completo = venta con metodo_pago).
     *
     * Idempotencia C1: si la `idempotency_key` ya se usó, devuelve la fila
     * actual + resultado previo para que el handler decida (éxito idempotente
     * o conflicto).
     *
     * Retorna (Venta, audit_id, resultado_previo, ya_facturada).
     */
    pub async fn facturar_local(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        idempotency_key: Option<&str>,
    ) -> Result<(Venta, Uuid, Option<String>, bool), sqlx::Error> {
        let mut tx = pool.begin().await?;

        /* Clave de idempotencia: `None`/`""` se normaliza a una clave única
         * generada. La auditoría usa (user_id, idempotency_key) como conflicto;
         * una clave "" reutilizada en otra venta haría que la segunda llamada
         * devolviera la auditoría de la primera sin facturar (éxito falso). */
        let key = idempotency_key.filter(|k| !k.is_empty()).map_or_else(
            || format!("factura-local-{id}-{}", Uuid::new_v4()),
            str::to_string,
        );

        let venta_actual: Option<Venta> = sqlx::query_as::<_, Venta>(
            "SELECT * FROM ventas WHERE id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(venta) = venta_actual else {
            return Err(sqlx::Error::RowNotFound);
        };

        /* [128A-1/F6][F6-2/F6-5] (C1) La idempotencia se resuelve ANTES de los
         * guards M9: un reintento con la misma clave sobre una venta ya
         * facturada es ÉXITO idempotente (no 409). La clave está scoped por
         * venta: si la fila previa apunta a OTRA venta, es clave reutilizada →
         * conflicto, nunca éxito falso. */
        let clave_prev = clave_previa(&mut tx, user_id, &key).await?;
        if let Some((audit_id, resultado, target_entity_id)) = clave_prev {
            if target_entity_id != id {
                tx.rollback().await?;
                return Err(sqlx::Error::Protocol(
                    "idempotency_key_otra_venta".to_string(),
                ));
            }
            tx.commit().await?;
            let ya_facturada = venta.facturada_local;
            return Ok((venta, audit_id, Some(resultado), ya_facturada));
        }

        validar_facturable(&venta)?;
        validar_pagos(&mut tx, id, &venta).await?;
        let numero = siguiente_numero_factura(&mut tx, user_id).await?;
        let maybe_audit_id =
            registrar_audit_factura(&mut tx, user_id, id, &venta, &numero, &key).await?;
        let Some(audit_id) = maybe_audit_id else {
            return resolver_carrera_factura(tx, id, user_id, &key).await;
        };

        let venta_facturada = marcar_facturada(&mut tx, id, user_id, &numero).await?;
        let Some(venta_facturada) = venta_facturada else {
            tx.rollback().await?;
            return Err(sqlx::Error::Protocol("venta_ya_facturada".to_string()));
        };

        tx.commit().await?;

        Ok((venta_facturada, audit_id, None, false))
    }
}

/// Busca una auditoría previa con la misma clave de idempotencia del usuario.
async fn clave_previa(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    key: &str,
) -> Result<Option<(Uuid, String, Uuid)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, resultado, target_entity_id FROM bdp_audit_log \
         WHERE user_id = $1 AND idempotency_key = $2",
    )
    .bind(user_id)
    .bind(key)
    .fetch_optional(&mut **tx)
    .await
}

/// M9: no se factura una venta anulada ni una ya facturada (local o BDP).
fn validar_facturable(venta: &Venta) -> Result<(), sqlx::Error> {
    if venta.anulada {
        return Err(sqlx::Error::Protocol(
            "venta_anulada_no_facturable".to_string(),
        ));
    }
    if venta.facturada_local
        || venta.bdp_invoiced
        || venta.bdp_order_status.as_deref() == Some("invoiced")
    {
        return Err(sqlx::Error::Protocol("venta_ya_facturada".to_string()));
    }
    Ok(())
}

/* [128A-1/F6][F6-3] El guard de pagos solo mira filas con resultado
 * 'exito'/'ambiguo': filas históricas legacy 'error'/'ambiguo' de un
 * flujo BDP previo NO bloquean la facturación local para siempre.
 * Si quedan filas reales, deben cubrir el total. */
async fn validar_pagos(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    venta: &Venta,
) -> Result<(), sqlx::Error> {
    let (tiene_pagos, pagado): (bool, rust_decimal::Decimal) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM bdp_pagos WHERE venta_id = $1 \
                       AND resultado IN ('exito', 'ambiguo')), \
                COALESCE((SELECT SUM(amount) FROM bdp_pagos \
                          WHERE venta_id = $1 AND resultado IN ('exito', 'ambiguo')), 0)",
    )
    .bind(id)
    .fetch_one(&mut **tx)
    .await?;
    let total = venta.importe_base + venta.importe_iva;
    if tiene_pagos && (total - pagado) > rust_decimal::Decimal::new(1, 3) {
        return Err(sqlx::Error::Protocol(
            "venta_con_pagos_pendientes".to_string(),
        ));
    }
    Ok(())
}

/* [128A-1/F6][F6-4] Numeración local por (user_id, año):
 * F-{año}-{n:04} sin mezclar años ni reutilizar números tras
 * borrados (MAX del secuencial + 1; el retry 23505 del servicio
 * cubre la carrera). */
async fn siguiente_numero_factura(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<String, sqlx::Error> {
    let anio = chrono::Utc::now().format("%Y");
    let max_numero: Option<i32> = sqlx::query_scalar(
        "SELECT MAX((regexp_match(factura_numero, '^F-[0-9]{4}-([0-9]+)$'))[1]::integer) \
         FROM ventas WHERE user_id = $1 AND factura_numero LIKE $2",
    )
    .bind(user_id)
    .bind(format!("F-{anio}-%"))
    .fetch_one(&mut **tx)
    .await?;
    Ok(format!("F-{anio}-{:04}", max_numero.unwrap_or(0) + 1))
}

/// Auditoría obligatoria de la factura (origen local). `None` si la clave ya existía.
async fn registrar_audit_factura(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    id: Uuid,
    venta: &Venta,
    numero: &str,
    key: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let total = venta.importe_base + venta.importe_iva;
    let audit_payload = serde_json::json!({
        "venta_id": id,
        "factura_numero": numero,
        "importe_base": venta.importe_base,
        "importe_iva": venta.importe_iva,
        "total": total,
        "bdp_synced": venta.bdp_synced,
        "bdp_order_id": venta.bdp_order_id,
    });

    sqlx::query_scalar(
        r"INSERT INTO bdp_audit_log
           (user_id, operacion, direccion, datos_enviados, resultado, origen_operacion,
            target_entity_type, target_entity_id, authorization_reason, idempotency_key)
           VALUES ($1, 'factura_local', 'internal', $2, 'exito', 'local', 'venta', $3, $4, $5)
           ON CONFLICT (user_id, idempotency_key) WHERE idempotency_key IS NOT NULL DO NOTHING
           RETURNING id",
    )
    .bind(user_id)
    .bind(audit_payload)
    .bind(id)
    .bind(format!(
        "Factura local {numero} de la venta {id} — operación interna, no requiere autorización BDP"
    ))
    .bind(key)
    .fetch_optional(&mut **tx)
    .await
}

/// Marca la venta como facturada; `None` si ya lo estaba (la condición va en el UPDATE).
async fn marcar_facturada(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    user_id: Uuid,
    numero: &str,
) -> Result<Option<Venta>, sqlx::Error> {
    sqlx::query_as::<_, Venta>(
        "UPDATE ventas SET facturada_local = true, factura_numero = $3, \
         factura_fecha = NOW(), updated_at = NOW() \
         WHERE id = $1 AND user_id = $2 AND facturada_local = false \
         RETURNING *",
    )
    .bind(id)
    .bind(user_id)
    .bind(numero)
    .fetch_optional(&mut **tx)
    .await
}

/// Carrera: otra transacción insertó la clave mientras tanto; devuelve su resultado previo.
async fn resolver_carrera_factura(
    mut tx: Transaction<'_, Postgres>,
    id: Uuid,
    user_id: Uuid,
    key: &str,
) -> Result<(Venta, Uuid, Option<String>, bool), sqlx::Error> {
    let (existing_id, resultado, target_entity_id): (Uuid, String, Uuid) = sqlx::query_as(
        "SELECT id, resultado, target_entity_id FROM bdp_audit_log \
         WHERE user_id = $1 AND idempotency_key = $2",
    )
    .bind(user_id)
    .bind(key)
    .fetch_one(&mut *tx)
    .await?;
    if target_entity_id != id {
        tx.rollback().await?;
        return Err(sqlx::Error::Protocol(
            "idempotency_key_otra_venta".to_string(),
        ));
    }
    let venta_actual =
        sqlx::query_as::<_, Venta>("SELECT * FROM ventas WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;
    tx.commit().await?;
    let ya_facturada = venta_actual.facturada_local;
    Ok((venta_actual, existing_id, Some(resultado), ya_facturada))
}
