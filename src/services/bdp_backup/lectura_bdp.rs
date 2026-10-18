use tracing::warn;

use super::BdpBackupService;
use crate::models::ConfiguracionRestaurante;
use crate::services::bdp_weblink::{response_error_message, BdpWeblinkClient, BdpWeblinkError};
use crate::services::bdp_weblink_catalog::{
    BdpExportArticlesRequest, BdpExportCustomersRequest, BdpExportDepartmentsRequest,
    BdpGetEmployeesRequest, BdpGetRoomsTablesRequest,
};

/* Lecturas de cada colección de BDP para los snapshots. Son auxiliares de
 * snapshot_bdp: cada una valida la respuesta antes de devolverla. */
impl BdpBackupService {
    pub(super) async fn fetch_articles(
        client: &BdpWeblinkClient<'_>,
        config: &ConfiguracionRestaurante,
    ) -> Result<serde_json::Value, String> {
        /* [048A-7] Fix: type_price usa bdp_catalog_price_type (1 = IVA incluido),
         * NO bdp_pos_id (31). El BDP real rechaza pos_id como tipo de precio:
         * [200106]-TIPO DE PRECIO INCORRECTO.
         *
         * [200109] (incidente 2026-09-05): si un artículo web quedó con datos
         * de validación rotos, ExportArticles falla con [200109]. En lugar de
         * abortar el snapshot completo (5 categorías), se cae al fallback de
         * perfil H-Q1-03 via list_profile_articles() (GetPOSList del perfil de
         * items), igual que sync_catalog. Solo [200109] cae al fallback; otros
         * errores de ExportArticles siguen abortando. */
        let export_result = client
            .export_articles(&BdpExportArticlesRequest::all_web_articles(
                config.bdp_catalog_price_type,
            ))
            .await;

        let value = match export_result {
            Ok(json) => json,
            Err(error) => {
                let msg = error.to_string();
                if !msg.contains("[200109]") {
                    return Err(format!("Snapshot BDP abortado al leer artículos: {msg}"));
                }
                warn!(
                    "[200109] ExportArticles falló → fetch_articles cae al fallback \
                     de perfil H-Q1-03 (GetPOSList)"
                );
                let perfil = client.list_profile_articles().await.map_err(|e| {
                    format!("ExportArticles [200109] y el fallback de perfil falló: {e}")
                })?;
                serde_json::json!({ "Articles": perfil, "ErrorMessage": "" })
            }
        };

        /* [200109/H-Q1-03] Nota: cuando se usó el fallback, el JSON devuelto
         * no tiene keys de ExportArticles nativo (ArticlesListData etc.), sino
         * {"Articles": [...]} armado en el fallback. validate_snapshot_response
         * acepta "Articles" (está en el array de keys), así que pasa igual. */
        Self::validate_snapshot_response(
            "artículos",
            value,
            &[
                "ArticlesListData",
                "ArticleListData",
                "Articles",
                "ArticleList",
            ],
        )
    }

    pub(super) async fn fetch_customers(
        client: &BdpWeblinkClient<'_>,
    ) -> Result<serde_json::Value, String> {
        let value = client
            .export_customers(&BdpExportCustomersRequest::default())
            .await
            .map_err(|error| format!("Snapshot BDP abortado al leer clientes: {error}"))?;
        Self::validate_snapshot_response("clientes", value, &["Customers", "CustomerList"])
    }

    pub(super) async fn fetch_departments(
        client: &BdpWeblinkClient<'_>,
    ) -> Result<serde_json::Value, String> {
        let value = client
            .export_departments(&BdpExportDepartmentsRequest::default())
            .await
            .map_err(|error| format!("Snapshot BDP abortado al leer departamentos: {error}"))?;
        /* [048A-7] El BDP real devuelve la key DepartmentListData (con un
         * ErrorMessage posicional de .NET cuando no hay items). */
        Self::validate_snapshot_response(
            "departamentos",
            value,
            &[
                "Departments",
                "Department",
                "DepartmentList",
                "DepartmentListData",
            ],
        )
    }

    pub(super) async fn fetch_rooms(
        client: &BdpWeblinkClient<'_>,
    ) -> Result<serde_json::Value, String> {
        let value = match client
            .get_rooms_tables(&BdpGetRoomsTablesRequest::default())
            .await
        {
            Ok(value) => value,
            /* [048A-7] El BDP real no tiene contratada la subscripción de salones:
             * get_rooms_tables lanza BdpWeblinkError::Remote("Subscripción no
             * activada") por el ErrorMessage, ANTES de llegar al análisis de la
             * respuesta. No es fallo de red ni de parseo; el módulo simplemente
             * no está activo. Se guarda [] para no abortar el snapshot completo
             * y queda trazado en metadata/log. */
            Err(BdpWeblinkError::Remote(message))
                if message.trim() == "Subscripción no activada" =>
            {
                warn!("Snapshot BDP: salones no disponibles para esta conexión ({message})");
                return Ok(serde_json::json!([]));
            }
            Err(error) => {
                return Err(format!("Snapshot BDP abortado al leer salones: {error}"));
            }
        };
        /* Si pese a todo llega respuesta sin colección (Rooms:null sin
         * ErrorMessage), también se tolera como []. */
        if !value.get("Rooms").is_some_and(serde_json::Value::is_array) {
            warn!(
                "Snapshot BDP: salones no disponibles para esta conexión ({})",
                response_error_message(&value).unwrap_or_default()
            );
            return Ok(serde_json::json!([]));
        }
        Self::validate_snapshot_response("salones", value, &["Rooms", "RoomList"])
    }

    pub(super) async fn fetch_employees(
        client: &BdpWeblinkClient<'_>,
    ) -> Result<serde_json::Value, String> {
        let value = client
            .get_employees(&BdpGetEmployeesRequest {
                ids: vec![],
                only_salespeople: None,
            })
            .await
            .map_err(|error| format!("Snapshot BDP abortado al leer empleados: {error}"))?;
        Self::validate_snapshot_response(
            "empleados",
            value,
            &["Employees", "Employee", "EmployeeList"],
        )
    }

    fn validate_snapshot_response(
        label: &str,
        value: serde_json::Value,
        expected_keys: &[&str],
    ) -> Result<serde_json::Value, String> {
        /* [048A-7] Se valida primero la colección: si existe una key reconocible
         * con payload no-nulo (aunque array vacío, p.ej. DepartmentListData:[]
         * con un ErrorMessage posicional de .NET), la lectura es válida y no
         * debe abortar. El ErrorMessage solo se usa como contexto cuando no
         * hay colección (caso salones con Rooms:null). */
        let has_payload = expected_keys
            .iter()
            .any(|key| value.get(*key).is_some_and(|payload| !payload.is_null()));
        if !has_payload {
            if let Some(error) = response_error_message(&value) {
                return Err(format!(
                    "Snapshot BDP abortado: la lectura de {label} devolvió {error}"
                ));
            }
            return Err(format!(
                "Snapshot BDP abortado: la lectura de {label} no contiene una colección reconocible"
            ));
        }
        Ok(value)
    }
}
