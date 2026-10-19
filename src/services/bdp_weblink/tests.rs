/* Tests del cliente BDP: destino de escritura, sesion y envio de peticiones. */

use serde_json::Value;

use crate::models::ConfiguracionRestaurante;
use crate::services::bdp_weblink_catalog::{BdpCreateOrderRequest, BdpExportArticlesRequest};

use super::tipos::{BdpWeblinkClient, BdpWeblinkError};
use super::transporte::response_error_message;

use chrono::{NaiveTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub(super) fn config(base_url: String) -> ConfiguracionRestaurante {
    ConfiguracionRestaurante {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        reserva_email_obligatorio: false,
        reserva_telefono_obligatorio: true,
        reserva_nombre_obligatorio: true,
        reserva_apellidos_obligatorio: false,
        iva_por_defecto: Decimal::new(10, 0),
        nombre_restaurante: "Nakomi".to_string(),
        groq_api_key: None,
        auto_venta_reserva: true,
        hora_desayuno_inicio: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
        hora_desayuno_fin: NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
        hora_comida_inicio: NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
        hora_comida_fin: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
        hora_cena_inicio: NaiveTime::from_hms_opt(20, 0, 0).unwrap(),
        hora_cena_fin: NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
        url_haddock: String::new(),
        haddock_api_token: String::new(),
        haddock_sync_enabled: false,
        bdp_base_url: base_url,
        bdp_login: "usuario".to_string(),
        bdp_password: "secreto".to_string(),
        bdp_integrator_code: "INTEGRADOR".to_string(),
        bdp_configurado: true,
        bdp_sync_enabled: true,
        bdp_pos_id: 1,
        bdp_employee_id: 1,
        bdp_items_profile_id: 1,
        bdp_catalog_price_type: 1,
        bdp_purchase_notes_profile_id: None,
        bdp_default_article_code: String::new(),
        bdp_default_article_name: String::new(),
        bdp_tender_map: serde_json::json!({}),
        bdp_order_type_map: serde_json::json!({}),
        bdp_default_customer_code: String::new(),
        bdp_poll_interval_secs: 60,
        bdp_poll_enabled: false,
        google_review_url: String::new(),
        telefono_restaurante: String::new(),
        url_reservas: String::new(),
        bdp_auto_sync_customers: false,
        bdp_sync_mode: "read_only".to_string(),
        bdp_backup_retention_days: 30,
        bdp_auto_backup_before_write: true,
        bdp_env_bootstrap_applied_at: None,
        /* [XT2-1] Feature flags BDP desactivados por defecto en tests */
        ff_bdp_auto_arm: false,
        ff_bdp_partial_payments: false,
        ff_bdp_cancel_order: false,
        ff_bdp_purchase_notes_read: false,
        ff_bdp_purchase_notes_draft: false,
        ff_bdp_purchase_notes_receive: false,
        modo_operacion: "auto".to_string(),
        anulacion_modalidad: "credito_completo".to_string(),
        permisos_catalogo_edicion: "admin".to_string(),
        permisos_stock_ajuste: "admin".to_string(),
        permisos_albaranes_gestion: "admin".to_string(),
        permisos_anulacion_ventas: "admin".to_string(),
        permisos_pagos_locales: "admin".to_string(),
        permisos_facturacion_local: "admin".to_string(),
        push_modalidad: "automatico".to_string(),
        bdp_tav_map: serde_json::json!({}),
        bdp_almacen_default: 1,
        bdp_codreg_default: 1,
        bdp_articulo_rango_inicial: 90_000_000,
        bdp_order_end_type: 0,
        bdp_billing_details: false,
        bdp_overwrite_clientes: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[test]
fn external_write_and_only_check_are_denied_without_explicit_allowlist() {
    let config = config("http://192.0.2.10:8068".to_string());
    let client = BdpWeblinkClient::new(&config);

    assert!(client
        .ensure_target_allowed("BDP_TEST_ALLOWLIST_THAT_DOES_NOT_EXIST")
        .is_err());
}

#[test]
fn loopback_simulator_is_allowed_without_external_allowlist() {
    let config = config("http://127.0.0.1:18765".to_string());
    let client = BdpWeblinkClient::new(&config);

    assert!(client
        .ensure_target_allowed("BDP_TEST_ALLOWLIST_THAT_DOES_NOT_EXIST")
        .is_ok());
}

#[test]
fn write_target_rejects_embedded_path_or_credentials() {
    for base_url in [
        "http://127.0.0.1:18765/otra-ruta",
        "http://usuario:clave@127.0.0.1:18765",
    ] {
        let config = config(base_url.to_string());
        let client = BdpWeblinkClient::new(&config);
        assert!(client
            .ensure_target_allowed("BDP_TEST_ALLOWLIST_THAT_DOES_NOT_EXIST")
            .is_err());
    }
}

#[tokio::test]
async fn health_posts_to_service_health() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Service/Health"))
        .and(body_json(serde_json::json!({})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "IsAlive": true
        })))
        .mount(&server)
        .await;

    let config = config(server.uri());
    let client = BdpWeblinkClient::new(&config);
    let health = client.health().await.unwrap();

    assert!(health.is_alive);
}

