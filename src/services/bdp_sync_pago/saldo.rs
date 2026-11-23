// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [21AB-5a] Saldo de una comanda **desde BDP**, no desde la copia local.
 *
 * El cálculo anterior (`importe_base + importe_iva` menos el ledger) se
 * desviaba en cuanto BDP cobraba un total distinto del que la web creía: con
 * un descuento, el TPV cobraba 4,50 y la web seguía mostrando 0,45 pendientes
 * tras haberlo cobrado todo (regresión del 21/11). Guillermo lo pide explícito:
 * `Order.Total` menos los `Payments` que devuelve `GetOrder`.
 *
 * Los pagos locales sin identidad remota se suman aparte (misma regla que la
 * puerta de pago): un pago del ledger que no se puede demostrar presente en
 * BDP no puede restarse del pendiente sin arriesgar un sobrepago.
 *
 * Solo lectura: esta función no escribe ni en BDP ni en el ledger. */

use rust_decimal::prelude::Decimal;
use serde_json::Value;
use sqlx::PgPool;

use crate::models::{ConfiguracionRestaurante, Venta};
use crate::repositories::BdpPagoRepository;
use crate::services::bdp_weblink::{BdpWeblinkClient, BdpWeblinkError};
use crate::services::bdp_weblink_catalog::{BdpGetOrderRequest, BdpOrderIdentifier};

use super::super::bdp_sync::BdpSyncService;

/* De dónde salió el saldo. La web lo muestra tal cual: un saldo calculado sin
 * BDP no puede presentarse como si fuera el del TPV. */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrigenSaldo {
    Bdp,
    Local,
}

#[derive(Debug, Clone, Copy)]
pub struct SaldoVenta {
    pub total: Decimal,
    pub pagado: Decimal,
    pub pendiente: Decimal,
    pub origen: OrigenSaldo,
}

impl BdpSyncService {
    /* [21AB-5a] Refresca la comanda en BDP y devuelve el saldo que manda el
     * TPV. Si BDP no responde, cae al cálculo local **declarándolo**: el
     * operador ve que ese saldo no viene del TPV y puede decidir no cobrar.
     * La puerta de pago sigue exigiendo BDP para escribir, así que un saldo
     * local nunca autoriza un cobro por sí solo. */
    pub async fn saldo_de_venta(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
    ) -> Result<SaldoVenta, String> {
        /* Solo se abre sesión si hay comanda remota y BDP está habilitado: en
         * modo independiente no hay a quién preguntar. */
        let consultable = venta.bdp_order_id.is_some()
            && crate::services::ServicioModoOperacion::modo_efectivo_desde_config(config)
                == crate::services::ModoEfectivo::Bdp;
        let cliente = if consultable {
            match Self::conectar_bdp(config).await {
                Ok(cliente) => Some(cliente),
                Err(error) => {
                    tracing::warn!(
                        "[21AB-5a] Sin sesión BDP para el saldo de la venta {}: {error}",
                        venta.id
                    );
                    None
                }
            }
        } else {
            None
        };
        Self::saldo_con_cliente(pool, venta, cliente.as_ref()).await
    }

    /* Variante con sesión ya abierta: la usa quien acaba de autenticarse para
     * no pagar un login extra. */
    pub(crate) async fn saldo_con_cliente(
        pool: &PgPool,
        venta: &Venta,
        cliente: Option<&BdpWeblinkClient<'_>>,
    ) -> Result<SaldoVenta, String> {
        /* [21AB-5d] Una comanda facturada deja de existir para `GetOrder`
         * (`Order: null`, `Status: 3`): caer al cálculo local mostraba 7,80
         * pendientes en una venta cobrada y facturada. Facturar ya exigió
         * saldo cero en BDP y en el ledger, así que aquí el pendiente es cero
         * por construcción; el total es lo que se cobró. */
        if venta.bdp_invoiced || venta.facturada_local {
            return Self::saldo_cerrado(pool, venta).await;
        }
        if let (Some(cliente), Some(order_id)) = (cliente, venta.bdp_order_id) {
            match Self::saldo_remoto(cliente, pool, venta, order_id).await {
                Ok(saldo) => return Ok(saldo),
                Err(error) => {
                    tracing::warn!(
                        "[21AB-5a] Saldo remoto no disponible para la venta {}: {error}",
                        venta.id
                    );
                }
            }
        }
        Self::saldo_local(pool, venta).await
    }

