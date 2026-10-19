use chrono::{NaiveTime, Utc};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::payload::{build_only_check_order, build_only_check_order_con_pago, first_article};
use super::{BdpDryRunArticle, BdpSyncDryRunResponse, BdpSyncPreflightService};
use crate::models::ConfiguracionRestaurante;
use crate::services::bdp_weblink::BdpWeblinkClient;

fn config() -> ConfiguracionRestaurante {
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
        bdp_base_url: "http://bdp.test".to_string(),
        bdp_login: "usuario".to_string(),
        bdp_password: "secreto".to_string(),
        bdp_integrator_code: "INTEGRADOR".to_string(),
        bdp_configurado: true,
        bdp_sync_enabled: true,
        bdp_pos_id: 31,
        bdp_employee_id: 1,
        bdp_items_profile_id: 1,
        bdp_catalog_price_type: 1,
        bdp_purchase_notes_profile_id: None,
        bdp_default_article_code: "GLORY".to_string(),
        bdp_default_article_name: "Servicio Glory".to_string(),
        bdp_tender_map: serde_json::json!({"efectivo": "1", "tarjeta": "2"}),
        bdp_order_type_map: serde_json::json!({"comedor": "0", "barra": "0"}),
        bdp_default_customer_code: "DEFAULT".to_string(),
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
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[test]
fn first_article_uses_real_bdp_fields() {
    let value = json!({
        "ArticleListData": [{
            "ArtCode": 1001,
            "ArtDescription": "COCA-COLA",
            "Price1": 1.05,
            "TAVPer": 10.0
        }]
    });

    let article = first_article(&value, 10.0).unwrap();

    assert_eq!(article.id, 1001);
    assert_eq!(article.name, "COCA-COLA");
    assert!((article.price - 1.05).abs() < f64::EPSILON);
    assert!((article.vat_pct - 10.0).abs() < f64::EPSILON);
}

#[test]
fn first_article_accepts_integer_prices() {
    let value = json!({
        "ArticleListData": [{
            "ArtCode": 1001,
            "ArtDescription": "MENU",
            "Price1": 12,
            "TAVPer": 10
        }]
    });

    let article = first_article(&value, 10.0).unwrap();

    assert!((article.price - 12.0).abs() < f64::EPSILON);
    assert!((article.vat_pct - 10.0).abs() < f64::EPSILON);
}

#[test]
fn first_article_accepts_real_bdp_articles_list_key() {
    let value = json!({
        "ArticlesListData": [{
            "ArtCode": 1001,
            "ArtDescription": "CAFE BOMBON",
            "Price1": 1.4,
            "TAVPer": 10.0
        }]
    });

    let article = first_article(&value, 10.0).unwrap();

    assert_eq!(article.id, 1001);
    assert_eq!(article.name, "CAFE BOMBON");
}

/* [R12] Test: first_article usa el IVA por defecto del config cuando BDP no lo devuelve */
#[test]
fn first_article_uses_default_iva_when_missing() {
    let value = json!({
        "ArticleListData": [{
            "ArtCode": 1001,
            "ArtDescription": "SIN IVA",
            "Price1": 5.0
        }]
    });

    let article = first_article(&value, 21.0).unwrap();
    assert!(
        (article.vat_pct - 21.0).abs() < f64::EPSILON,
        "Debe usar el default_iva_pct pasado como parámetro"
    );
}

#[test]
fn build_order_never_uses_create_operation() {
    let config = config();
    let article = BdpDryRunArticle {
        id: 1001,
        name: "COCA-COLA".to_string(),
        price: 1.05,
        vat_pct: 10.0,
    };
    let order = build_only_check_order(&config, &article);

    assert_eq!(order.order_operation_type, 1);
    /* [169A-2] EndType como el flujo real: el valor configurable (por defecto 0). */
    assert_eq!(order.order_end_type, config.bdp_order_end_type);
    assert_eq!(order.invoice, Some(false));
    assert_eq!(order.order["Items"][0]["Id"], 1001);
    assert_eq!(order.order["AlreadyInvoiced"], false);
    assert!(order.order.get("Payments").is_none());
    assert!(order.order["MarketplaceOrderId"].as_str().unwrap().len() <= 15);
}

/* [169A-2] P1 silencioso: el OnlyCheck con pago replica el payload real
 * (EndType=1, Payments=total, PaymentId=MarketplaceOrderId, Invoice=true)
 * sin crear nada (OperationType=1). */
#[test]
fn only_check_con_pago_replica_payload_real() {
    let config = config();
    let article = BdpDryRunArticle {
        id: 1001,
        name: "COCA-COLA".to_string(),
        price: 1.05,
        vat_pct: 10.0,
    };
    let order = build_only_check_order_con_pago(&config, &article, 1);

    assert_eq!(order.order_operation_type, 1, "OnlyCheck: no crea nada");
    assert_eq!(order.order_end_type, config.bdp_order_end_type, "EndType configurable");
    assert_eq!(order.invoice, Some(true));
    let payments = order
        .order
        .get("Payments")
        .and_then(Value::as_array)
        .expect("Payments presente");
    assert_eq!(payments.len(), 1);
    assert_eq!(payments[0].get("TenderId").and_then(Value::as_i64), Some(1));
    assert!(
        (payments[0].get("Amount").and_then(Value::as_f64).unwrap() - 1.05).abs() < f64::EPSILON
    );
    assert_eq!(
        payments[0].get("Amount").and_then(Value::as_f64),
        order.order.get("Total").and_then(Value::as_f64),
        "el pago iguala el total (si no, BDP no factura)"
    );
    let market_id = order
        .order
        .get("MarketplaceOrderId")
        .and_then(Value::as_str)
        .unwrap();
    assert_eq!(
        payments[0].get("PaymentId").and_then(Value::as_str),
        Some(market_id)
    );
    assert!(market_id.len() <= 15);
}

/* [169A-2] Cableado P1: el OnlyCheck con tender válido del POS valida el
 * payload de pago-en-creación contra BDP (mock) sin crear nada. */
fn dry_run_articles() -> Value {
    json!({
        "ArticleListData": [{
            "ArtCode": 1001,
            "ArtDescription": "CAFE",
            "Price1": 1.10,
            "TAVPer": 10.0
        }]
    })
}

fn dry_run_tenders() -> Value {
    json!({
        "TenderList": [
            {"Id": 1, "Name": "EFECTIVO"},
            {"Id": 2, "Name": "TARJETA"}
        ]
    })
}

fn mock_config(base_url: String) -> ConfiguracionRestaurante {
    let mut cfg = config();
    cfg.bdp_base_url = base_url;
    cfg
}

async fn mock_bdp_only_check(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/Auth/Login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ErrorMessage": "",
            "AuthSession": {"Token": "token-test", "ExpiresIn_InSecconds": 3540}
        })))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/API/Orders/Create"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ErrorMessage": ""})))
        .mount(server)
        .await;
}

