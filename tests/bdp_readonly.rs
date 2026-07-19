/* [BDP-C] Tests de integración read-only contra BDP real.
 * Estos tests NO escriben ni modifican nada en el sistema BDP.
 * Solo hacen llamadas de lectura: health, login, export_articles, get_tenders.
 *
 * Para ejecutar:
 *   BDP_BASE_URL=http://... BDP_LOGIN=admin BDP_PASSWORD=pass BDP_INTEGRATOR_CODE=TEST cargo test --test bdp_readonly -- --include-ignored
 *
 * Si las env vars no están configurados, los tests se ignoran automáticamente. */

use chrono::{NaiveTime, Utc};
use glory_backend::models::ConfiguracionRestaurante;
use glory_backend::services::bdp_weblink::BdpWeblinkClient;
use glory_backend::services::bdp_weblink_catalog::{
    BdpExportArticlesRequest, BdpGetOrderRequest, BdpOrderIdentifier,
};
use rust_decimal::Decimal;
use serde_json::json;
use uuid::Uuid;

/* Helper: construye ConfiguracionRestaurante desde env vars.
 * Si falta BDP_BASE_URL, retorna None (tests se ignoran). */
fn bdp_config_from_env() -> Option<ConfiguracionRestaurante> {
    let base_url = std::env::var("BDP_BASE_URL").ok()?;
    if base_url.is_empty() {
        return None;
    }
    Some(ConfiguracionRestaurante {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        reserva_email_obligatorio: false,
        reserva_telefono_obligatorio: true,
        reserva_nombre_obligatorio: true,
        reserva_apellidos_obligatorio: false,
        iva_por_defecto: Decimal::new(10, 0),
        nombre_restaurante: "Test".to_string(),
        groq_api_key: None,
        auto_venta_reserva: false,
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
        bdp_login: std::env::var("BDP_LOGIN").unwrap_or_else(|_| "admin".into()),
        bdp_password: std::env::var("BDP_PASSWORD").unwrap_or_default(),
        bdp_integrator_code: std::env::var("BDP_INTEGRATOR_CODE").unwrap_or_default(),
        bdp_sync_enabled: true,
        bdp_pos_id: std::env::var("BDP_POS_ID")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1),
        bdp_employee_id: std::env::var("BDP_EMPLOYEE_ID")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1),
        bdp_items_profile_id: std::env::var("BDP_ITEMS_PROFILE_ID")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1),
        bdp_default_article_code: "1001".into(),
        bdp_default_article_name: "Servicio".into(),
        bdp_tender_map: json!({"efectivo": "1", "tarjeta": "2"}),
        bdp_order_type_map: json!({"comedor": "0", "barra": "0"}),
        bdp_default_customer_code: String::new(),
        bdp_poll_interval_secs: 60,
        bdp_poll_enabled: false,
        bdp_auto_sync_customers: false,
        bdp_sync_mode: "read_only".to_string(),
        bdp_backup_retention_days: 30,
        bdp_auto_backup_before_write: true,
        bdp_env_bootstrap_applied_at: None,
        google_review_url: String::new(),
        telefono_restaurante: String::new(),
        url_reservas: String::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

macro_rules! skip_if_no_bdp {
    ($config:ident) => {
        let $config = match bdp_config_from_env() {
            Some(c) => c,
            None => {
                eprintln!("SKIP: BDP_BASE_URL no configurado — test ignorado");
                return;
            }
        };
    };
}

/* ──────────────────────────────────────────────────────
 * Test 1: Health check al servicio BDP
 * Endpoint: POST /service/health (sin auth)
 * ────────────────────────────────────────────────────── */
#[tokio::test]
#[ignore] /* Solo ejecutar con BDP_BASE_URL configurado */
async fn bdp_real_health() {
    skip_if_no_bdp!(config);
    let client = BdpWeblinkClient::new(&config);

    let result = client.health().await;
    match &result {
        Ok(h) => {
            println!("BDP health: is_alive={}", h.is_alive);
            assert!(h.is_alive, "BDP should report is_alive=true");
        }
        Err(e) => {
            /* Si BDP no está corriendo, no fallamos — es un error de infraestructura */
            eprintln!("SKIP: BDP health failed (servidor no disponible?): {e}");
        }
    }
}

/* ──────────────────────────────────────────────────────
 * Test 2: Login y obtención de token
 * Endpoint: POST /auth/login
 * ────────────────────────────────────────────────────── */
