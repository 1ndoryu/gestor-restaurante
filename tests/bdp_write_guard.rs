// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
//! Pruebas `SQLx` del armado BDP. Solo usa una base temporal local; no hace HTTP.

use glory_backend::repositories::ConfiguracionRepository;
use glory_backend::services::{
    fijar_permiso, AuditoriaDirecta, BdpBackupService, BdpWriteGuard, EscrituraAutorizar,
};
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
        r"INSERT INTO bdp_snapshots
           (user_id, tipo, direccion, trigger_tipo, datos, target_base_url, connection_fingerprint)
           VALUES ($1, 'completo', 'bdp', 'manual', $2, $3, $4)
           RETURNING id",
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
        r"INSERT INTO bdp_write_arming
           (user_id, base_url, scopes, target_entity_type, target_entity_id,
            reason, expires_at, remaining_operations, snapshot_id, connection_fingerprint)
           VALUES ($1, $2, ARRAY['create_order'], 'venta', $3,
                   'prueba local', NOW() + INTERVAL '5 minutes', 1, $4, $5)",
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
        EscrituraAutorizar {
            scope: "create_order",
            target_entity_type: "venta",
            target_entity_id: otra_venta,
            entity_json_field: "venta_id",
            datos_enviados: &serde_json::json!({"venta_id": otra_venta}),
            snapshot_pre_id: None,
            idempotency_key: None,
        },
    )
    .await;
    assert!(denied.is_err());

    let audit_id = BdpWriteGuard::authorize(
        &pool,
        user_id,
        &config,
        EscrituraAutorizar {
            scope: "create_order",
            target_entity_type: "venta",
            target_entity_id: venta_autorizada,
            entity_json_field: "venta_id",
            datos_enviados: &serde_json::json!({"venta_id": venta_autorizada}),
            snapshot_pre_id: None,
            idempotency_key: None,
        },
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
        r"INSERT INTO bdp_write_arming
           (user_id, base_url, scopes, target_entity_type, target_entity_id,
            reason, expires_at, remaining_operations, snapshot_id, connection_fingerprint)
           VALUES ($1, $2, ARRAY['invoice'], 'venta', $3,
                   'prueba bloqueo cruzado', NOW() + INTERVAL '5 minutes', 1, $4, $5)",
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
        EscrituraAutorizar {
            scope: "invoice",
            target_entity_type: "venta",
            target_entity_id: venta_autorizada,
            entity_json_field: "venta_id",
            datos_enviados: &serde_json::json!({"venta_id": venta_autorizada}),
            snapshot_pre_id: None,
            idempotency_key: None,
        },
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
        EscrituraAutorizar {
            scope: "create_order",
            target_entity_type: "venta",
            target_entity_id: venta_autorizada,
            entity_json_field: "venta_id",
            datos_enviados: &serde_json::json!({"venta_id": venta_autorizada}),
            snapshot_pre_id: None,
            idempotency_key: None,
        },
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
        r"UPDATE configuracion_restaurante
           SET bdp_base_url = $2, bdp_login = $3, bdp_password = $4,
               bdp_integrator_code = $5, bdp_sync_mode = 'unidirectional'
           WHERE user_id = $1",
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
        r"INSERT INTO bdp_snapshots
           (user_id, tipo, direccion, trigger_tipo, datos, target_base_url, connection_fingerprint)
           VALUES ($1, 'completo', 'bdp', 'manual', $2, $3, $4)
           RETURNING id",
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
        r"INSERT INTO bdp_write_arming
           (user_id, base_url, scopes, target_entity_type, target_entity_id,
            reason, expires_at, remaining_operations, snapshot_id, connection_fingerprint)
           VALUES ($1, $2, ARRAY['create_order'], 'venta', $3,
                   'prueba cambio conexión', NOW() + INTERVAL '5 minutes', 1, $4, $5)",
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
        EscrituraAutorizar {
            scope: "create_order",
            target_entity_type: "venta",
            target_entity_id: venta_id,
            entity_json_field: "venta_id",
            datos_enviados: &serde_json::json!({"venta_id": venta_id}),
            snapshot_pre_id: None,
            idempotency_key: None,
        },
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

#[sqlx::test(migrations = "./migrations")]
async fn auditoria_directa_call_waiter_registra_resultado(pool: PgPool) {
    /* [049A-1/H-W-1] La auditoría directa (call_waiter) debe dejar una fila en
     * bdp_audit_log con resultado y sin tocar arming ni snapshot. */
    let user_id = create_test_user(&pool).await;
    let mesa_id = Uuid::new_v4();
    let datos = serde_json::json!({ "mesa": 5, "sala": "Principal" });

    BdpBackupService::auditar_escritura_directa(AuditoriaDirecta {
        pool: &pool,
        user_id,
        operacion: "call_waiter",
        target_entity_type: "mesa",
        target_entity_id: mesa_id,
        datos_enviados: &datos,
        resultado: "exito",
        error_mensaje: None,
    })
    .await
    .expect("auditoría directa exitosa");

    let (operacion, resultado, dir, target, entity_id): (String, String, String, String, Uuid) =
        sqlx::query_as(
            "SELECT operacion, resultado, direccion, target_entity_type, target_entity_id \
             FROM bdp_audit_log WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("fila de auditoría");

    assert_eq!(operacion, "call_waiter");
    assert_eq!(resultado, "exito");
    assert_eq!(dir, "glory_to_bdp");
    assert_eq!(target, "mesa");
    assert_eq!(entity_id, mesa_id);

    let arming_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM bdp_write_arming WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .expect("contar arming");
    assert_eq!(arming_count, 0, "la auditoría directa no crea arming");
}

#[sqlx::test(migrations = "./migrations")]
async fn auditoria_directa_registra_error_sin_romper(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let mesa_id = Uuid::new_v4();

    BdpBackupService::auditar_escritura_directa(AuditoriaDirecta {
        pool: &pool,
        user_id,
        operacion: "call_waiter",
        target_entity_type: "mesa",
        target_entity_id: mesa_id,
        datos_enviados: &serde_json::json!({ "mesa": 3, "sala": "Terraza" }),
        resultado: "error",
        error_mensaje: Some("BDP respondio HTTP 500: boom"),
    })
    .await
    .expect("auditoría de error exitosa");

    let (resultado, error_mensaje): (String, Option<String>) =
        sqlx::query_as("SELECT resultado, error_mensaje FROM bdp_audit_log WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .expect("fila de auditoría");

    assert_eq!(resultado, "error");
    assert!(error_mensaje.unwrap().contains("500"));
}

/// [267A-7] El modo automático autoriza sin armado y sin volver a `read_only`.
#[sqlx::test(migrations = "./migrations")]
async fn modo_automatico_autoriza_sin_armado_ni_cierre(pool: PgPool) {
    assert!(BdpWriteGuard::modo_permite_escritura("automatic"));
    assert!(BdpWriteGuard::modo_permite_escritura("unidirectional"));
    assert!(!BdpWriteGuard::modo_permite_escritura("read_only"));

    let user_id = create_test_user(&pool).await;
    let mut config = ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración");
    config.bdp_base_url = "http://127.0.0.1:18765".into();
    config.bdp_sync_mode = "automatic".into();
    sqlx::query(
        "UPDATE configuracion_restaurante SET bdp_base_url = $2, bdp_sync_mode = 'automatic' WHERE user_id = $1",
    )
    .bind(user_id)
    .bind(&config.bdp_base_url)
    .execute(&pool)
    .await
    .expect("configurar BDP local en automático");

    /* [26AA-3] Con el modo automático, cada categoría necesita su permiso. */
    fijar_permiso(&pool, user_id, "comandas", true)
        .await
        .expect("permitir comandas");

    let venta_id = Uuid::new_v4();
    let audit_id = BdpWriteGuard::authorize(
        &pool,
        user_id,
        &config,
        EscrituraAutorizar {
            scope: "create_order",
            target_entity_type: "venta",
            target_entity_id: venta_id,
            entity_json_field: "venta_id",
            datos_enviados: &serde_json::json!({"venta_id": venta_id}),
            snapshot_pre_id: None,
            idempotency_key: None,
        },
    )
    .await
    .expect("el modo automático autoriza sin armado");

    let (resultado, motivo): (String, Option<String>) =
        sqlx::query_as("SELECT resultado, authorization_reason FROM bdp_audit_log WHERE id = $1")
            .bind(audit_id)
            .fetch_one(&pool)
            .await
            .expect("leer auditoría");
    assert_eq!(resultado, "pendiente");
    assert_eq!(motivo.as_deref(), Some("modo_automatico"));

    let mode: String = sqlx::query_scalar(
        "SELECT bdp_sync_mode FROM configuracion_restaurante WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("leer modo");
    assert_eq!(mode, "automatic", "el automático no se cierra tras operar");

    let arming_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM bdp_write_arming WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .expect("contar arming");
    assert_eq!(arming_count, 0, "el automático no crea ni consume arming");
}

/// [26AA-3] El modo automático no escribe una categoría apagada, aunque el
/// interruptor general esté encendido. Otra categoría encendida no la abre.
#[sqlx::test(migrations = "./migrations")]
async fn modo_automatico_bloquea_categoria_apagada(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let mut config = ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración");
    config.bdp_base_url = "http://127.0.0.1:18765".into();
    config.bdp_sync_mode = "automatic".into();
    sqlx::query(
        "UPDATE configuracion_restaurante SET bdp_base_url = $2, bdp_sync_mode = 'automatic' WHERE user_id = $1",
    )
    .bind(user_id)
    .bind(&config.bdp_base_url)
    .execute(&pool)
    .await
    .expect("configurar BDP local en automático");
    fijar_permiso(&pool, user_id, "pagos", true)
        .await
        .expect("permitir pagos");

    let venta_id = Uuid::new_v4();
    let error = BdpWriteGuard::authorize(
        &pool,
        user_id,
        &config,
        EscrituraAutorizar {
            scope: "create_order",
            target_entity_type: "venta",
            target_entity_id: venta_id,
            entity_json_field: "venta_id",
            datos_enviados: &serde_json::json!({"venta_id": venta_id}),
            snapshot_pre_id: None,
            idempotency_key: None,
        },
    )
    .await
    .expect_err("comandas está apagada");
    assert!(
        error.contains("comandas"),
        "el error nombra la categoría: {error}"
    );
}

/// [10AA-3-A] El armado manual vigente (el que se concede por lote con
/// `PUT /configuracion/bdp/sync-mode`, nombrando destino y entidad) autoriza el
/// push en lugar de bloquearlo. Antes, `armar_push` caía siempre en
/// `auto_arm_inner`, que se niega a pisar un armado activo: la fila de la cola
/// no salía nunca y la operación autorizada quedaba muerta.
#[sqlx::test(migrations = "./migrations")]
async fn armar_push_reutiliza_el_armado_manual_vigente(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let mut config = ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración");
    config.bdp_base_url = "http://127.0.0.1:18765".into();
    config.bdp_sync_mode = "unidirectional".into();
    config.bdp_sync_enabled = true;
    config.bdp_login = "prueba".into();
    config.bdp_password = "prueba".into();
    config.bdp_integrator_code = "PRUEBA".into();
    sqlx::query(
        r"UPDATE configuracion_restaurante
             SET bdp_base_url = $2, bdp_sync_mode = 'unidirectional',
                 bdp_sync_enabled = TRUE, bdp_login = 'prueba', bdp_password = 'prueba',
                 bdp_integrator_code = 'PRUEBA'
           WHERE user_id = $1",
    )
    .bind(user_id)
    .bind(&config.bdp_base_url)
    .execute(&pool)
    .await
    .expect("configurar BDP local");

    let fingerprint = BdpBackupService::connection_fingerprint(&config).expect("fingerprint");
    let snapshot_id: Uuid = sqlx::query_scalar(
        r"INSERT INTO bdp_snapshots
           (user_id, tipo, direccion, trigger_tipo, datos, target_base_url, connection_fingerprint)
           VALUES ($1, 'completo', 'bdp', 'manual', $2, $3, $4)
           RETURNING id",
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

    let venta_id = Uuid::new_v4();
    sqlx::query(
        r"INSERT INTO bdp_write_arming
           (user_id, base_url, scopes, target_entity_type, target_entity_id,
            reason, expires_at, remaining_operations, snapshot_id, connection_fingerprint)
           VALUES ($1, $2, ARRAY['cancel_order'], 'venta', $3,
                   'prueba local', NOW() + INTERVAL '10 minutes', 1, $4, $5)",
    )
    .bind(user_id)
    .bind(&config.bdp_base_url)
    .bind(venta_id)
    .bind(snapshot_id)
    .bind(&fingerprint)
    .execute(&pool)
    .await
    .expect("crear armado manual");

    BdpWriteGuard::armar_push(
        &pool,
        user_id,
        &config,
        "cancel_order",
        "venta",
        venta_id,
        true,
    )
    .await
    .expect("el armado manual vigente autoriza el push");

    let (motivo, operaciones): (String, i32) = sqlx::query_as(
        "SELECT reason, remaining_operations FROM bdp_write_arming WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("leer armado");
    assert_eq!(
        motivo, "prueba local",
        "el armado manual se conserva intacto"
    );
    assert_eq!(operaciones, 1, "armar el push no consume el armado");

    /* Otra entidad no lo reutiliza: cae al auto-armado, que se niega a pisar un
     * armado consumible. El fail-closed se mantiene y el armado no se toca. */
    let error = BdpWriteGuard::armar_push(
        &pool,
        user_id,
        &config,
        "cancel_order",
        "venta",
        Uuid::new_v4(),
        true,
    )
    .await
    .expect_err("un armado de otra entidad no autoriza esta operación");
    assert!(error.contains("armado BDP activo"), "{error}");

    let (motivo_final, operaciones_finales): (String, i32) = sqlx::query_as(
        "SELECT reason, remaining_operations FROM bdp_write_arming WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("leer armado tras el rechazo");
    assert_eq!(motivo_final, "prueba local", "el rechazo no pisa el armado");
    assert_eq!(operaciones_finales, 1);
}

/// [D-auto-push] Un armado que `authorize` nunca podrá consumir no puede
/// bloquear la cola entera. Caso real: con `bdp_sync_mode='read_only'` quedó una
/// fila de `transfer_stock` para otra entidad y todo envío automático respondía
/// «Ya existe un armado BDP activo» hasta que vencía, sin botón que lo arreglara.
#[sqlx::test(migrations = "./migrations")]
async fn auto_armar_pisa_un_armado_que_no_se_puede_consumir(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let mut config = ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración");
    config.bdp_base_url = "http://127.0.0.1:18765".into();
    config.bdp_sync_mode = "read_only".into();
    config.bdp_sync_enabled = true;
    config.bdp_login = "prueba".into();
    config.bdp_password = "prueba".into();
    config.bdp_integrator_code = "PRUEBA".into();
    sqlx::query(
        r"UPDATE configuracion_restaurante
             SET bdp_base_url = $2, bdp_sync_mode = 'read_only',
                 bdp_sync_enabled = TRUE, bdp_login = 'prueba', bdp_password = 'prueba',
                 bdp_integrator_code = 'PRUEBA'
           WHERE user_id = $1",
    )
    .bind(user_id)
    .bind(&config.bdp_base_url)
    .execute(&pool)
    .await
    .expect("configurar BDP local en solo lectura");

    let fingerprint = BdpBackupService::connection_fingerprint(&config).expect("fingerprint");
    let snapshot_id: Uuid = sqlx::query_scalar(
        r"INSERT INTO bdp_snapshots
           (user_id, tipo, direccion, trigger_tipo, datos, target_base_url, connection_fingerprint)
           VALUES ($1, 'completo', 'bdp', 'manual', $2, $3, $4)
           RETURNING id",
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

    /* Armado huérfano: otra entidad y modo que `authorize` no consume. */
    sqlx::query(
        r"INSERT INTO bdp_write_arming
           (user_id, base_url, scopes, target_entity_type, target_entity_id,
            reason, expires_at, remaining_operations, snapshot_id, connection_fingerprint)
           VALUES ($1, $2, ARRAY['transfer_stock'], 'stock', $3,
                   'auto_arm:transfer_stock:stock:huerfano', NOW() + INTERVAL '10 minutes', 1, $4, $5)",
    )
    .bind(user_id)
    .bind(&config.bdp_base_url)
    .bind(Uuid::new_v4())
    .bind(snapshot_id)
    .bind(&fingerprint)
    .execute(&pool)
    .await
    .expect("crear armado huérfano");

    BdpWriteGuard::armar_push(
        &pool,
        user_id,
        &config,
        "regularize_stock",
        "stock",
        Uuid::new_v4(),
        true,
    )
    .await
    .expect("un armado no consumible no bloquea el envío");

    let (scopes, motivo): (Vec<String>, String) =
        sqlx::query_as("SELECT scopes, reason FROM bdp_write_arming WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .expect("leer armado reemplazado");
    assert_eq!(
        scopes,
        vec!["regularize_stock".to_string()],
        "el armado huérfano se reemplaza"
    );
    assert!(motivo.starts_with("auto_arm:regularize_stock:"), "{motivo}");
}
