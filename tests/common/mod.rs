// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* Helpers compartidos de los tests de albaranes de compra BDP (ciclo de vida,
 * handlers y locales). Sin tests propios; el allow evita avisos de helpers que un
 * archivo no usa. */
#![allow(dead_code)]

use chrono::NaiveDate;
use glory_backend::config::AppConfig;
use glory_backend::middleware::AuthUser;
use glory_backend::models::{BdpPurchaseNoteListParams, UserRole};
use glory_backend::repositories::gasto::NuevoGasto;
use glory_backend::repositories::{ConfiguracionRepository, GastoRepository};
use glory_backend::services::bdp_weblink_catalog::BdpPurchaseNoteData;
use glory_backend::services::NotifHub;
use glory_backend::AppState;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

pub async fn create_test_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let email = format!("test-{id}@example.com");
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(&email)
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("crear usuario de prueba");
    id
}

/// Crea configuración por defecto y activa los feature flags de compras BDP.
pub async fn create_config_with_purchase_note_flags(pool: &PgPool, user_id: Uuid) {
    ConfiguracionRepository::obtener_o_crear(pool, user_id)
        .await
        .expect("obtener_o_crear configuración");

    sqlx::query(
        "UPDATE configuracion_restaurante \
         SET ff_bdp_purchase_notes_read = TRUE, \
             ff_bdp_purchase_notes_draft = TRUE, \
             ff_bdp_purchase_notes_receive = TRUE \
         WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(pool)
    .await
    .expect("activar flags de compras BDP");
}

pub fn sample_purchase_note_data(
    serie: &str,
    numero: &str,
    total: Option<Decimal>,
) -> BdpPurchaseNoteData {
    BdpPurchaseNoteData {
        serie_albaran: Some(serie.to_string()),
        num_albaran: Some(numero.to_string()),
        fecha_albaran: Some("2024-07-20".to_string()),
        cod_proveedor: Some(serde_json::json!("PROV-001")),
        nom_proveedor: Some("Proveedor Test".to_string()),
        total_albaran: total,
        extra: serde_json::json!({}),
    }
}

pub async fn create_test_gasto(
    pool: &PgPool,
    user_id: Uuid,
    importe: Decimal,
) -> glory_backend::models::Gasto {
    let nuevo = NuevoGasto {
        user_id,
        fecha: NaiveDate::from_ymd_opt(2024, 7, 20).unwrap(),
        proveedor: "Proveedor Test",
        categoria_id: None,
        tipo_documento: "albaran",
        metodo_pago: "",
        numero_documento: "A-100",
        recurrente: false,
        importe_base: importe,
        importe_iva: Decimal::ZERO,
    };
    GastoRepository::create(pool, &nuevo)
        .await
        .expect("crear gasto de prueba")
}

pub fn default_filters() -> BdpPurchaseNoteListParams {
    BdpPurchaseNoteListParams {
        proveedor: None,
        fecha_desde: None,
        fecha_hasta: None,
    }
}

pub fn make_app_state(pool: PgPool) -> AppState {
    let notif_hub = Arc::new(NotifHub::new());
    AppState {
        pool,
        jwt_secret: "test-secret".to_string(),
        config: AppConfig {
            database_url: "postgres://localhost".to_string(),
            jwt_secret: "test-secret".to_string(),
            host: "127.0.0.1".to_string(),
            port: 3000,
            smtp: None,
            app_url: "http://localhost".to_string(),
            error_report_email: None,
        },
        notif_hub,
        modo_operacion: glory_backend::services::ServicioModoOperacion::default(),
        resumen_cache: Default::default(),
        listados_cache: Default::default(),
    }
}

pub fn make_auth(user_id: Uuid) -> AuthUser {
    AuthUser {
        user_id,
        role: UserRole::Admin,
        effective_role: UserRole::Admin,
        impersonator: None,
        trabajador_id: None,
    }
}
