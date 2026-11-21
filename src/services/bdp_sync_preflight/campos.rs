use serde_json::Value;

pub(super) fn value_array<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Vec<Value>> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_array))
}

/* [169A-2] IDs de tenders del POS (respuesta de Tenders/GetPOSList).
 * Sin lista (None) no hay ningún id válido: el P1 con pago exige tenders. */
pub(super) fn tender_ids_presentes(tenders: Option<&Value>) -> Vec<i64> {
    tenders
        .and_then(|value| value_array(value, &["TenderList", "Tenders"]))
        .map(|items| {
            items
                .iter()
                .filter_map(|item| number_i64(item, &["Id"]))
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn summarize_value(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(items) = value.get(*key).and_then(Value::as_array) {
            return Some(format!("{} elementos", items.len()));
        }
        if let Some(text) = value.get(*key).and_then(Value::as_str) {
            return Some(text.to_string());
        }
        if let Some(number) = value.get(*key).and_then(Value::as_i64) {
            return Some(number.to_string());
        }
    }
    None
}

pub(super) fn value_count(value: &Value, keys: &[&str]) -> Option<usize> {
    for key in keys {
        if let Some(items) = value.get(*key).and_then(Value::as_array) {
            return Some(items.len());
        }
        if value.get(*key).and_then(Value::as_object).is_some() {
            return Some(1);
        }
    }
    None
}

pub(super) fn number_i64(value: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|field| {
            field.as_i64().or_else(|| {
                #[allow(clippy::cast_possible_truncation)]
                field.as_f64().map(|number| number as i64)
            })
        })
    })
}

pub(super) fn number_f64(value: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|field| {
            field.as_f64().or_else(|| {
                #[allow(clippy::cast_precision_loss)]
                field.as_i64().map(|number| number as f64)
            })
        })
    })
}

/* [21AB-4] Código de cliente de una fila de `Customers/Export`. BDP lo
 * devuelve en `Customer` o en `Code`, como número o como texto; el cero no
 * cuenta como cliente (mismo criterio que el import de clientes). */
pub(super) fn customer_code(value: &Value) -> Option<i32> {
    value
        .get("Customer")
        .or_else(|| value.get("Code"))
        .and_then(|code| {
            code.as_i64()
                .or_else(|| code.as_str()?.trim().parse::<i64>().ok())
        })
        .and_then(|code| i32::try_from(code).ok())
        .filter(|code| *code > 0)
}

/* [21AB-4b] Entero de un valor suelto (no de un campo): numero o texto
 * numerico. La configuracion BDP guarda los mapas de las dos formas segun se
 * rellenen por la UI (numero) o por el bootstrap (texto), y los consumidores
 * reales aceptan ambas; los checks tienen que medir lo mismo que se ejecuta. */
pub(super) fn codigo_entero(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str()?.trim().parse::<i64>().ok())
}

pub(super) fn text_field(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(ToOwned::to_owned)
    })
}
