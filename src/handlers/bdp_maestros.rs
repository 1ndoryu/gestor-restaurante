/* [D24-p] Maestros de identificación del BDP: terminal POS, empleado y perfil
 * de artículos.
 *
 *   GET /api/bdp/maestros — listas reales de `POSes/Get`, `Employees/Get` y
 *                           `ProfilesLists/GetCreateArticleList`
 *
 * Por qué existe: la tarjeta de conexión pide esos tres números a mano y a la
 * vez advierte «el cliente no debe inventar códigos». BDP sí los publica, así
 * que la web puede comprobar el número configurado en vez de confiar en él. Es
 * solo lectura: no escribe ni arma nada. */

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use serde_json::Value;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::services::bdp_weblink_catalog::BdpGetEmployeesRequest;
use crate::services::{BdpWeblinkClient, ConfiguracionService};
use crate::AppState;

use super::bdp_guard::exigir_modo_bdp;

pub fn routes() -> Router<AppState> {
    Router::new().route("/bdp/maestros", get(listar_maestros))
}

/// Un registro de BDP reducido a lo que la pantalla necesita.
#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct MaestroBdp {
    pub id: i64,
    pub nombre: String,
    /// Dato extra que ayuda a distinguir dos registros con el mismo nombre.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detalle: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct MaestrosBdp {
    pub terminales: Vec<MaestroBdp>,
    pub empleados: Vec<MaestroBdp>,
    pub perfiles: Vec<MaestroBdp>,
    /// Error por categoría. Una lista vacía por un fallo no debe confundirse
    /// con «BDP no tiene ninguno».
    pub errores: Vec<String>,
}

/// Convierte la respuesta de BDP en registros, o explica por qué no se pudo.
///
/// Los nombres de las colecciones cambian entre versiones del manual, así que
/// se aceptan varias claves. Un `ErrorMessage` no vacío es un fallo aunque el
/// HTTP sea 200.
fn maestros_de(
    valor: &Value,
    colecciones: &[&str],
    id: &[&str],
    nombre: &[&str],
    detalle: Option<&str>,
) -> Result<Vec<MaestroBdp>, String> {
    if let Some(mensaje) = valor
        .get("ErrorMessage")
        .and_then(Value::as_str)
        .filter(|m| !m.trim().is_empty())
    {
        return Err(format!("BDP respondió: {mensaje}"));
    }
    let items = colecciones
        .iter()
        .find_map(|clave| valor.get(*clave).and_then(Value::as_array))
        .ok_or_else(|| "La respuesta del BDP no trae la lista esperada".to_string())?;
    Ok(items
        .iter()
        .filter_map(|item| {
            let id = id.iter().find_map(|clave| numero_de(item.get(*clave)))?;
            let nombre = nombre
                .iter()
                .find_map(|clave| item.get(*clave).and_then(Value::as_str))
                .unwrap_or("")
                .trim()
                .to_string();
            let detalle = detalle
                .and_then(|clave| item.get(clave))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|texto| !texto.is_empty())
                .map(str::to_string);
            Some(MaestroBdp {
                id,
                nombre,
                detalle,
            })
        })
        .collect())
}

/// Los identificadores llegan como entero o como decimal (`12.0`), igual que
/// los números de documento: sin normalizar, un `1.0` no casa con `1`.
fn numero_de(valor: Option<&Value>) -> Option<i64> {
    let texto = match valor? {
        Value::Number(numero) => numero.to_string(),
        Value::String(texto) => texto.clone(),
        _ => return None,
    };
    /* El decimal se descarta por texto, no con `as`: una conversión de f64 a
     * i64 puede truncar, y clippy lo rechaza con razón. */
    texto
        .trim()
        .split('.')
        .next()
        .and_then(|entero| entero.parse::<i64>().ok())
}

