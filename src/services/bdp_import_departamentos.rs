/* [159A-2/F1] Importación de departamentos del BDP a clasificaciones locales.
 * Lee `/API/Departments/ExportFromProfile` (solo lectura, perfil del terminal) y
 * vuelca los departamentos como clasificaciones `tipo=departamento`.
 *
 * Reglas locales (paralelas a M6/M7 del sync de artículos):
 * - El import NUNCA pisa una fila local: si ya existe el mismo `code`, se cuenta
 *   como vinculada y se deja intacta (nombre local manda).
 * - Si existe el mismo `nombre` con distinto `code`, es conflicto: se omite y se
 *   reporta (los UNIQUE de (user_id,tipo,code) y (user_id,tipo,nombre) lo impedirían).
 * - Códigos fuera del CHECK 1..999 se omiten y se reportan (no se remapean a mano).
 * - Familias: el BDP no expone export de familias (solo CreateFamily/CreateSubfamily
 *   y códigos Family/Subfamily en ExportArticles sin nombres); por eso no se importan.
 */

use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::repositories::BdpCatalogoClasificacionRepository;

/// Resultado de importar departamentos BDP → clasificaciones locales.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct BdpImportDepartamentosResult {
    pub nuevos: u32,
    pub vinculados: u32,
    pub conflictos_nombre: u32,
    pub omitidos_fuera_rango: u32,
    pub errores: u32,
    pub total_bdp: usize,
}

/// Departamento aplanado del árbol del BDP (incluye subdepartamentos).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BdpDepartamentoImportado {
    pub code: i32,
    pub nombre: String,
}

/// Aplana la respuesta cruda de ExportFromProfile:
/// `{"Departamentos":[{Codigo, Descripcion, SubDepartamentos:[...]}]}`.
/// `Codigo` puede venir como número entero o float. Función pura (testeable sin BDP).
pub fn aplanar_departamentos(valor: &serde_json::Value) -> Vec<BdpDepartamentoImportado> {
    let mut salida = Vec::new();
    if let Some(lista) = valor.get("Departamentos").and_then(|v| v.as_array()) {
        for item in lista {
            aplanar_nodo(item, &mut salida);
        }
    }
    salida
}

fn aplanar_nodo(nodo: &serde_json::Value, salida: &mut Vec<BdpDepartamentoImportado>) {
    if let Some(code) = codigo_entero(nodo.get("Codigo")) {
        let nombre = nodo
            .get("Descripcion")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if !nombre.is_empty() {
            salida.push(BdpDepartamentoImportado { code, nombre });
        }
    }
    if let Some(subs) = nodo.get("SubDepartamentos").and_then(|v| v.as_array()) {
        for sub in subs {
            aplanar_nodo(sub, salida);
        }
    }
}

fn codigo_entero(valor: Option<&serde_json::Value>) -> Option<i32> {
    match valor {
        Some(serde_json::Value::Number(n)) => {
            if let Some(i) = n.as_i64() {
                i32::try_from(i).ok()
            } else {
                n.as_f64().map(|f| f as i32)
            }
        }
        _ => None,
    }
}

pub struct BdpImportDepartamentosService;

impl BdpImportDepartamentosService {
    /// Vuelca los departamentos ya aplanados en clasificaciones locales.
    pub async fn importar(
        pool: &PgPool,
        user_id: Uuid,
        items: &[BdpDepartamentoImportado],
        tipo: &str,
    ) -> Result<BdpImportDepartamentosResult, String> {
        let mut resultado = BdpImportDepartamentosResult {
            nuevos: 0,
            vinculados: 0,
            conflictos_nombre: 0,
            omitidos_fuera_rango: 0,
            errores: 0,
            total_bdp: items.len(),
        };
        for item in items {
            /* CHECK de la tabla: code BETWEEN 1 AND 999. */
            if !(1..=999).contains(&item.code) {
                resultado.omitidos_fuera_rango += 1;
                continue;
            }
            if BdpCatalogoClasificacionRepository::buscar_por_code(pool, user_id, tipo, item.code)
                .await
                .map_err(|e| format!("No se pudo buscar clasificación por código: {e}"))?
                .is_some()
            {
                /* La fila local manda: no se pisa el nombre. */
                resultado.vinculados += 1;
                continue;
            }
            if BdpCatalogoClasificacionRepository::buscar_por_nombre(
                pool,
                user_id,
                tipo,
                &item.nombre,
            )
            .await
            .map_err(|e| format!("No se pudo buscar clasificación por nombre: {e}"))?
            .is_some()
            {
                resultado.conflictos_nombre += 1;
                continue;
            }
            if let Err(e) = BdpCatalogoClasificacionRepository::crear_con_code(
                pool,
                user_id,
                tipo,
                item.code,
                &item.nombre,
            )
            .await
            {
                tracing::warn!("[159A-2] No se pudo importar depto {code}: {e}", code = item.code);
                resultado.errores += 1;
                continue;
            }
            resultado.nuevos += 1;
        }
        Ok(resultado)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aplana_arbol_con_subdepartamentos_y_floats() {
        let valor = serde_json::json!({
            "Departamentos": [
                {"Codigo": 1.0, "Descripcion": "CAFES", "SubDepartamentos": []},
                {"Codigo": 51, "Descripcion": "ALCOHOLES (GRUPO)", "SubDepartamentos": [
                    {"Codigo": 8.0, "Descripcion": "GINEBRAS", "SubDepartamentos": []},
                    {"Codigo": null, "Descripcion": "SIN CODIGO", "SubDepartamentos": []},
                    {"Codigo": 10.0, "Descripcion": "  ", "SubDepartamentos": []}
                ]}
            ]
        });
        let planos = aplanar_departamentos(&valor);
        assert_eq!(
            planos,
            vec![
                BdpDepartamentoImportado { code: 1, nombre: "CAFES".into() },
                BdpDepartamentoImportado { code: 51, nombre: "ALCOHOLES (GRUPO)".into() },
                BdpDepartamentoImportado { code: 8, nombre: "GINEBRAS".into() },
            ]
        );
    }
}
