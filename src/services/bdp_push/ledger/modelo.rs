// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [D26g] Lo que el libro de stock sabe de una escritura pendiente y las tres
 * reglas con las que decide si un envío sin respuesta se puede repetir.
 *
 * BDP no devuelve identificador de operación, así que `id_local` es nuestro
 * (uno por intento) y el dedo se echa entre `stock_esperado` (S+V) y lo que
 * se lea después con `GetStock`. */

use std::str::FromStr;

use rust_decimal::Decimal;
use serde_json::Value;

/// Decisión tras una respuesta perdida (D26g).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconciliacionStock {
    /// Se ve S+V: el movimiento sí se aplicó. No reintentar.
    Aplicado,
    /// Se ve S: no se aplicó. Un único reintento controlado.
    Pendiente,
    /// Cualquier otro valor: no se sabe qué pasó. Parar y revisar a mano.
    Discordante,
}

/// Regla de D26g, comparando la lectura posterior con S y S+V.
/// `stock_previo` desconocido (no se pudo leer antes de escribir) siempre es
/// discordante: sin S no hay nada que comparar y reintentar a ciegas es lo
/// que la prueba prohíbe.
pub fn clasificar(
    stock_previo: Option<Decimal>,
    variacion: Decimal,
    stock_posterior: Decimal,
) -> ReconciliacionStock {
    let Some(previo) = stock_previo else {
        return ReconciliacionStock::Discordante;
    };
    if stock_posterior == previo + variacion {
        ReconciliacionStock::Aplicado
    } else if stock_posterior == previo {
        ReconciliacionStock::Pendiente
    } else {
        ReconciliacionStock::Discordante
    }
}

/// Variación efectiva de un artículo del lote: la que hay que aplicar al stock
/// previo para saber qué se espera ver. En regularización y traspaso el payload
/// ya dice `Units` (delta en origen); en inventario absoluto la variación sale
/// de la lectura previa, porque `Units` es el stock final, no un delta.
pub fn variacion_de_linea(
    operacion: &str,
    unidades_payload: Decimal,
    stock_previo: Option<Decimal>,
) -> Option<Decimal> {
    match operacion {
        super::super::constantes::OPERACION_INVENTARIO => {
            stock_previo.map(|previo| unidades_payload - previo)
        }
        _ => Some(unidades_payload),
    }
}

/// Líneas `(artículo BDP, variación)` del payload ya construido.
/// Un payload que no encaja con la operación devuelve error en vez de una
/// lista vacía: sin líneas no hay libro y la prueba quedaría falsa.
pub fn lineas_del_payload(operacion: &str, payload: &Value) -> Result<Vec<(i64, Decimal)>, String> {
    use super::super::constantes::{
        OPERACION_INVENTARIO, OPERACION_REGULARIZAR, OPERACION_TRASPASAR,
    };
    let entero = |clave: &str| -> Result<i64, String> {
        payload
            .get(clave)
            .and_then(Value::as_i64)
            .ok_or_else(|| format!("payload de stock sin {clave}"))
    };
    let unidades = |objeto: &Value| -> Result<Decimal, String> {
        objeto
            .get("Units")
            .and_then(decimal)
            .ok_or_else(|| "payload de stock sin Units decimal".to_string())
    };
    match operacion {
        OPERACION_REGULARIZAR | OPERACION_TRASPASAR => {
            Ok(vec![(entero("Article")?, unidades(payload)?)])
        }
        OPERACION_INVENTARIO => {
            let lista = payload
                .get("ArticlesList")
                .and_then(Value::as_array)
                .ok_or_else(|| "payload de inventario sin ArticlesList".to_string())?;
            lista
                .iter()
                .map(|linea| Ok((entero_de(linea, "Article")?, unidades(linea)?)))
                .collect()
        }
        otra => Err(format!("operación de stock sin libro: {otra}")),
    }
}

/// Decimal de un valor JSON: BDP lo manda como texto (`"2.500"`) o como número.
pub(super) fn decimal(valor: &Value) -> Option<Decimal> {
    match valor {
        Value::String(texto) => Decimal::from_str(texto.trim()).ok(),
        Value::Number(numero) => Decimal::from_str(&numero.to_string()).ok(),
        _ => None,
    }
}

fn entero_de(objeto: &Value, clave: &str) -> Result<i64, String> {
    objeto
        .get(clave)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("línea de stock sin {clave}"))
}

/// Stock del artículo en el almacén según la respuesta de `GetStock`.
/// Un `ErrorMessage` no vacío se devuelve como error: un 0 leído de un error
/// no es un stock de 0.
pub fn stock_desde_respuesta(valor: &Value) -> Result<Decimal, String> {
    if let Some(error) = valor.get("ErrorMessage").and_then(Value::as_str) {
        if !error.is_empty() {
            return Err(format!("GetStock: {error}"));
        }
    }
    valor
        .get("Stock")
        .and_then(decimal)
        .ok_or_else(|| "GetStock sin campo Stock decimal".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(texto: &str) -> Decimal {
        Decimal::from_str(texto).expect("decimal de prueba")
    }

    #[test]
    fn ver_s_mas_v_es_aplicado() {
        assert_eq!(
            clasificar(Some(d("10")), d("2.5"), d("12.5")),
            ReconciliacionStock::Aplicado
        );
    }

    #[test]
    fn ver_s_es_pendiente() {
        assert_eq!(
            clasificar(Some(d("10")), d("2.5"), d("10")),
            ReconciliacionStock::Pendiente
        );
    }

    #[test]
    fn otro_valor_es_discordante() {
        assert_eq!(
            clasificar(Some(d("10")), d("2.5"), d("7")),
            ReconciliacionStock::Discordante
        );
    }

    #[test]
    fn sin_stock_previo_no_se_reintenta_a_ciegas() {
        assert_eq!(
            clasificar(None, d("1"), d("0")),
            ReconciliacionStock::Discordante
        );
    }

    #[test]
    fn el_inventario_calcula_su_variacion_contra_el_previo() {
        assert_eq!(
            variacion_de_linea("inventario", d("7"), Some(d("5"))),
            Some(d("2"))
        );
        assert_eq!(
            variacion_de_linea("regularizar", d("2"), Some(d("5"))),
            Some(d("2"))
        );
        assert_eq!(variacion_de_linea("inventario", d("7"), None), None);
    }

    #[test]
    fn lee_las_lineas_de_los_tres_payloads() {
        let reg = serde_json::json!({ "Article": 1001, "Units": "2.500" });
        assert_eq!(
            lineas_del_payload("regularizar", &reg),
            Ok(vec![(1001, d("2.500"))])
        );
        let inv = serde_json::json!({
            "ArticlesList": [{ "Article": 1001, "Units": "5" }, { "Article": 1002, "Units": "7" }]
        });
        assert_eq!(
            lineas_del_payload("inventario", &inv),
            Ok(vec![(1001, d("5")), (1002, d("7"))])
        );
        assert!(lineas_del_payload("traspasar", &serde_json::json!({})).is_err());
    }

    #[test]
    fn un_error_de_getstock_no_es_stock_cero() {
        assert!(stock_desde_respuesta(&serde_json::json!({
            "Stock": 0.0,
            "ErrorMessage": "[200005]-EL ALMACÉN CON CÓDIGO 99 NO EXISTE"
        }))
        .is_err());
        assert_eq!(
            stock_desde_respuesta(&serde_json::json!({ "Stock": 12.5, "ErrorMessage": "" })),
            Ok(d("12.5"))
        );
    }
}
