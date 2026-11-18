// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [18AB-2] Factura posterior al cobro.
 *
 * Guillermo (18/11/2026) pide separar pago y facturación: el `CreateOrder`
 * lleva el pago pero **no** pide factura (`Invoice: false`), se comprueba con
 * `GetOrder` que la comanda quedó guardada y cobrada, y solo entonces se llama
 * a `Orders/Invoice`. Dos motivos: una respuesta perdida ya no deja la
 * comanda facturada sin que nadie lo sepa, y la factura se puede reintentar
 * sola porque `invoice_order` reconcilia antes de escribir.
 */

use rust_decimal::prelude::Decimal;
use serde_json::Value;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::models::{ConfiguracionRestaurante, Venta};
use crate::repositories::VentaRepository;
use crate::services::bdp_sync::BdpSyncService;
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{BdpGetOrderRequest, BdpOrderIdentifier};

impl BdpSyncService {
    /* [18AB-2] Cierra el flujo de una comanda creada con el pago incluido:
     * confirma en BDP que el pago está, lo registra en el ledger local y emite
     * la factura. Si algo falla, la comanda sigue cobrada en BDP y la factura
     * queda pendiente (botón de facturar), con el motivo en el log. */
    pub(super) async fn facturar_orden_cobrada(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
    ) {
        let Some(tender_id) = Self::resolve_tender_id(venta, config) else {
            return;
        };
        let Ok(Some(fresca)) = VentaRepository::find_by_id(pool, venta.id, venta.user_id).await
        else {
            warn!(
                "[18AB-2] No se pudo releer la venta {} para facturarla",
                venta.id
            );
            return;
        };
        let Some(order_id) = fresca.bdp_order_id.filter(|id| *id > 0) else {
            return;
        };
        if fresca.bdp_invoiced {
            return;
        }

        let client = BdpWeblinkClient::new(config);
        let order = match client
            .get_order(&BdpGetOrderRequest {
                order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            })
            .await
        {
            Ok(respuesta) => respuesta.get("Order").cloned().unwrap_or(respuesta),
            /* Sin lectura previa no se factura ni se toca el ledger: la factura
             * espera a que BDP conteste. */
            Err(error) => {
                warn!(
                    "[18AB-2] Comanda {} creada; no se pudo confirmar el cobro en BDP: {error}",
                    venta.id
                );
                return;
            }
        };

        let payment_id = crate::services::bdp_sync_pago::payment_id_de_pago(venta.id, "create");
        let amount = Self::importe_del_pago(&order, &payment_id, tender_id);
        let Some(amount) = amount else {
            info!(
                "[18AB-2] La comanda {} no trae el pago {payment_id} en BDP; no hay factura que emitir",
                venta.id
            );
            return;
        };

        /* El pago viajó dentro del `CreateOrder`, así que el ledger local no lo
         * tenía: sin esta fila, `verificar_saldo_facturable` vería saldo
         * pendiente y bloquearía la factura de un cobro que sí existe. */
        if let Err(error) =
            Self::registrar_pago_incluido(pool, venta.id, amount, tender_id, order_id, &payment_id)
                .await
        {
            warn!(
                "[18AB-2] Comanda {} cobrada sin fila en el ledger: {error}",
                venta.id
            );
            return;
        }

        match Self::invoice_order(pool, &fresca, config, None).await {
            Ok(numero) => info!(
                "[18AB-2] Comanda {} cobrada y facturada aparte → InvoiceNumber={numero}",
                venta.id
            ),
            Err(error) => warn!("[18AB-2] Comanda {} cobrada sin factura: {error}", venta.id),
        }
    }

    /* [18AB-2] Importe del pago que viajó en la creación, leído de BDP. Un
     * identificador presente con otro importe o forma de pago no se da por
     * bueno: sin coincidencia completa no hay factura. */
    fn importe_del_pago(order: &Value, payment_id: &str, tender_id: i32) -> Option<Decimal> {
        let pagos = order.get("Payments").and_then(Value::as_array)?;
        let pago = pagos.iter().find(|pago| {
            Self::remote_payment_id(pago.get("PaymentId")).as_deref() == Some(payment_id)
        })?;
        if let Some(tender) = pago.get("TenderId").and_then(Value::as_i64) {
            if tender != i64::from(tender_id) {
                return None;
            }
        }
        Self::parse_remote_money(pago.get("Amount")?, "Payments.Amount")
            .ok()
            .filter(|amount| *amount > Decimal::ZERO)
    }

    /* [18AB-2] Pago que viajó dentro del `CreateOrder`, en `bdp_pagos`. La
     * clave es el propio `PaymentId` del pago: es único por venta, así que dos
     * envíos de la misma comanda no duplican la fila. */
    async fn registrar_pago_incluido(
        pool: &PgPool,
        venta_id: uuid::Uuid,
        amount: Decimal,
        tender_id: i32,
        order_id: i64,
        payment_id: &str,
    ) -> Result<(), String> {
        sqlx::query(
            r"INSERT INTO bdp_pagos
            (venta_id, amount, tender_id, idempotency_key, bdp_order_id, bdp_payment_id, resultado)
            VALUES ($1, $2, $3, $4, $5, $6, 'exito')
            ON CONFLICT (idempotency_key) DO NOTHING",
        )
        .bind(venta_id)
        .bind(amount)
        .bind(tender_id)
        .bind(payment_id)
        .bind(order_id)
        .bind(payment_id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
    }
}
