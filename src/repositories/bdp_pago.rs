// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [247A-9] Repositorio del ledger local de pagos parciales BDP.
 * Usa sqlx::query (funcion) con mapeo manual para evitar la macro query_as!,
 * que requeriria metadata offline del nuevo schema. */

use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::bdp_pago::BdpPago;

pub struct BdpPagoRepository;

impl BdpPagoRepository {
    fn row_to_bdp_pago(row: &sqlx::postgres::PgRow) -> BdpPago {
        BdpPago {
            id: row.get("id"),
            venta_id: row.get("venta_id"),
            amount: row.get("amount"),
            tender_id: row.get("tender_id"),
            idempotency_key: row.get("idempotency_key"),
            bdp_order_id: row.get("bdp_order_id"),
            bdp_payment_id: row.get("bdp_payment_id"),
            resultado: row.get("resultado"),
            datos_respuesta: row.get("datos_respuesta"),
            error_mensaje: row.get("error_mensaje"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    /// Inserta un nuevo pago en estado 'exito'.
    pub async fn insertar(
        pool: &PgPool,
        venta_id: Uuid,
        amount: Decimal,
        tender_id: i32,
        idempotency_key: &str,
        bdp_order_id: Option<i64>,
        bdp_payment_id: Option<&str>,
    ) -> Result<BdpPago, AppError> {
        let row = sqlx::query(
            r"
            INSERT INTO bdp_pagos (venta_id, amount, tender_id, idempotency_key, bdp_order_id, bdp_payment_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, venta_id, amount, tender_id, idempotency_key, bdp_order_id, bdp_payment_id,
                      resultado, datos_respuesta, error_mensaje, created_at, updated_at
",
        )
        .bind(venta_id)
        .bind(amount)
        .bind(tender_id)
        .bind(idempotency_key)
        .bind(bdp_order_id)
        .bind(bdp_payment_id)
        .fetch_one(pool)
        .await?;

        Ok(Self::row_to_bdp_pago(&row))
    }

    /// Lista todos los pagos de una venta ordenados por fecha.
    pub async fn listar_por_venta(pool: &PgPool, venta_id: Uuid) -> Result<Vec<BdpPago>, AppError> {
        let rows = sqlx::query(
            r"
            SELECT id, venta_id, amount, tender_id, idempotency_key, bdp_order_id, bdp_payment_id,
                   resultado, datos_respuesta, error_mensaje, created_at, updated_at
            FROM bdp_pagos
            WHERE venta_id = $1
            ORDER BY created_at ASC
",
        )
        .bind(venta_id)
        .fetch_all(pool)
        .await?;

        Ok(rows.iter().map(Self::row_to_bdp_pago).collect())
    }

    /// Suma de pagos exitosos para una venta.
    pub async fn total_pagado(pool: &PgPool, venta_id: Uuid) -> Result<Decimal, AppError> {
        let row = sqlx::query(
            r"
            SELECT COALESCE(SUM(amount), 0) as total
            FROM bdp_pagos
            WHERE venta_id = $1 AND resultado = 'exito'
",
        )
        .bind(venta_id)
        .fetch_one(pool)
        .await?;

        let total: Decimal = row.get("total");
        Ok(total)
    }

    /// Busca un pago por su clave de idempotencia.
    pub async fn obtener_por_idempotency_key(
        pool: &PgPool,
        idempotency_key: &str,
    ) -> Result<Option<BdpPago>, AppError> {
        let row = sqlx::query(
            r"
            SELECT id, venta_id, amount, tender_id, idempotency_key, bdp_order_id, bdp_payment_id,
                   resultado, datos_respuesta, error_mensaje, created_at, updated_at
            FROM bdp_pagos
            WHERE idempotency_key = $1
",
        )
        .bind(idempotency_key)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| Self::row_to_bdp_pago(&r)))
    }

    /// Lista pagos ambiguos de un usuario para reconciliación.
    pub async fn listar_ambiguos(pool: &PgPool, user_id: Uuid) -> Result<Vec<BdpPago>, AppError> {
        let rows = sqlx::query(
            r"
            SELECT id, venta_id, amount, tender_id, idempotency_key, bdp_order_id, bdp_payment_id,
                   resultado, datos_respuesta, error_mensaje, created_at, updated_at
            FROM bdp_pagos
            WHERE venta_id IN (SELECT id FROM ventas WHERE user_id = $1)
              AND resultado = 'ambiguo'
            ORDER BY created_at ASC
            LIMIT 100
",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(rows.iter().map(Self::row_to_bdp_pago).collect())
    }

    /// Actualiza el resultado y datos de respuesta de un pago.
    pub async fn actualizar_resultado(
        pool: &PgPool,
        id: Uuid,
        resultado: &str,
        datos_respuesta: Option<&serde_json::Value>,
        error_mensaje: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query(
            r"
            UPDATE bdp_pagos
            SET resultado = $2, datos_respuesta = $3, error_mensaje = $4, updated_at = NOW()
            WHERE id = $1
",
        )
        .bind(id)
        .bind(resultado)
        .bind(datos_respuesta)
        .bind(error_mensaje)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Marca un pago ambiguo como exitoso tras reconciliarlo con BDP.
    pub async fn reconciliar_exito(
        pool: &PgPool,
        id: Uuid,
        bdp_payment_id: Option<&str>,
        datos_respuesta: Option<&serde_json::Value>,
    ) -> Result<(), AppError> {
        sqlx::query(
            r"
            UPDATE bdp_pagos
            SET resultado = 'exito',
                bdp_payment_id = COALESCE($2, bdp_payment_id),
                datos_respuesta = COALESCE($3, datos_respuesta),
                error_mensaje = NULL,
                updated_at = NOW()
            WHERE id = $1
",
        )
        .bind(id)
        .bind(bdp_payment_id)
        .bind(datos_respuesta)
        .execute(pool)
        .await?;

        Ok(())
    }

    /* [128A-1/F6] Pago parcial local (A8/M13) — escritura sobre el ledger
     * existente `bdp_pagos` sin renombrar (compatibilidad). Inserta una fila
     * local (`bdp_order_id`/`bdp_payment_id` NULL) con:
     *   - guards: venta no anulada ni facturada (local o BDP); importe dentro
     *     del saldo pendiente (total - pagos exitosos).
     *   - idempotencia: `UNIQUE(idempotency_key)` + `ON CONFLICT DO NOTHING`;
     *     si la clave ya existe devuelve la fila previa y `audit_id = None`.
     *   - auditoría obligatoria `pago_parcial_local` con `origen_operacion='local'`.
     *
     * El lock `FOR UPDATE` sobre la venta serializa pagos concurrentes de la
     * misma venta (evita sobrepago por carrera).
     *
     * Retorna (BdpPago, audit_id: Option<Uuid>).
     */
    #[allow(clippy::too_many_lines)]
    pub async fn insertar_local(
        pool: &PgPool,
        user_id: Uuid,
        venta_id: Uuid,
        amount: Decimal,
        tender_id: i32,
        idempotency_key: &str,
    ) -> Result<(BdpPago, Option<Uuid>), AppError> {
        let mut tx = pool.begin().await.map_err(AppError::from)?;

        /* Clave de idempotencia: si la petición no trae una, se genera una
         * única. El ledger exige `idempotency_key NOT NULL UNIQUE`; usar ""
         * como clave colapsaría pagos distintos de la misma venta en uno solo
         * (el segundo haría ON CONFLICT y se devolvería el primero). */
        let key = if idempotency_key.is_empty() {
            format!("local-{venta_id}-{}", Uuid::new_v4())
        } else {
            idempotency_key.to_string()
        };

        /* Lock de la venta para serializar pagos de la misma venta. */
        let venta: crate::models::Venta =
            sqlx::query_as("SELECT * FROM ventas WHERE id = $1 AND user_id = $2 FOR UPDATE")
                .bind(venta_id)
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(AppError::from)?
                .ok_or_else(|| AppError::NotFound("Venta no encontrada".into()))?;

        /* Guards M9: no pagar anuladas ni facturadas. */
        if venta.anulada {
            return Err(AppError::Conflict(
                "La venta está anulada y no admite pagos.".into(),
            ));
        }
        if venta.facturada_local
            || venta.bdp_invoiced
            || venta.bdp_order_status.as_deref() == Some("invoiced")
        {
            return Err(AppError::Conflict(
                "La venta está facturada y no admite más pagos.".into(),
            ));
        }

        /* Saldo pendiente = total - pagos exitosos del ledger. */
        let pagado: Decimal = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount), 0) FROM bdp_pagos \
             WHERE venta_id = $1 AND resultado = 'exito'",
        )
        .bind(venta_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(AppError::from)?;
        let total = venta.importe_base + venta.importe_iva;
        let pendiente = total - pagado;
        if amount > pendiente + Decimal::new(1, 3) {
            return Err(AppError::Validation(format!(
                "El importe {amount:.2} excede el saldo pendiente {pendiente:.2}"
            )));
        }

        let fila = sqlx::query(
            r"INSERT INTO bdp_pagos
                 (venta_id, amount, tender_id, idempotency_key, bdp_order_id, bdp_payment_id, datos_respuesta)
               VALUES ($1, $2, $3, $4, NULL, NULL, $5)
               ON CONFLICT (idempotency_key) DO NOTHING
               RETURNING id, venta_id, amount, tender_id, idempotency_key, bdp_order_id,
                         bdp_payment_id, resultado, datos_respuesta, error_mensaje, created_at, updated_at",
        )
        .bind(venta_id)
        .bind(amount)
        .bind(tender_id)
        .bind(&key)
        .bind(serde_json::json!({ "origen": "local", "pendiente_previo": pendiente }))
        .fetch_optional(&mut *tx)
        .await
        .map_err(AppError::from)?;

        let pago = if let Some(row) = fila {
            Self::row_to_bdp_pago(&row)
        } else {
            /* Idempotencia: la clave ya existe en el ledger (puede ser otra
             * venta); el handler decide según la venta de la fila previa. */
            let row = sqlx::query(
                r"SELECT id, venta_id, amount, tender_id, idempotency_key, bdp_order_id,
                          bdp_payment_id, resultado, datos_respuesta, error_mensaje,
                          created_at, updated_at
                   FROM bdp_pagos WHERE idempotency_key = $1",
            )
            .bind(&key)
            .fetch_one(&mut *tx)
            .await
            .map_err(AppError::from)?;
            tx.commit().await.map_err(AppError::from)?;
            return Ok((Self::row_to_bdp_pago(&row), None));
        };

        /* Auditoría local obligatoria. */
        let audit_id = Self::auditar_pago_local(
            &mut tx,
            venta.user_id,
            venta_id,
            amount,
            tender_id,
            pendiente,
            &key,
        )
        .await?;

        tx.commit().await.map_err(AppError::from)?;

        Ok((pago, Some(audit_id)))
    }

    /* Auditoría local obligatoria del pago parcial. Aislada en un helper para
     * que insertar_local no exceda el límite de líneas efectivas por función. */
    async fn auditar_pago_local(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        user_id: Uuid,
        venta_id: Uuid,
        amount: Decimal,
        tender_id: i32,
        pendiente: Decimal,
        key: &str,
    ) -> Result<Uuid, AppError> {
        let audit_payload = serde_json::json!({
            "venta_id": venta_id,
            "amount": amount,
            "tender_id": tender_id,
            "saldo_pendiente": pendiente,
        });
        let audit_id: Uuid = sqlx::query_scalar(
            r"INSERT INTO bdp_audit_log
                 (user_id, operacion, direccion, datos_enviados, resultado, origen_operacion,
                  target_entity_type, target_entity_id, authorization_reason, idempotency_key)
               VALUES ($1, 'pago_parcial_local', 'internal', $2, 'exito', 'local', 'venta', $3, $4, $5)
               ON CONFLICT (user_id, idempotency_key) WHERE idempotency_key IS NOT NULL DO NOTHING
               RETURNING id",
        )
        .bind(user_id)
        .bind(audit_payload)
        .bind(venta_id)
        .bind(format!(
            "Pago parcial local de {amount:.2} sobre la venta {venta_id} — operación interna, no requiere autorización BDP"
        ))
        .bind(key)
        .fetch_optional(&mut **tx)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::Internal("No se pudo auditar el pago local".into()))?;

        Ok(audit_id)
    }
}
