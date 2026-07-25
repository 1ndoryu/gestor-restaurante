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
}