    /* Unión de libros: `Order.Total` menos los `Payments` de BDP, más los pagos
     * locales que no se pueden identificar en la respuesta remota. */
    async fn saldo_remoto(
        client: &BdpWeblinkClient<'_>,
        pool: &PgPool,
        venta: &Venta,
        order_id: i64,
    ) -> Result<SaldoVenta, String> {
        let current = client
            .get_order(&BdpGetOrderRequest {
                order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            })
            .await
            .map_err(|e: BdpWeblinkError| format!("GetOrder falló: {e}"))?;
        let order = current
            .get("Order")
            .ok_or_else(|| "GetOrder no devolvió el objeto Order".to_string())?;
        let total = Self::parse_remote_money(
            order
                .get("Total")
                .ok_or_else(|| "GetOrder no devolvió Total".to_string())?,
            "Total",
        )?;
        let payments = order
            .get("Payments")
            .and_then(Value::as_array)
            .ok_or_else(|| "GetOrder no devolvió la colección Payments".to_string())?;
        let (remote_amounts, remote_paid) = Self::parse_remote_payments(payments)?;

        let locales = BdpPagoRepository::listar_por_venta(pool, venta.id)
            .await
            .map_err(|e| format!("No se pudo leer el ledger local: {e}"))?;
        let locales_sueltos = locales
            .iter()
            .filter(|pago| pago.resultado == "exito")
            .try_fold(Decimal::ZERO, |suma, pago| {
                Self::local_payment_contribution(
                    &remote_amounts,
                    pago.bdp_payment_id.as_deref(),
                    pago.amount,
                )
                .map(|aporte| suma + aporte)
            })?;

        let pagado = remote_paid + locales_sueltos;
        Ok(SaldoVenta {
            total,
            pagado,
            pendiente: (total - pagado).max(Decimal::ZERO),
            origen: OrigenSaldo::Bdp,
        })
    }

    /* Venta ya cerrada: lo cobrado es el total. Facturar exige saldo cero, así
     * que el pendiente no puede ser otro que cero. */
    async fn saldo_cerrado(pool: &PgPool, venta: &Venta) -> Result<SaldoVenta, String> {
        let cobrado = BdpPagoRepository::total_pagado(pool, venta.id)
            .await
            .map_err(|e| format!("No se pudo leer el ledger local: {e}"))?;
        /* Cobrada por partes: lo cobrado es el total real del TPV. Cobrada de
         * una vez (`metodo_pago`, sin filas en el ledger): el total es el de la
         * venta y está pagada entera, que es lo que exigió facturar. */
        let total = if cobrado > Decimal::ZERO {
            cobrado
        } else {
            venta.importe_base + venta.importe_iva
        };
        Ok(SaldoVenta {
            total,
            pagado: total,
            pendiente: Decimal::ZERO,
            origen: OrigenSaldo::Bdp,
        })
    }

    /* Sin BDP: lo que la web sabe. Es el cálculo antiguo, conservado solo como
     * respaldo declarado para que la pantalla no quede en blanco. */
    async fn saldo_local(pool: &PgPool, venta: &Venta) -> Result<SaldoVenta, String> {
        let total = venta.importe_base + venta.importe_iva;
        let pagado = BdpPagoRepository::total_pagado(pool, venta.id)
            .await
            .map_err(|e| format!("No se pudo leer el ledger local: {e}"))?;
        Ok(SaldoVenta {
            total,
            pagado,
            pendiente: (total - pagado).max(Decimal::ZERO),
            origen: OrigenSaldo::Local,
        })
    }
}