#[tokio::test]
async fn login_uses_pascal_case_payload() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Auth/Login"))
        .and(body_json(serde_json::json!({
            "Login": "usuario",
            "Password": "secreto",
            "TiempoSession": 59,
            "CodigoIntegrador": "INTEGRADOR"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "AuthSession": {
                "Token": "token-bdp",
                "ExpiresIn_InSecconds": 3540
            }
        })))
        .mount(&server)
        .await;

    let config = config(server.uri());
    let client = BdpWeblinkClient::new(&config);
    let session = client.login().await.unwrap();

    assert_eq!(session.token, "token-bdp");
    assert_eq!(session.expires_in_seconds, 3540);
}

#[tokio::test]
async fn authenticated_calls_use_bearer_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/API/Tenders/GetList"))
        .and(header("authorization", "Bearer token-bdp"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "TenderList": []
        })))
        .mount(&server)
        .await;

    let config = config(server.uri());
    let client = BdpWeblinkClient::new(&config);
    let response: Value = client
        .post_authenticated("/API/Tenders/GetList", &serde_json::json!({}), "token-bdp")
        .await
        .unwrap();

    assert_eq!(response_error_message(&response), None);
}

#[test]
fn external_write_target_is_denied_by_default() {
    let config = config("https://bdp.example.invalid".to_string());
    let client = BdpWeblinkClient::new(&config);
    assert!(matches!(
        client.ensure_write_target_allowed(),
        Err(BdpWeblinkError::WriteTargetDenied(_))
    ));
}

#[test]
fn loopback_write_target_is_allowed_for_simulator() {
    let config = config("http://127.0.0.1:18765".to_string());
    let client = BdpWeblinkClient::new(&config);
    assert!(client.ensure_write_target_allowed().is_ok());
}

#[tokio::test]
async fn export_articles_logs_in_and_posts_catalog_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Auth/Login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "AuthSession": {
                "Token": "token-bdp",
                "ExpiresIn_InSecconds": 3540
            }
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/API/Articles/Export"))
        .and(header("authorization", "Bearer token-bdp"))
        .and(body_json(serde_json::json!({
            "Dept1": 1,
            "Dept2": 999,
            "Art1": 1,
            "Art2": 9_999_999_999_999_i64,
            "Modified": false,
            "TypePrice": 1,
            "Disc": 0
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "Articles": []
        })))
        .mount(&server)
        .await;

    let config = config(server.uri());
    let client = BdpWeblinkClient::new(&config);
    let response = client
        .export_articles(&BdpExportArticlesRequest::all_web_articles(1))
        .await
        .unwrap();

    assert!(response["Articles"].is_array());
}

#[tokio::test]
async fn check_order_forces_only_check_mode() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Auth/Login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "AuthSession": {
                "Token": "token-bdp",
                "ExpiresIn_InSecconds": 3540
            }
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/API/Orders/Create"))
        .and(header("authorization", "Bearer token-bdp"))
        .and(body_json(serde_json::json!({
            "EmployeeId": 1,
            "ItemsProfileId": 1,
            "OrderEndType": 0,
            "OrderOperationType": 1,
            "Order": { "Items": [] }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": ""
        })))
        .mount(&server)
        .await;

    let config = config(server.uri());
    let client = BdpWeblinkClient::new(&config);
    let response = client
        .check_order(&BdpCreateOrderRequest {
            employee_id: 1,
            items_profile_id: 1,
            order_end_type: 0,
            order_operation_type: 0,
            invoice: None,
            order: serde_json::json!({ "Items": [] }),
        })
        .await
        .unwrap();

    assert_eq!(response_error_message(&response), None);
}
