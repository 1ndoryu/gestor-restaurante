//! [18AB-3] Puertas de la anulación contra BDP: ninguna comanda con pagos se
//! anula, ni desde el servicio (antes del push local) ni desde el worker (antes
//! del `CancelOrder`). Guillermo, 18/11/2026: «bloquear desde el servidor la
//! cancelación de cualquier pedido que ya tenga pagos; consultar siempre estado
//! y pagos antes de cancelar».

use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::ConfiguracionRestaurante;
use crate::repositories::VentaRepository;
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{BdpGetOrderRequest, BdpOrderIdentifier};

use super::venta::VentaService;
use super::ModoEfectivo;

impl VentaService {
    /// [18AB-3] Lee estado y pagos de la comanda antes de anular.
    /// Sin comanda en BDP, o en modo standalone, no hay nada que consultar.
    /// Si BDP no responde se bloquea: anular a ciegas puede dejar un cobro vivo.
    pub(crate) async fn bloquear_anulacion_con_pagos_bdp(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        config: &ConfiguracionRestaurante,
    ) -> Result<(), AppError> {
        let venta = VentaRepository::find_by_id(pool, id, user_id)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("Venta no encontrada".into()))?;
        let Some(order_id) = venta.bdp_order_id else {
            return Ok(());
        };
        if super::ServicioModoOperacion::modo_efectivo_desde_config(config)
            == ModoEfectivo::Standalone
        {
            return Ok(());
        }

        let client = BdpWeblinkClient::new(config);
        let respuesta = client
            .get_order(&BdpGetOrderRequest {
                order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            })
            .await
            .map_err(|error| {
                AppError::Conflict(format!(
                    "No se puede anular: BDP no confirmó el estado de la comanda {order_id} \
                     ({error}). Inténtalo cuando el TPV responda."
                ))
            })?;

        let Some(order) = respuesta.get("Order").filter(|order| !order.is_null()) else {
            /* `Status: 3` con `Order: null` = comanda facturada: ya no es
             * legible. Cualquier otro caso es una respuesta que no se entiende. */
            if respuesta.get("Status").and_then(Value::as_i64) == Some(3) {
                return Err(AppError::Conflict(format!(
                    "La comanda {order_id} ya está facturada en BDP y no se puede anular."
                )));
            }
            return Err(AppError::Conflict(format!(
                "No se puede anular: BDP no devolvió la comanda {order_id}."
            )));
        };

        /* Sin pagos, una comanda abierta (pending/accepted) sí se anula: para
         * eso existe `CancelOrder`. El estado solo bloquea si BDP ya la facturó. */
        if respuesta
            .get("Status")
            .or_else(|| order.get("Status"))
            .and_then(Value::as_i64)
            == Some(3)
        {
            return Err(AppError::Conflict(format!(
                "La comanda {order_id} ya está facturada en BDP y no se puede anular."
            )));
        }
        let pagado = Self::pagado_remoto(order);
        if pagado > Decimal::ZERO {
            return Err(AppError::Conflict(format!(
                "La comanda {order_id} tiene {pagado} € cobrados en BDP y no se puede anular. \
                 Devuelve o retira los pagos en el TPV antes de anularla."
            )));
        }
        Ok(())
    }

    /// Suma de `Payments[].Amount` del pedido remoto. Un importe ilegible cuenta
    /// como cobro: ante la duda no se anula.
    fn pagado_remoto(order: &Value) -> Decimal {
        let Some(payments) = order.get("Payments").and_then(Value::as_array) else {
            return Decimal::ZERO;
        };
        payments
            .iter()
            .map(|pago| cantidad_remota(pago.get("Amount")).unwrap_or(Decimal::MAX))
            .sum()
    }
}

/// Un `Amount` de BDP puede llegar como número o como cadena.
fn cantidad_remota(valor: Option<&Value>) -> Option<Decimal> {
    let valor = valor?;
    if let Some(n) = valor.as_f64() {
        return Decimal::from_str_exact(&format!("{n:.2}")).ok();
    }
    valor.as_str()?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn pagado_remoto_suma_todos_los_pagos() {
        let order = json!({
            "Payments": [
                {"PaymentId": "P1", "Amount": 2.0},
                {"PaymentId": "P2", "Amount": "2.20"},
            ]
        });
        assert_eq!(VentaService::pagado_remoto(&order), Decimal::new(420, 2));
    }

    #[test]
    fn pagado_remoto_sin_payments_es_cero() {
        assert_eq!(
            VentaService::pagado_remoto(&json!({ "Total": 3.15 })),
            Decimal::ZERO
        );
    }

    #[test]
    fn importe_ilegible_cuenta_como_cobro() {
        let order = json!({ "Payments": [{ "PaymentId": "P1", "Amount": "x" }] });
        assert_eq!(VentaService::pagado_remoto(&order), Decimal::MAX);
    }

    #[test]
    fn un_pago_en_el_ledger_no_cuenta_como_pago_remoto() {
        /* El ledger local es otra puerta; esta función solo mira BDP. */
        assert_eq!(
            VentaService::pagado_remoto(&json!({ "Payments": [] })),
            Decimal::ZERO
        );
    }
}
