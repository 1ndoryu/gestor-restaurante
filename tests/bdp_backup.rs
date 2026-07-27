/* [BKP-007] Tests de integración DB para BdpBackupService.
 * Usa #[sqlx::test(migrations = "./migrations")] — BD temporal, migraciones automáticas.
 * NO contacta al servidor BDP — solo valida operaciones contra PostgreSQL.
 * Cubre: snapshot_glory, CRUD snapshots, audit log, restauración, limpieza expirados. */

use sqlx::PgPool;
use uuid::Uuid;

use glory_backend::services::{BdpBackupService, BdpSnapshot};

/* ========== Helpers ========== */

async fn create_test_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let email = format!("bkp-test-{id}@example.com");
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(&email)
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("create_test_user failed");
    id
}

/// Crea configuración mínima para el usuario (necesaria para `retention/auto_backup` defaults).
async fn create_test_config(pool: &PgPool, user_id: Uuid) {
    sqlx::query(
        r"INSERT INTO configuracion_restaurante (user_id, nombre_restaurante, bdp_auto_backup_before_write)
        VALUES ($1, 'Test Restaurant', true)
        ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user_id)
    .execute(pool)
    .await
    .expect("create_test_config failed");
}

/// Inserta un snapshot directamente sin pasar por los fetchers de BDP (para tests de restauración).
async fn insert_snapshot_for_test(
    pool: &PgPool,
    user_id: Uuid,
    tipo: &str,
    direccion: &str,
    trigger_tipo: &str,
    datos: serde_json::Value,
) -> BdpSnapshot {
    sqlx::query_as::<_, BdpSnapshot>(
        r"INSERT INTO bdp_snapshots (user_id, tipo, direccion, trigger_tipo, datos)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, user_id, tipo, direccion, trigger_tipo, datos, metadata,
                  target_base_url, connection_fingerprint, created_at, expires_at, notas",
    )
    .bind(user_id)
    .bind(tipo)
    .bind(direccion)
    .bind(trigger_tipo)
    .bind(datos)
    .fetch_one(pool)
    .await
    .expect("insert_snapshot_for_test failed")
}

async fn insert_audit_for_test(
    pool: &PgPool,
    user_id: Uuid,
    operacion: &str,
    datos: serde_json::Value,
) -> Uuid {
    sqlx::query_scalar(
        r"INSERT INTO bdp_audit_log
           (user_id, operacion, direccion, datos_enviados, resultado)
           VALUES ($1, $2, 'glory_to_bdp', $3, 'pendiente')
           RETURNING id",
    )
    .bind(user_id)
    .bind(operacion)
    .bind(datos)
    .fetch_one(pool)
    .await
    .expect("insert_audit_for_test failed")
}

/// Crea un artículo en `bdp_article_map` para tests de restauración.
async fn create_test_article_map(
    pool: &PgPool,
    user_id: Uuid,
    codigo_glory: &str,
    codigo_bdp: &str,
) -> Uuid {
    let id: Uuid = sqlx::query_scalar(
        r"INSERT INTO bdp_article_map
        (user_id, articulo_glory_codigo, articulo_bdp_codigo, articulo_bdp_nombre,
         descripcion, precio_tarifa1, iva_pct, activo)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id",
    )
    .bind(user_id)
    .bind(codigo_glory)
    .bind(codigo_bdp)
    .bind(format!("Articulo {codigo_bdp}"))
    .bind(format!("Descripción {codigo_glory}"))
    .bind(rust_decimal::Decimal::from(1000))
    .bind(rust_decimal::Decimal::from(21))
    .bind(true)
    .fetch_one(pool)
    .await
    .expect("create_test_article_map failed");
    id
}

/// Crea un cliente para tests de restauración.
async fn create_test_cliente(pool: &PgPool, user_id: Uuid, nombre: &str) -> Uuid {
    let id: Uuid = sqlx::query_scalar(
        r"INSERT INTO clientes (user_id, nombre, email)
        VALUES ($1, $2, $3)
        RETURNING id",
    )
    .bind(user_id)
    .bind(nombre)
    .bind(format!("{nombre}@test.com"))
    .fetch_one(pool)
    .await
    .expect("create_test_cliente failed");
    id
}

