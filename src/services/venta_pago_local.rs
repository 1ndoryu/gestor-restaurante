/*! [21AB-5a] Pago parcial local (A8/M13) y la puerta de su importe.
 *
 * Vive aparte de `venta.rs` porque el servicio rozaba el límite de god-object
 * que vigila Sentinel: este bloque —cobro en el ledger local y su tope de
 * saldo— se lee solo y no comparte estado con el alta, la edición ni la
 * anulación.
 *
 * El tope sale del saldo del TPV (`GetOrder`) cuando la venta está en BDP; en
 * modo independiente, de la copia local.
 */

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::BdpPago;
use crate::repositories::{BdpPagoRepository, ConfiguracionRepository, VentaRepository};

use super::{BdpSyncService, OrigenSaldo};

use super::venta::VentaService;

impl VentaService {
    /* [128A-1/F6] Pago parcial local (A8/M13).
     * Escribe sobre el ledger existente `bdp_pagos` (fila local sin
     * `bdp_order_id`). El repo aplica guards (anulada/facturada), saldo
     * pendiente e idempotencia dentro de una transacción con lock de venta.
     * Un reintento con la misma clave es éxito idempotente solo si la fila
     * previa pertenece a la misma venta con el mismo importe. */
    pub async fn pago_parcial_local(
        pool: &PgPool,
        user_id: Uuid,
        venta_id: Uuid,
        amount: rust_decimal::Decimal,
        tender_id: i32,
        idempotency_key: Option<&str>,
    ) -> Result<(BdpPago, Option<Uuid>), AppError> {
        let key = idempotency_key.unwrap_or_default();
        /* [21AB-5a] El tope del importe sale del saldo del TPV, no de base +
         * IVA. Si BDP no responde, el saldo cae a local y se sigue pudiendo
         * cobrar en modo independiente. */
        let pendiente_remoto = Self::pendiente_bdp_de_venta(pool, user_id, venta_id).await;
        let (pago, audit_id) = BdpPagoRepository::insertar_local(
            pool,
            user_id,
            venta_id,
            amount,
            tender_id,
            key,
            pendiente_remoto,
        )
        .await?;

        if audit_id.is_none() {
            /* Reintento idempotente: la clave ya se usó. */
            if pago.venta_id != venta_id {
                return Err(AppError::Conflict(
                    "idempotency_key ya usada para otra venta".into(),
                ));
            }
            if pago.amount != amount {
                return Err(AppError::Conflict(
                    "idempotency_key ya usada con otro importe".into(),
                ));
            }
        }

        Ok((pago, audit_id))
    }

    /* [21AB-5a] Pendiente del TPV para la puerta del pago local. `None` cuando
     * la venta no está en BDP o BDP no contesta: el repositorio calcula
     * entonces sobre la copia local, que es lo correcto en modo independiente. */
    async fn pendiente_bdp_de_venta(
        pool: &PgPool,
        user_id: Uuid,
        venta_id: Uuid,
    ) -> Option<rust_decimal::Decimal> {
        let venta = VentaRepository::find_by_id(pool, venta_id, user_id)
            .await
            .ok()
            .flatten()?;
        /* Sin comanda remota no hay a quién preguntar el saldo. */
        venta.bdp_order_id?;
        let config = ConfiguracionRepository::obtener_o_crear(pool, user_id)
            .await
            .ok()?;
        let saldo = BdpSyncService::saldo_de_venta(pool, &venta, &config)
            .await
            .ok()?;
        (saldo.origen == OrigenSaldo::Bdp).then_some(saldo.pendiente)
    }
}
