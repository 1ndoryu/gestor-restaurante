// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
/* [198A-1/D9] Ledger local de puntos. Registra cada operación (sumar/restar)
 * para poder consultar el saldo local sin BDP; el push AddPoints se encola en
 * el handler. */

use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::BdpPuntoCliente;

pub struct BdpPuntoClienteRepository;

impl BdpPuntoClienteRepository {
    pub async fn registrar(
        pool: &PgPool,
        user_id: Uuid,
        cliente_id: Uuid,
        bdp_customer_code: i32,
        points_added: Decimal,
        reason: &str,
    ) -> Result<BdpPuntoCliente, sqlx::Error> {
        sqlx::query_as::<_, BdpPuntoCliente>(
            "INSERT INTO bdp_puntos_cliente \
             (id, user_id, cliente_id, bdp_customer_code, points_added, reason) \
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(cliente_id)
        .bind(bdp_customer_code)
        .bind(points_added)
        .bind(reason)
        .fetch_one(pool)
        .await
    }

    pub async fn saldo(
        pool: &PgPool,
        user_id: Uuid,
        cliente_id: Uuid,
    ) -> Result<Decimal, sqlx::Error> {
        let saldo: Option<Decimal> = sqlx::query_scalar(
            "SELECT COALESCE(SUM(points_added), 0) FROM bdp_puntos_cliente \
             WHERE user_id = $1 AND cliente_id = $2",
        )
        .bind(user_id)
        .bind(cliente_id)
        .fetch_one(pool)
        .await?;
        Ok(saldo.unwrap_or(Decimal::ZERO))
    }

    pub async fn listar(
        pool: &PgPool,
        user_id: Uuid,
        cliente_id: Uuid,
    ) -> Result<Vec<BdpPuntoCliente>, sqlx::Error> {
        sqlx::query_as::<_, BdpPuntoCliente>(
            "SELECT * FROM bdp_puntos_cliente WHERE user_id = $1 AND cliente_id = $2 \
             ORDER BY created_at DESC",
        )
        .bind(user_id)
        .bind(cliente_id)
        .fetch_all(pool)
        .await
    }
}