#[tokio::test]
async fn check_order_only_sin_tender_mantiene_dry_run_clasico() {
    let server = MockServer::start().await;
    mock_bdp_only_check(&server).await;
    let cfg = mock_config(server.uri());
    let client = BdpWeblinkClient::new(&cfg);
    let articles = dry_run_articles();
    let tenders = dry_run_tenders();
    let mut response = BdpSyncDryRunResponse::new(true);

    BdpSyncPreflightService::check_order_only(
        &cfg,
        &client,
        Some(&articles),
        Some(&tenders),
        None,
        &mut response,
    )
    .await;

    let check = response.checks.last().expect("check OnlyCheck");
    assert!(check.ok, "OnlyCheck clásico aceptado: {}", check.mensaje);
    assert_eq!(check.nombre, "CreateOrder OnlyCheck");
    let preview = response
        .payload_preview
        .as_ref()
        .expect("preview del payload");
    assert!(preview
        .get("Order")
        .and_then(|o| o.get("Payments"))
        .is_none());
}

#[tokio::test]
async fn check_order_only_con_pago_valido_envia_payments_en_only_check() {
    let server = MockServer::start().await;
    mock_bdp_only_check(&server).await;
    let cfg = mock_config(server.uri());
    let client = BdpWeblinkClient::new(&cfg);
    let articles = dry_run_articles();
    let tenders = dry_run_tenders();
    let mut response = BdpSyncDryRunResponse::new(true);

    BdpSyncPreflightService::check_order_only(
        &cfg,
        &client,
        Some(&articles),
        Some(&tenders),
        Some(1),
        &mut response,
    )
    .await;

    let check = response.checks.last().expect("check OnlyCheck con pago");
    assert!(check.ok, "OnlyCheck con pago aceptado: {}", check.mensaje);
    assert_eq!(check.nombre, "CreateOrder OnlyCheck con pago");
    let preview = response
        .payload_preview
        .as_ref()
        .expect("preview del payload");
    assert_eq!(
        preview.get("OrderOperationType").and_then(Value::as_i64),
        Some(1),
        "sigue siendo OnlyCheck: cero creación"
    );
    assert_eq!(
        preview.get("OrderEndType").and_then(Value::as_i64),
        Some(0),
        "OrderEndType por defecto: factura tras pagar"
    );
    assert_eq!(preview.get("Invoice").and_then(Value::as_bool), Some(true));
    let payments = preview
        .get("Order")
        .and_then(|o| o.get("Payments"))
        .and_then(Value::as_array)
        .expect("Payments en el preview");
    assert_eq!(payments.len(), 1);
    assert_eq!(payments[0].get("TenderId").and_then(Value::as_i64), Some(1));
}

/* [169A-2] Tender inexistente en el POS: error local, BDP ni se llama. */
#[tokio::test]
async fn check_order_only_con_pago_rechaza_tender_desconocido_sin_red() {
    let server = MockServer::start().await;
    let cfg = mock_config(server.uri());
    let client = BdpWeblinkClient::new(&cfg);
    let articles = dry_run_articles();
    let tenders = dry_run_tenders();
    let mut response = BdpSyncDryRunResponse::new(true);

    BdpSyncPreflightService::check_order_only(
        &cfg,
        &client,
        Some(&articles),
        Some(&tenders),
        Some(99),
        &mut response,
    )
    .await;

    let check = response.checks.last().expect("check de error");
    assert!(!check.ok);
    assert_eq!(check.nombre, "CreateOrder OnlyCheck con pago");
    assert!(
        server
            .received_requests()
            .await
            .unwrap_or_default()
            .is_empty(),
        "tender inválido: ninguna llamada de red"
    );
}
