//! Pruebas SQLx del armado BDP. Solo usa una base temporal local; no hace HTTP.

use glory_backend::repositories::ConfiguracionRepository;
use glory_backend::services::{BdpBackupService, BdpWriteGuard};
use sqlx::PgPool;
use uuid::Uuid;

async fn create_test_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(format!("bdp-guard-{id}@example.com"))
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("crear usuario de prueba");
    id
}

#[sqlx::test(migrations = "./migrations")]
async fn armado_solo_se_consume_por_entidad_exacta(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let mut config = ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración");
    config.bdp_base_url = "http://127.0.0.1:18765".into();
    config.bdp_sync_mode = "unidirectional".into();
    sqlx::query(
        "UPDATE configuracion_restaurante SET bdp_base_url = $2, bdp_sync_mode = 'unidirectional' WHERE user_id = $1",
    )
    .bind(user_id)
    .bind(&config.bdp_base_url)
    .execute(&pool)
    .await
    .expect("configurar BDP local");

    let fingerprint = BdpBackupService::connection_fingerprint(&config).expect("fingerprint");
    let snapshot_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO bdp_snapshots
           (user_id, tipo, direccion, trigger_tipo, datos, target_base_url, connection_fingerprint)
           VALUES ($1, 'completo', 'bdp', 'manual', $2, $3, $4)
           RETURNING id"#,
    )
    .bind(user_id)
    .bind(serde_json::json!({
        "articulos": [], "clientes": [], "departamentos": [], "salones": [], "empleados": []
    }))
    .bind(&config.bdp_base_url)
    .bind(&fingerprint)
    .fetch_one(&pool)
    .await
    .expect("crear snapshot exacto");

    let venta_autorizada = Uuid::new_v4();
    let otra_venta = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO bdp_write_arming
           (user_id, base_url, scopes, target_entity_type, target_entity_id,
            reason, expires_at, remaining_operations, snapshot_id, connection_fingerprint)
           VALUES ($1, $2, ARRAY['create_order'], 'venta', $3,
                   'prueba local', NOW() + INTERVAL '5 minutes', 1, $4, $5)"#,
    )
    .bind(user_id)
    .bind(&config.bdp_base_url)
    .bind(venta_autorizada)
    .bind(snapshot_id)
    .bind(&fingerprint)
    .execute(&pool)
    .await
    .expect("crear armado");

    let denied = BdpWriteGuard::authorize(
        &pool,
        user_id,
        &config,
        "create_order",
        "venta",
        otra_venta,
        "venta_id",
        &serde_json::json!({"venta_id": otra_venta}),
        None,
        None,
    )
    .await;
    assert!(denied.is_err());

    let audit_id = BdpWriteGuard::authorize(
        &pool,
        user_id,
        &config,
        "create_order",
        "venta",
        venta_autorizada,
        "venta_id",
        &serde_json::json!({"venta_id": venta_autorizada}),
        None,
        None,
    )
    .await
    .expect("la entidad autorizada debe consumir el armado");
    let audit_result: String =
        sqlx::query_scalar("SELECT resultado FROM bdp_audit_log WHERE id = $1")
            .bind(audit_id)
            .fetch_one(&pool)
            .await
            .expect("leer auditoría");
    assert_eq!(audit_result, "pendiente");
    let (audit_reason, audit_snapshot_id): (Option<String>, Option<Uuid>) = sqlx::query_as(
        "SELECT authorization_reason, snapshot_pre_id FROM bdp_audit_log WHERE id = $1",
    )
    .bind(audit_id)
    .fetch_one(&pool)
    .await
    .expect("leer motivo y evidencia de auditoría");
    assert_eq!(audit_reason.as_deref(), Some("prueba local"));
    assert_eq!(audit_snapshot_id, Some(snapshot_id));
    let mode: String = sqlx::query_scalar(
        "SELECT bdp_sync_mode FROM configuracion_restaurante WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("leer modo");
    assert_eq!(mode, "read_only", "el kill switch se cierra antes del HTTP");

    sqlx::query(
        "UPDATE configuracion_restaurante SET bdp_sync_mode = 'unidirectional' WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("rearmar solo para probar bloqueo cruzado");
    sqlx::query(
        r#"INSERT INTO bdp_write_arming
           (user_id, base_url, scopes, target_entity_type, target_entity_id,
            reason, expires_at, remaining_operations, snapshot_id, connection_fingerprint)
           VALUES ($1, $2, ARRAY['invoice'], 'venta', $3,
                   'prueba bloqueo cruzado', NOW() + INTERVAL '5 minutes', 1, $4, $5)"#,
    )
    .bind(user_id)
    .bind(&config.bdp_base_url)
    .bind(venta_autorizada)
    .bind(snapshot_id)
    .bind(&fingerprint)
    .execute(&pool)
    .await
    .expect("crear segundo armado");

    let cross_operation = BdpWriteGuard::authorize(
        &pool,
        user_id,
        &config,
        "invoice",
        "venta",
        venta_autorizada,
        "venta_id",
        &serde_json::json!({"venta_id": venta_autorizada}),
        None,
        None,
    )
    .await;
    assert!(
        cross_operation.is_err(),
        "una intención pendiente debe bloquear cualquier otra escritura de la venta"
    );

    let second = BdpWriteGuard::authorize(
        &pool,
        user_id,
        &config,
        "create_order",
        "venta",
        venta_autorizada,
        "venta_id",
        &serde_json::json!({"venta_id": venta_autorizada}),
        None,
        None,
    )
    .await;
    assert!(second.is_err(), "el cupo no puede reutilizarse");
}

#[sqlx::test(migrations = "./migrations")]
async fn cambio_de_conexion_invalida_armado_sin_consumirlo(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let mut config = ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración");
    config.bdp_base_url = "http://127.0.0.1:18765".into();
    config.bdp_login = "usuario-original".into();
    config.bdp_password = "secreto-original".into();
    config.bdp_integrator_code = "integrador-local".into();
    config.bdp_sync_mode = "unidirectional".into();
    sqlx::query(
        r#"UPDATE configuracion_restaurante
           SET bdp_base_url = $2, bdp_login = $3, bdp_password = $4,
               bdp_integrator_code = $5, bdp_sync_mode = 'unidirectional'
           WHERE user_id = $1"#,
    )
    .bind(user_id)
    .bind(&config.bdp_base_url)
    .bind(&config.bdp_login)
    .bind(&config.bdp_password)
    .bind(&config.bdp_integrator_code)
    .execute(&pool)
    .await
    .expect("configurar conexión original");

    let original_fingerprint =
        BdpBackupService::connection_fingerprint(&config).expect("fingerprint original");
    let snapshot_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO bdp_snapshots
           (user_id, tipo, direccion, trigger_tipo, datos, target_base_url, connection_fingerprint)
           VALUES ($1, 'completo', 'bdp', 'manual', $2, $3, $4)
           RETURNING id"#,
    )
    .bind(user_id)
    .bind(serde_json::json!({
        "articulos": [], "clientes": [], "departamentos": [], "salones": [], "empleados": []
    }))
    .bind(&config.bdp_base_url)
    .bind(&original_fingerprint)
    .fetch_one(&pool)
    .await
    .expect("crear snapshot original");
    let venta_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO bdp_write_arming
           (user_id, base_url, scopes, target_entity_type, target_entity_id,
            reason, expires_at, remaining_operations, snapshot_id, connection_fingerprint)
           VALUES ($1, $2, ARRAY['create_order'], 'venta', $3,
                   'prueba cambio conexión', NOW() + INTERVAL '5 minutes', 1, $4, $5)"#,
    )
    .bind(user_id)
    .bind(&config.bdp_base_url)
    .bind(venta_id)
    .bind(snapshot_id)
    .bind(&original_fingerprint)
    .execute(&pool)
    .await
    .expect("crear armado original");

    config.bdp_password = "secreto-cambiado".into();
    sqlx::query("UPDATE configuracion_restaurante SET bdp_password = $2 WHERE user_id = $1")
        .bind(user_id)
        .bind(&config.bdp_password)
        .execute(&pool)
        .await
        .expect("cambiar conexión");

    let denied = BdpWriteGuard::authorize(
        &pool,
        user_id,
        &config,
        "create_order",
        "venta",
        venta_id,
        "venta_id",
        &serde_json::json!({"venta_id": venta_id}),
        None,
        None,
    )
    .await;
    assert!(denied.is_err());
    let remaining: i32 =
        sqlx::query_scalar("SELECT remaining_operations FROM bdp_write_arming WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .expect("armado debe permanecer");
    assert_eq!(remaining, 1);
    let audit_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM bdp_audit_log WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .expect("contar auditoría");
    assert_eq!(audit_count, 0);
}
