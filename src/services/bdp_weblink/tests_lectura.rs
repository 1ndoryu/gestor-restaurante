#![cfg(test)]
/* [D24-m] Lectura de formas de pago y empleados: las llamadas solo deben usar rutas Get* (sin altas ni cambios). */

use crate::services::bdp_weblink_catalog::{BdpExportDocumentsRequest, BdpGetEmployeesRequest};

use super::tests::config;
use super::tipos::BdpWeblinkClient;

use wiremock::matchers::{body_json, method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/* Monta login y cualquier ruta POST; devuelve la lista de rutas recibidas. */
async fn montar_servidor() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path_regex("^/Auth/Login$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "AuthSession": { "Token": "token-bdp", "ExpiresIn_InSecconds": 3540 }
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path_regex("^/API/.*$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "TenderList": [],
            "EmployeeList": []
        })))
        .mount(&server)
        .await;
    server
}

/* Solo se permiten login, exportaciones de lectura y rutas cuyo último segmento empieza por Get. */
async fn assert_solo_lectura(server: &MockServer) -> usize {
    let peticiones = server.received_requests().await.unwrap_or_default();
    for peticion in &peticiones {
        let ruta = peticion.url.path();
        let ultimo = ruta.rsplit('/').next().unwrap_or_default();
        assert!(
            ruta == "/Auth/Login" || ruta == "/API/ExportProfiles/Documents" || ultimo.starts_with("Get"),
            "ruta no permitida en lectura: {ruta}"
        );
    }
    peticiones.len()
}

#[tokio::test]
async fn get_tenders_solo_usa_ruta_de_lectura() {
    let server = montar_servidor().await;
    let config = config(server.uri());
    let client = BdpWeblinkClient::new(&config);

    client.get_tenders().await.expect("get_tenders debe responder");

    assert!(assert_solo_lectura(&server).await >= 2);
}

/* [D28] La exportación de documentos usa la ruta de lectura y el cuerpo en PascalCase que espera BDP. */
#[tokio::test]
async fn export_documents_envia_cuerpo_pascal_case_a_ruta_de_documentos() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Auth/Login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "AuthSession": { "Token": "token-bdp", "ExpiresIn_InSecconds": 3540 }
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/API/ExportProfiles/Documents"))
        .and(body_json(serde_json::json!({
            "ExportProfileCode": 1,
            "InitialDate": "2026-10-13",
            "FinalDate": "2026-10-20",
            "OnlyLastMovements": true
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "Documents": []
        })))
        .mount(&server)
        .await;

    let config = config(server.uri());
    let client = BdpWeblinkClient::new(&config);
    let request = BdpExportDocumentsRequest {
        export_profile_code: 1,
        initial_date: "2026-10-13".to_string(),
        final_date: "2026-10-20".to_string(),
        only_last_movements: true,
    };

    let response = client
        .export_documents(&request)
        .await
        .expect("export_documents debe responder");

    assert!(response["Documents"].is_array());
    assert_eq!(assert_solo_lectura(&server).await, 2);
}

#[tokio::test]
async fn get_employees_solo_usa_ruta_de_lectura() {
    let server = montar_servidor().await;
    let config = config(server.uri());
    let client = BdpWeblinkClient::new(&config);
    let request = BdpGetEmployeesRequest {
        ids: Vec::new(),
        only_salespeople: None,
    };

    client.get_employees(&request).await.expect("get_employees debe responder");

    assert!(assert_solo_lectura(&server).await >= 2);
}
