//! Verifica el aprovisionamiento dirigido sin contactar ningún BDP real.

use glory_backend::services::{
    BdpBootstrapOutcome, BdpBootstrapSettings, BdpConfigBootstrapService,
};
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "./migrations")]
async fn bootstrap_es_dirigido_idempotente_y_no_expone_secretos(pool: PgPool) {
    let user_id = Uuid::new_v4();
    let email = format!("bdp-bootstrap-{user_id}@example.com");
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(&email)
        .bind("argon2_hash_placeholder")
        .execute(&pool)
        .await
        .expect("crear cuenta objetivo");

    let settings = BdpBootstrapSettings {
        user_email: email,
        base_url: "http://127.0.0.1:8068".into(),
        login: "admin".into(),
        password: "secreto-bootstrap".into(),
        integrator_code: "integrador".into(),
        pos_id: 31,
        employee_id: 7,
        items_profile_id: 9,
        default_article_code: "1001".into(),
        default_article_name: "Servicio".into(),
        tender_map: serde_json::json!({"efectivo": 1}),
        order_type_map: serde_json::json!({"comedor": 1}),
        default_customer_code: "10".into(),
        poll_interval_secs: 60,
    };

    let outcome = BdpConfigBootstrapService::apply(&pool, &settings)
        .await
        .expect("aplicar bootstrap local");
    assert_eq!(outcome, BdpBootstrapOutcome::Applied { user_id });

    let (base_url, article_code, mode, sync_enabled, polling_enabled, applied): (
        String,
        String,
        String,
        bool,
        bool,
        bool,
    ) = sqlx::query_as(
        "SELECT bdp_base_url, bdp_default_article_code, bdp_sync_mode,
                bdp_sync_enabled, bdp_poll_enabled,
                bdp_env_bootstrap_applied_at IS NOT NULL
         FROM configuracion_restaurante WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("leer configuración aprovisionada");
    assert_eq!(base_url, settings.base_url);
    assert_eq!(article_code, "1001");
    assert_eq!(mode, "read_only");
    assert!(!sync_enabled);
    assert!(!polling_enabled);
    assert!(applied);

    let (operation, sent): (String, serde_json::Value) =
        sqlx::query_as("SELECT operacion, datos_enviados FROM bdp_audit_log WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .expect("leer auditoría del bootstrap");
    assert_eq!(operation, "config_bootstrap");
    assert!(!sent.to_string().contains(&settings.password));

    let repeated = BdpConfigBootstrapService::apply(&pool, &settings)
        .await
        .expect("repetir bootstrap local");
    assert_eq!(repeated, BdpBootstrapOutcome::AlreadyApplied { user_id });
    let audit_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM bdp_audit_log WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .expect("contar auditoría");
    assert_eq!(audit_count, 1);
}