/// Terminales, empleados y perfiles dados de alta en BDP (solo lectura).
#[utoipa::path(
    get,
    path = "/api/bdp/maestros",
    tag = "BDP Backup",
    responses(
        (status = 200, description = "Maestros del BDP", body = MaestrosBdp),
        (status = 400, description = "BDP no configurado"),
        (status = 500, description = "Error interno"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_maestros(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<MaestrosBdp>, AppError> {
    exigir_modo_bdp(&state, auth.user_id).await?;
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation("BDP no está configurado.".into()));
    }

    let client = BdpWeblinkClient::new(&config);
    let mut errores = Vec::new();

    /* Una categoría caída no debe tumbar las otras dos: cada una guarda su
     * propio error y la pantalla sigue mostrando lo que sí respondió. */
    let terminales = match client.get_poses().await {
        Ok(valor) => maestros_de(&valor, &["POSes", "POSList"], &["Id"], &["Name"], None)
            .unwrap_or_else(|mensaje| {
                errores.push(format!("terminales: {mensaje}"));
                Vec::new()
            }),
        Err(error) => {
            errores.push(format!("terminales: {error}"));
            Vec::new()
        }
    };

    let empleados = match client
        .get_employees(&BdpGetEmployeesRequest {
            ids: Vec::new(),
            only_salespeople: None,
        })
        .await
    {
        Ok(valor) => maestros_de(
            &valor,
            &["Employees", "EmployeeList"],
            &["Id"],
            &["Name", "Nombre"],
            None,
        )
        .unwrap_or_else(|mensaje| {
            errores.push(format!("empleados: {mensaje}"));
            Vec::new()
        }),
        Err(error) => {
            errores.push(format!("empleados: {error}"));
            Vec::new()
        }
    };

    let perfiles = match client.get_profiles_create_article_list().await {
        Ok(valor) => maestros_de(
            &valor,
            &["ProfilesList", "Profiles"],
            &["Profile", "Id"],
            &["ProfileName", "Name", "Nombre"],
            None,
        )
        .unwrap_or_else(|mensaje| {
            errores.push(format!("perfiles: {mensaje}"));
            Vec::new()
        }),
        Err(error) => {
            errores.push(format!("perfiles: {error}"));
            Vec::new()
        }
    };

    Ok(Json(MaestrosBdp {
        terminales,
        empleados,
        perfiles,
        errores,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /* Las tres respuestas reales del BDP local (2026-10-21), recortadas. */
    #[test]
    fn lee_las_tres_listas_reales() {
        let terminales = maestros_de(
            &json!({"POSes": [{"Id": 1, "Name": "TERMINAL 1"}], "ErrorMessage": ""}),
            &["POSes"],
            &["Id"],
            &["Name"],
            None,
        )
        .expect("POSes debe parsear");
        assert_eq!(
            terminales,
            vec![MaestroBdp {
                id: 1,
                nombre: "TERMINAL 1".into(),
                detalle: None
            }]
        );

        let empleados = maestros_de(
            &json!({"Employees": [
                {"Id": 1, "Name": "Camarero 01", "Salesperson": true},
                {"Id": 2, "Name": "Camarero 02", "Salesperson": true}
            ]}),
            &["Employees"],
            &["Id"],
            &["Name"],
            None,
        )
        .expect("Employees debe parsear");
        assert_eq!(empleados.len(), 2);
        assert_eq!(empleados[1].nombre, "Camarero 02");

        let perfiles = maestros_de(
            &json!({"ProfilesList": [{"Profile": 1, "ProfileName": "PERFIL 1"}]}),
            &["ProfilesList"],
            &["Profile", "Id"],
            &["ProfileName"],
            None,
        )
        .expect("ProfilesList debe parsear");
        assert_eq!(perfiles[0].id, 1);
        assert_eq!(perfiles[0].nombre, "PERFIL 1");
    }

    /* Un 200 con `ErrorMessage` es un fallo, no una lista vacía. */
    #[test]
    fn error_remoto_no_es_lista_vacia() {
        let resultado = maestros_de(
            &json!({"POSes": [], "ErrorMessage": "Subscripción no activada"}),
            &["POSes"],
            &["Id"],
            &["Name"],
            None,
        );
        assert_eq!(
            resultado,
            Err("BDP respondió: Subscripción no activada".to_string())
        );
    }

    /* Los ids llegan como decimal en algunas respuestas del BDP. */
    #[test]
    fn identificador_decimal_se_normaliza() {
        assert_eq!(numero_de(Some(&json!(12.0))), Some(12));
        assert_eq!(numero_de(Some(&json!("3.0"))), Some(3));
        assert_eq!(numero_de(Some(&json!(" x "))), None);
        assert_eq!(numero_de(None), None);
    }
}