#[tokio::test]
#[ignore]
async fn bdp_real_login() {
    skip_if_no_bdp!(config);
    let client = BdpWeblinkClient::new(&config);

    let result = client.login().await;
    match &result {
        Ok(session) => {
            println!(
                "BDP login OK: token_len={}, expires_in={}s",
                session.token.len(),
                session.expires_in_seconds
            );
            assert!(!session.token.is_empty(), "Token should not be empty");
            assert!(
                session.expires_in_seconds > 0,
                "Session should expire in the future"
            );
        }
        Err(e) => {
            eprintln!("FAIL: BDP login failed: {e}");
            panic!("BDP login failed — verify BDP_LOGIN/BDP_PASSWORD/BDP_INTEGRATOR_CODE: {e}");
        }
    }
}

/* ──────────────────────────────────────────────────────
 * Test 3: Export articles (catálogo de artículos)
 * Endpoint: POST /articles/export — solo lectura
 * ────────────────────────────────────────────────────── */
#[tokio::test]
#[ignore]
async fn bdp_real_export_articles() {
    skip_if_no_bdp!(config);
    let client = BdpWeblinkClient::new(&config);

    let request = BdpExportArticlesRequest::all_web_articles(1);

    let result = client.export_articles(&request).await;
    match &result {
        Ok(articles) => {
            let count = articles.as_array().map_or(0, std::vec::Vec::len);
            println!("BDP export_articles: {count} artículos recibidos");
            if let Some(first) = articles.as_array().and_then(|a| a.first()) {
                println!("  Primer artículo: {first}");
            }
        }
        Err(e) => {
            eprintln!("FAIL: BDP export_articles failed: {e}");
            panic!("export_articles should succeed: {e}");
        }
    }
}

/* ──────────────────────────────────────────────────────
 * Test 4: Get tenders (métodos de pago)
 * Endpoint: POST /tenders/get — solo lectura
 * ────────────────────────────────────────────────────── */
#[tokio::test]
#[ignore]
async fn bdp_real_get_tenders() {
    skip_if_no_bdp!(config);
    let client = BdpWeblinkClient::new(&config);

    let result = client.get_tenders().await;
    match &result {
        Ok(tenders) => {
            let pretty = serde_json::to_string_pretty(&tenders).unwrap_or_default();
            println!("BDP get_tenders: {pretty}");
        }
        Err(e) => {
            eprintln!("FAIL: BDP get_tenders failed: {e}");
            panic!("get_tenders should succeed: {e}");
        }
    }
}

/* ──────────────────────────────────────────────────────
 * Test 5: Get order inexistente — no crea nada en BDP
 * Endpoint: POST /order/get con código inexistente
 * ────────────────────────────────────────────────────── */
#[tokio::test]
#[ignore]
async fn bdp_real_get_order_inexistente() {
    skip_if_no_bdp!(config);
    let client = BdpWeblinkClient::new(&config);

    let request = BdpGetOrderRequest {
        order_identifier: BdpOrderIdentifier {
            order_id: None,
            market_id: None,
            marketplace_order_id: Some("GLORY_TEST_NOEXISTE_99999".to_string()),
            room_number: None,
            table_number: None,
        },
    };

    let result = client.get_order(&request).await;
    match &result {
        Ok(value) => {
            println!(
                "BDP get_order(inexistente): {}",
                serde_json::to_string_pretty(&value).unwrap_or_default()
            );
            println!("Respuesta recibida sin error HTTP — OK");
        }
        Err(e) => {
            /* BDP puede devolver error 4xx para "not found" — comportamiento esperado */
            println!("BDP get_order(inexistente) error (esperado): {e}");
        }
    }
}

/* ──────────────────────────────────────────────────────
 * Test 6: Flujo login → export_articles
 * Valida que el token funciona para llamadas autenticadas.
 * ────────────────────────────────────────────────────── */
#[tokio::test]
#[ignore]
async fn bdp_real_login_then_export_articles() {
    skip_if_no_bdp!(config);
    let client = BdpWeblinkClient::new(&config);

    let _session = match client.login().await {
        Ok(s) => {
            println!("Login OK, token expires in {}s", s.expires_in_seconds);
            s
        }
        Err(e) => {
            eprintln!("SKIP: Login failed: {e}");
            return;
        }
    };

    let request = BdpExportArticlesRequest::all_web_articles(1);

    let articles = match client.export_articles(&request).await {
        Ok(a) => a,
        Err(e) => {
            panic!("export_articles failed after successful login: {e}");
        }
    };

    let count = articles.as_array().map_or(0, std::vec::Vec::len);
    println!("Flujo completo login→export_articles: {count} artículos");
}
