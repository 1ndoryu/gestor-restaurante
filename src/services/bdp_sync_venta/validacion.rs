/* [267A-9/F4.1] Validación de líneas y parseo de importes y pagos remotos.
 * Funciones puras: sin red ni BD. */

use std::collections::HashMap;

use rust_decimal::prelude::Decimal;
use serde_json::Value;

use super::OrderSendFailure;
use crate::models::VentaLinea;
use crate::services::bdp_sync::BdpSyncService;

impl BdpSyncService {
    /* [028A-BDP-WRITE] Valida las líneas justo antes de construir el payload.
     * Precio cero sigue permitido para cortesías; precios negativos y cantidades
     * no positivas no representan una línea válida para una escritura BDP. */
    pub(crate) fn validate_order_lines(lineas: &[VentaLinea]) -> Result<(), OrderSendFailure> {
        for linea in lineas {
            if linea.cantidad <= Decimal::ZERO {
                return Err(OrderSendFailure::Rejected(format!(
                    "Línea BDP inválida '{}': la cantidad debe ser mayor que cero",
                    linea.descripcion
                )));
            }
            if linea.precio_unitario < Decimal::ZERO {
                return Err(OrderSendFailure::Rejected(format!(
                    "Línea BDP inválida '{}': el precio no puede ser negativo",
                    linea.descripcion
                )));
            }
            if linea.descuento < Decimal::ZERO {
                return Err(OrderSendFailure::Rejected(format!(
                    "Línea BDP inválida '{}': el descuento no puede ser negativo",
                    linea.descripcion
                )));
            }
            let bruto = linea.precio_unitario * linea.cantidad;
            if linea.descuento > bruto {
                return Err(OrderSendFailure::Rejected(format!(
                    "Línea BDP inválida '{}': el descuento supera el importe bruto",
                    linea.descripcion
                )));
            }
        }
        Ok(())
    }

    /* [028A-BDP-WRITE] Parseo estricto de importes remotos. Nunca convertir
     * null/formatos inválidos a cero: en dinero, cero es un dato válido y no
     * puede usarse como fallback de una respuesta corrupta. */
    pub(crate) fn parse_remote_money(value: &Value, field: &str) -> Result<Decimal, String> {
        let raw = match value {
            Value::Number(number) => number.to_string(),
            Value::String(text) => text.trim().to_string(),
            _ => return Err(format!("BDP devolvió {field} con formato inválido")),
        };
        let parsed = raw
            .parse::<Decimal>()
            .map_err(|_| format!("BDP devolvió {field} inválido: '{raw}'"))?;
        if parsed < Decimal::ZERO {
            return Err(format!("BDP devolvió {field} negativo"));
        }
        Ok(parsed)
    }

    /* PaymentId no tiene un tipo estable entre respuestas BDP. Un identificador
     * vacío equivale a ausencia de identidad y no debe usarse para deduplicar. */
    pub(crate) fn remote_payment_id(value: Option<&Value>) -> Option<String> {
        let value = value?;
        let id = match value {
            Value::String(text) => text.trim().to_string(),
            Value::Number(number) => number.to_string(),
            _ => return None,
        };
        (!id.is_empty()).then_some(id)
    }

    /* [028A-BDP-WRITE] Decide la contribución de una fila local a la unión
     * financiera. Cero significa que el mismo pago ya está en BDP; el importe
     * completo significa identidad ausente/desconocida; una discrepancia de
     * importe bloquea. */
    pub(crate) fn local_payment_contribution(
        remote_payment_amounts: &HashMap<String, Decimal>,
        payment_id: Option<&str>,
        amount: Decimal,
    ) -> Result<Decimal, String> {
        let Some(payment_id) = payment_id else {
            return Ok(amount);
        };
        match remote_payment_amounts.get(payment_id) {
            None => Ok(amount),
            Some(remote_amount) if remote_amount == &amount => Ok(Decimal::ZERO),
            Some(_) => Err(format!(
                "Pago bloqueado: PaymentId {payment_id} tiene importe local y remoto distinto"
            )),
        }
    }

    /* [028A-BDP-WRITE] Normaliza y agrega Payments en una función pura para
     * que la regla financiera tenga regresiones unitarias independientes de
     * PostgreSQL/HTTP. Un ID repetido con el mismo importe es una repetición
     * de respuesta; el mismo ID con importe distinto es inconsistente y bloquea. */
    pub(crate) fn parse_remote_payments(
        payments: &[Value],
    ) -> Result<(HashMap<String, Decimal>, Decimal), String> {
        let mut amounts = HashMap::<String, Decimal>::new();
        let mut total = Decimal::ZERO;
        for payment in payments {
            let paid = Self::parse_remote_money(
                payment.get("Amount").ok_or_else(|| {
                    "Pago bloqueado: un pago remoto no contiene Amount".to_string()
                })?,
                "Payments.Amount",
            )?;
            let Some(payment_id) = Self::remote_payment_id(payment.get("PaymentId")) else {
                total += paid;
                continue;
            };
            if let Some(previous) = amounts.get(&payment_id) {
                if previous != &paid {
                    return Err(format!(
                        "Pago bloqueado: PaymentId remoto {payment_id} tiene importes contradictorios"
                    ));
                }
                continue;
            }
            amounts.insert(payment_id, paid);
            total += paid;
        }
        Ok((amounts, total))
    }
}
