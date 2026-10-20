/* Formas en que el BDP devuelve una lista de objetos, que no son una sola.
 *
 * Comprobado contra el BDP local el 2026-10-20: `ExportProfiles/Documents`
 * contesta `{"DocumentsLists":{"TB":[ … ]}}` —agrupado por serie— y
 * `ExportProfiles/PurchaseNotes` contesta `{"ExportedData":{"Cabecera":[ … ]}}`.
 * Los dos contratos decían «array» y los dos leían cero elementos **sin error**,
 * que es la peor forma de fallar: el parser devolvía una lista vacía y nadie se
 * enteraba. Esto vive aquí, y no en cada contrato, para que la próxima lista del
 * BDP no vuelva a tropezar con lo mismo. */

use serde_json::Value;

/// Objetos de `lista`, con las dos formas que BDP usa de verdad: array directo y
/// objeto agrupado por clave (`{"TB":[ … ],"FA":[ … ]}`). Del objeto se toman las
/// claves en orden alfabético para que la lectura sea reproducible —el orden de
/// un objeto JSON no está garantizado—; la deduplicación va después, en quien la
/// necesite.
#[must_use]
pub fn objetos_de(lista: Option<&Value>) -> Vec<&Value> {
    match lista {
        Some(Value::Array(items)) => items.iter().collect(),
        Some(Value::Object(grupos)) => {
            let mut claves: Vec<&String> = grupos.keys().collect();
            claves.sort();
            claves
                .into_iter()
                .filter_map(|clave| grupos[clave].as_array())
                .flatten()
                .collect()
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn array_directo_se_devuelve_tal_cual() {
        let valor = json!([{"a": 1}, {"a": 2}]);
        assert_eq!(objetos_de(Some(&valor)).len(), 2);
    }

    #[test]
    fn objeto_agrupado_se_aplana_en_orden_alfabetico() {
        let valor = json!({"TB": [{"serie": "TB"}], "FA": [{"serie": "FA"}]});
        let series: Vec<&str> = objetos_de(Some(&valor))
            .iter()
            .map(|o| o["serie"].as_str().unwrap())
            .collect();
        assert_eq!(series, vec!["FA", "TB"]);
    }

    #[test]
    fn lo_que_no_es_lista_da_vacio() {
        assert!(objetos_de(None).is_empty());
        assert!(objetos_de(Some(&json!({"ErrorMessage": ""}))).is_empty());
        assert!(objetos_de(Some(&json!("texto"))).is_empty());
    }
}