/* ========== Snapshot Glory ========== */

#[sqlx::test(migrations = "./migrations")]
async fn test_snapshot_glory_mapeos(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_test_config(&pool, user_id).await;
    create_test_article_map(&pool, user_id, "CAFE001", "1001").await;
    create_test_article_map(&pool, user_id, "CERVEZA01", "2001").await;

    let snap = BdpBackupService::snapshot_glory(
        &pool,
        user_id,
        &["mapeos".to_string()],
        Some("test mapeos".into()),
    )
    .await
    .expect("snapshot_glory should succeed");

    assert_eq!(snap.direccion, "glory");
    assert_eq!(snap.tipo, "glory_mapeos");
    assert_eq!(snap.user_id, user_id);
    assert_eq!(snap.notas.as_deref(), Some("test mapeos"));

    let mapeos = snap.datos["mapeos"]
        .as_array()
        .expect("mapeos should be array");
    assert_eq!(mapeos.len(), 2, "should have 2 mapeos");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_snapshot_glory_clientes(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_test_config(&pool, user_id).await;
    create_test_cliente(&pool, user_id, "Juan").await;
    create_test_cliente(&pool, user_id, "Maria").await;

    let snap = BdpBackupService::snapshot_glory(&pool, user_id, &["clientes".to_string()], None)
        .await
        .expect("snapshot_glory clientes should succeed");

    assert_eq!(snap.tipo, "glory_clientes");
    let clientes = snap.datos["clientes"]
        .as_array()
        .expect("clientes should be array");
    assert_eq!(clientes.len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_snapshot_glory_empty_table(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_test_config(&pool, user_id).await;

    let snap = BdpBackupService::snapshot_glory(&pool, user_id, &["mapeos".to_string()], None)
        .await
        .expect("snapshot on empty table should succeed");

    let mapeos = snap.datos["mapeos"]
        .as_array()
        .expect("mapeos should be array");
    assert_eq!(mapeos.len(), 0, "empty table → empty array");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_snapshot_glory_multiple_tipos(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_test_config(&pool, user_id).await;
    create_test_article_map(&pool, user_id, "CAFE001", "1001").await;
    create_test_cliente(&pool, user_id, "Juan").await;

    let snap = BdpBackupService::snapshot_glory(
        &pool,
        user_id,
        &["mapeos".to_string(), "clientes".to_string()],
        Some("multi".into()),
    )
    .await
    .expect("multi-tipo snapshot should succeed");

    assert_eq!(snap.tipo, "glory_mapeos_clientes");
    assert!(snap.datos["mapeos"].as_array().is_some());
    assert!(snap.datos["clientes"].as_array().is_some());
}

/* ========== CRUD Snapshots ========== */

#[sqlx::test(migrations = "./migrations")]
async fn test_listar_snapshots_vacio(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    let snapshots = BdpBackupService::listar_snapshots(&pool, user_id, 10)
        .await
        .expect("listar should succeed");

    assert_eq!(snapshots.len(), 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_listar_snapshots_despues_de_crear(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_test_config(&pool, user_id).await;
    create_test_article_map(&pool, user_id, "CAFE001", "1001").await;

    BdpBackupService::snapshot_glory(
        &pool,
        user_id,
        &["mapeos".to_string()],
        Some("primero".into()),
    )
    .await
    .unwrap();
    BdpBackupService::snapshot_glory(
        &pool,
        user_id,
        &["mapeos".to_string()],
        Some("segundo".into()),
    )
    .await
    .unwrap();

    let snapshots = BdpBackupService::listar_snapshots(&pool, user_id, 10)
        .await
        .expect("listar should succeed");

    assert_eq!(snapshots.len(), 2);
    /* Orden: más reciente primero */
    assert_eq!(snapshots[0].notas.as_deref(), Some("segundo"));
    assert_eq!(snapshots[1].notas.as_deref(), Some("primero"));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_listar_snapshots_aisla_usuarios(pool: PgPool) {
    let user_a = create_test_user(&pool).await;
    let user_b = create_test_user(&pool).await;
    create_test_config(&pool, user_a).await;
    create_test_config(&pool, user_b).await;
    create_test_article_map(&pool, user_a, "CAFE001", "1001").await;
    create_test_article_map(&pool, user_b, "CERVEZA01", "2001").await;

    BdpBackupService::snapshot_glory(&pool, user_a, &["mapeos".to_string()], None)
        .await
        .unwrap();
    BdpBackupService::snapshot_glory(&pool, user_b, &["mapeos".to_string()], None)
        .await
        .unwrap();

    let snap_a = BdpBackupService::listar_snapshots(&pool, user_a, 10)
        .await
        .unwrap();
    let snap_b = BdpBackupService::listar_snapshots(&pool, user_b, 10)
        .await
        .unwrap();

    assert_eq!(snap_a.len(), 1);
    assert_eq!(snap_b.len(), 1);
    assert_ne!(snap_a[0].id, snap_b[0].id);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_obtener_snapshot(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_test_config(&pool, user_id).await;
    create_test_article_map(&pool, user_id, "CAFE001", "1001").await;

    let created = BdpBackupService::snapshot_glory(
        &pool,
        user_id,
        &["mapeos".to_string()],
        Some("test".into()),
    )
    .await
    .unwrap();

    let fetched = BdpBackupService::obtener_snapshot(&pool, created.id)
        .await
        .expect("obtener should succeed")
        .expect("snapshot should exist");

    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.tipo, "glory_mapeos");
    assert_eq!(fetched.notas.as_deref(), Some("test"));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_obtener_snapshot_inexistente(pool: PgPool) {
    let result = BdpBackupService::obtener_snapshot(&pool, Uuid::new_v4())
        .await
        .expect("should not error");

    assert!(result.is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_eliminar_snapshot(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_test_config(&pool, user_id).await;
    create_test_article_map(&pool, user_id, "CAFE001", "1001").await;

    let created = BdpBackupService::snapshot_glory(&pool, user_id, &["mapeos".to_string()], None)
        .await
        .unwrap();

    let deleted = BdpBackupService::eliminar_snapshot(&pool, created.id, user_id)
        .await
        .expect("eliminar should succeed");
    assert!(deleted);

    let fetched = BdpBackupService::obtener_snapshot(&pool, created.id)
        .await
        .unwrap();
    assert!(fetched.is_none(), "snapshot should be gone");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_eliminar_snapshot_wrong_user(pool: PgPool) {
    let user_a = create_test_user(&pool).await;
    let user_b = create_test_user(&pool).await;
    create_test_config(&pool, user_a).await;
    create_test_article_map(&pool, user_a, "CAFE001", "1001").await;

    let created = BdpBackupService::snapshot_glory(&pool, user_a, &["mapeos".to_string()], None)
        .await
        .unwrap();

    let deleted = BdpBackupService::eliminar_snapshot(&pool, created.id, user_b)
        .await
        .expect("should not error");
    assert!(!deleted, "wrong user should not delete");

    let still_exists = BdpBackupService::obtener_snapshot(&pool, created.id)
        .await
        .unwrap();
    assert!(still_exists.is_some(), "snapshot should still exist");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_listar_snapshots_limit(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_test_config(&pool, user_id).await;
    create_test_article_map(&pool, user_id, "CAFE001", "1001").await;

    for i in 0..5 {
        BdpBackupService::snapshot_glory(
            &pool,
            user_id,
            &["mapeos".to_string()],
            Some(format!("snap-{i}")),
        )
        .await
        .unwrap();
    }

    let all = BdpBackupService::listar_snapshots(&pool, user_id, 100)
        .await
        .unwrap();
    assert_eq!(all.len(), 5);

    let limited = BdpBackupService::listar_snapshots(&pool, user_id, 2)
        .await
        .unwrap();
    assert_eq!(limited.len(), 2);
}

/* ========== Audit Log ========== */

#[sqlx::test(migrations = "./migrations")]
async fn test_snapshot_parcial_rechaza_tipo_desconocido_sin_http(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_test_config(&pool, user_id).await;
    let config = glory_backend::models::ConfiguracionRestaurante {
        user_id,
        bdp_base_url: "http://192.0.2.10:8068".into(),
        ..Default::default()
    };

    let result = BdpBackupService::snapshot_bdp_parcial(
        &pool,
        user_id,
        &config,
        &["desconocido".to_string()],
        None,
    )
    .await;

    assert!(result.is_err());
    assert!(BdpBackupService::listar_snapshots(&pool, user_id, 10)
        .await
        .unwrap()
        .is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_snapshot_pago_sin_order_id_falla_antes_de_http(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_test_config(&pool, user_id).await;
    let config = glory_backend::models::ConfiguracionRestaurante {
        user_id,
        bdp_base_url: "http://192.0.2.10:8068".into(),
        bdp_auto_backup_before_write: true,
        bdp_env_bootstrap_applied_at: None,
        ..Default::default()
    };

    let result =
        BdpBackupService::preparar_snapshot_escritura(&pool, user_id, "add_payment", &config, None)
            .await;

    assert!(result.is_err());
    assert!(BdpBackupService::listar_snapshots(&pool, user_id, 10)
        .await
        .unwrap()
        .is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_preparar_snapshot_rechaza_auto_backup_off(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    /* Config con auto_backup = false */
    sqlx::query(
        r"INSERT INTO configuracion_restaurante (user_id, nombre_restaurante, bdp_auto_backup_before_write)
        VALUES ($1, 'Test', false)",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .unwrap();

    let config = glory_backend::models::ConfiguracionRestaurante {
        user_id,
        bdp_auto_backup_before_write: false,
        bdp_env_bootstrap_applied_at: None,
        ..Default::default()
    };

    let result = BdpBackupService::preparar_snapshot_escritura(
        &pool,
        user_id,
        "create_order",
        &config,
        None,
    )
    .await;

    assert!(
        result.is_err(),
        "auto_backup off debe bloquear la escritura"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn test_preparar_snapshot_create_no_hace_lectura_remota(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    sqlx::query(
        r"INSERT INTO configuracion_restaurante (user_id, nombre_restaurante, bdp_auto_backup_before_write)
        VALUES ($1, 'Test', true)",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .unwrap();

    let config = glory_backend::models::ConfiguracionRestaurante {
        user_id,
        bdp_auto_backup_before_write: true,
        bdp_env_bootstrap_applied_at: None,
        ..Default::default()
    };

    let result = BdpBackupService::preparar_snapshot_escritura(
        &pool,
        user_id,
        "create_customer",
        &config,
        None,
    )
    .await
    .expect("should succeed");

    assert!(result.is_none(), "create customer no necesita GetOrder");
    assert!(BdpBackupService::listar_audit(&pool, user_id, 10)
        .await
        .unwrap()
        .is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_actualizar_resultado(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    sqlx::query(
        r"INSERT INTO configuracion_restaurante (user_id, nombre_restaurante, bdp_auto_backup_before_write)
        VALUES ($1, 'Test', true)",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .unwrap();

    let entry_id = insert_audit_for_test(
        &pool,
        user_id,
        "create_order",
        serde_json::json!({"total": 500}),
    )
    .await;

    /* Actualizar a ok */
    BdpBackupService::actualizar_resultado(
        &pool,
        entry_id,
        "ok",
        Some(&serde_json::json!({"order_id": 999})),
        None,
    )
    .await
    .expect("update should succeed");

    let audit = BdpBackupService::listar_audit(&pool, user_id, 10)
        .await
        .unwrap();
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0].resultado, "ok");
    assert!(audit[0].datos_respuesta.is_some());
    assert!(audit[0].error_mensaje.is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_actualizar_resultado_con_error(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    sqlx::query(
        r"INSERT INTO configuracion_restaurante (user_id, nombre_restaurante, bdp_auto_backup_before_write)
        VALUES ($1, 'Test', true)",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .unwrap();

    let entry_id = insert_audit_for_test(
        &pool,
        user_id,
        "add_payment",
        serde_json::json!({"amount": 100}),
    )
    .await;

    BdpBackupService::actualizar_resultado(
        &pool,
        entry_id,
        "error",
        None,
        Some("BDP connection timeout"),
    )
    .await
    .unwrap();

    let audit = BdpBackupService::listar_audit(&pool, user_id, 10)
        .await
        .unwrap();
    assert_eq!(audit[0].resultado, "error");
    assert_eq!(
        audit[0].error_mensaje.as_deref(),
        Some("BDP connection timeout")
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn test_listar_audit_vacio(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    let entries = BdpBackupService::listar_audit(&pool, user_id, 10)
        .await
        .expect("listar should succeed");

    assert_eq!(entries.len(), 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_listar_audit_aisla_usuarios(pool: PgPool) {
    let user_a = create_test_user(&pool).await;
    let user_b = create_test_user(&pool).await;

    for uid in [user_a, user_b] {
        sqlx::query(
            r"INSERT INTO configuracion_restaurante (user_id, nombre_restaurante, bdp_auto_backup_before_write)
            VALUES ($1, 'Test', true)",
        )
        .bind(uid)
        .execute(&pool)
        .await
        .unwrap();
    }

    insert_audit_for_test(&pool, user_a, "create_order", serde_json::json!({})).await;
    insert_audit_for_test(&pool, user_b, "invoice", serde_json::json!({})).await;
    insert_audit_for_test(&pool, user_b, "create_customer", serde_json::json!({})).await;

    let audit_a = BdpBackupService::listar_audit(&pool, user_a, 100)
        .await
        .unwrap();
    let audit_b = BdpBackupService::listar_audit(&pool, user_b, 100)
        .await
        .unwrap();

    assert_eq!(audit_a.len(), 1, "user_a should have 1 entry");
    assert_eq!(audit_b.len(), 2, "user_b should have 2 entries");
    assert_eq!(audit_a[0].operacion, "create_order");
}

/* ========== Restauración ========== */

#[sqlx::test(migrations = "./migrations")]
async fn test_restaurar_glory_mapeos(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_test_config(&pool, user_id).await;

    /* Crear artículo con precio original */
    let art_id = create_test_article_map(&pool, user_id, "CAFE001", "1001").await;

    /* Crear snapshot con precio diferente (simula backup anterior) */
    let snap_datos = serde_json::json!({
        "mapeos": [
            {
                "id": art_id.to_string(),
                "descripcion": "Descripción ORIGINAL",
                "precio_tarifa1": 500.0,
                "iva_pct": 10.0,
                "activo": true
            }
        ],
        "clientes": []
    });

    let snap =
        insert_snapshot_for_test(&pool, user_id, "glory_test", "glory", "manual", snap_datos).await;

    /* Restaurar */
    let result = BdpBackupService::restaurar_glory(&pool, snap.id, user_id)
        .await
        .expect("restore should succeed");

    assert_eq!(result.registros_restaurados, 1, "1 mapeo restored");
    assert_eq!(result.errores, 0);

    /* Verificar que el artículo fue actualizado */
    let row: (String, rust_decimal::Decimal) =
        sqlx::query_as("SELECT descripcion, precio_tarifa1 FROM bdp_article_map WHERE id = $1")
            .bind(art_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(row.0, "Descripción ORIGINAL");
    assert_eq!(row.1, rust_decimal::Decimal::from(500));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_restaurar_glory_clientes(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_test_config(&pool, user_id).await;

    let cliente_id = create_test_cliente(&pool, user_id, "Juan").await;

    /* Snapshot con bdp_customer_code */
    let snap_datos = serde_json::json!({
        "mapeos": [],
        "clientes": [
            {
                "id": cliente_id.to_string(),
                "bdp_customer_code": 42
            }
        ]
    });

    let snap =
        insert_snapshot_for_test(&pool, user_id, "glory_test", "glory", "manual", snap_datos).await;

    let result = BdpBackupService::restaurar_glory(&pool, snap.id, user_id)
        .await
        .unwrap();

    assert_eq!(result.registros_restaurados, 1);

    let code: Option<i32> =
        sqlx::query_scalar("SELECT bdp_customer_code FROM clientes WHERE id = $1")
            .bind(cliente_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(code, Some(42));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_restaurar_glory_snapshot_no_encontrado(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    let result = BdpBackupService::restaurar_glory(&pool, Uuid::new_v4(), user_id).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no encontrado"));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_restaurar_glory_wrong_user(pool: PgPool) {
    let user_a = create_test_user(&pool).await;
    let user_b = create_test_user(&pool).await;
    create_test_config(&pool, user_a).await;
    create_test_article_map(&pool, user_a, "CAFE001", "1001").await;

    let snap = BdpBackupService::snapshot_glory(&pool, user_a, &["mapeos".to_string()], None)
        .await
        .unwrap();

    let result = BdpBackupService::restaurar_glory(&pool, snap.id, user_b).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No autorizado"));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_restaurar_glory_rejects_bdp_snapshot(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_test_config(&pool, user_id).await;

    /* Crear snapshot de tipo BDP (no glory) */
    let snap = insert_snapshot_for_test(
        &pool,
        user_id,
        "completo",
        "bdp",
        "manual",
        serde_json::json!({"articulos": [], "clientes": []}),
    )
    .await;

    let result = BdpBackupService::restaurar_glory(&pool, snap.id, user_id).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("Solo se pueden restaurar snapshots de Glory"));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_restaurar_glory_mapeo_inexistente(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_test_config(&pool, user_id).await;

    /* Snapshot con UUID que no existe en bdp_article_map */
    let fake_id = Uuid::new_v4();
    let snap_datos = serde_json::json!({
        "mapeos": [
            {
                "id": fake_id.to_string(),
                "descripcion": "No existe",
                "precio_tarifa1": 999.0,
                "iva_pct": 21.0,
                "activo": true
            }
        ],
        "clientes": []
    });

    let snap =
        insert_snapshot_for_test(&pool, user_id, "glory_test", "glory", "manual", snap_datos).await;

    let result = BdpBackupService::restaurar_glory(&pool, snap.id, user_id)
        .await
        .unwrap();

    assert_eq!(result.registros_restaurados, 0);
    assert_eq!(result.errores, 1);
    assert!(result.detalles.contains("no encontrado"));
}

/* ========== Limpieza expirados ========== */

#[sqlx::test(migrations = "./migrations")]
async fn test_limpiar_expirados(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_test_config(&pool, user_id).await;

    /* Insertar snapshot expirado directamente */
    sqlx::query(
        r"INSERT INTO bdp_snapshots (user_id, tipo, direccion, trigger_tipo, datos, expires_at)
        VALUES ($1, 'test', 'glory', 'manual', '{}', NOW() - INTERVAL '1 day')",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .unwrap();

    /* Insertar snapshot sin expiración */
    sqlx::query(
        r"INSERT INTO bdp_snapshots (user_id, tipo, direccion, trigger_tipo, datos, expires_at)
        VALUES ($1, 'test', 'glory', 'manual', '{}', NULL)",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .unwrap();

    /* Insertar snapshot con expiración futura */
    sqlx::query(
        r"INSERT INTO bdp_snapshots (user_id, tipo, direccion, trigger_tipo, datos, expires_at)
        VALUES ($1, 'test', 'glory', 'manual', '{}', NOW() + INTERVAL '30 days')",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .unwrap();

    let deleted = BdpBackupService::limpiar_expirados(&pool)
        .await
        .expect("cleanup should succeed");

    assert_eq!(deleted, 1, "only the expired one should be deleted");

    let remaining = BdpBackupService::listar_snapshots(&pool, user_id, 100)
        .await
        .unwrap();
    assert_eq!(remaining.len(), 2, "2 non-expired should remain");
}
