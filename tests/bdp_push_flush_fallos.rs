// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
/* [049A-1/S4 y H-W-2] Fallos del flush contra BDP: timeout a mitad de escritura
 * (estado ambiguo, reintento acotado) y payload inválido (rechazo sin reintento). */
use glory_backend::models::ActualizarConfiguracionRequest;
use glory_backend::services::bdp_push::{
    BdpPushService, ClavePendiente, ResultadoPendiente, DOMINIO_ARTICULO, ESTADO_ERROR,
    ESTADO_PENDIENTE, ESTADO_RECHAZADO, OPERACION_CREAR, REINTENTOS_MAX,
};
use glory_backend::services::{BdpBackupService, BdpPushFlushService, ConfiguracionService};
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/* [049A-1/S4] Timeout a mitad de escritura (respuesta >20 s): la fila de cola
 * se clasifica honestamente (estado `error` transitorio, reintentos +1,
 * `ultimo_error` visible), la auditoría registra el intento como `ambiguo`
 * (resultado desconocido: pudo haberse aplicado) y el flush NO reenvía dentro
 * de la misma pasada (una sola llamada HTTP). El reintento queda acotado a
 * REINTENTOS_MAX: al agotarlo el flush deja de enviar (error documentado,
 * nunca bucle infinito). H-W-2 (crash a mitad de HTTP): en la cola no hay
 * reconciliación automática — el límite + reintento manual + auditoría
 * ambiguo es el runbook; la reconciliación automática real vive en el poller
 * de `create_order` (simulator_reconcile_after_disconnect). */
#[sqlx::test(migrations = "./migrations")]
async fn flush_timeout_mid_write_estado_ambiguo_y_reintento_acotado(pool: PgPool) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Auth/Login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "AuthSession": { "Token": "token-bdp", "ExpiresIn_InSecconds": 3540 }
        })))
        .mount(&server)
        .await;
    /* Escritura que nunca responde a tiempo (25 s > timeout 20 s del cliente). */
    let create_mock = Mock::given(method("POST"))
        .and(path("/API/Articles/CreateAndUpdateProfiles"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({}))
                .set_delay(Duration::from_secs(25)),
        );
    create_mock.mount(&server).await;

    let user = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(user)
        .bind(format!("test-{user}@example.com"))
        .bind("argon2_hash_placeholder")
        .execute(&pool)
        .await
        .unwrap();

    let config = ConfiguracionService::actualizar(
        &pool,
        user,
        &ActualizarConfiguracionRequest {
            modo_operacion: Some("bdp".to_string()),
            bdp_base_url: Some(server.uri()),
            bdp_sync_enabled: Some(true),
            bdp_auto_backup_before_write: Some(true),
            bdp_login: Some("usuario".to_string()),
            bdp_password: Some("secreto".to_string()),
            bdp_integrator_code: Some("INTEGRADOR".to_string()),
            bdp_pos_id: Some(1),
            bdp_employee_id: Some(1),
            bdp_items_profile_id: Some(1),
            ..Default::default()
        },
    )
    .await
    .expect("config modo bdp");

    /* Snapshot completo vigente: prerequisito del arming (auto_arm_inner). */
    let target = BdpBackupService::canonical_target(&config).unwrap();
    let fingerprint = BdpBackupService::connection_fingerprint(&config).unwrap();
    sqlx::query(
        "INSERT INTO bdp_snapshots \
         (user_id, tipo, direccion, trigger_tipo, datos, target_base_url, connection_fingerprint) \
         VALUES ($1, 'completo', 'bdp', 'manual', '{}'::jsonb, $2, $3)",
    )
    .bind(user)
    .bind(&target)
    .bind(&fingerprint)
    .execute(&pool)
    .await
    .unwrap();

    let payload = serde_json::json!({
        "AutomaticCode": false,
        "ArticleData": { "ArtCode": 90_000_123, "ArtDescription": "Plato" },
        "AllProfiles": true
    });
    BdpPushService::encolar(
        &pool,
        user,
        DOMINIO_ARTICULO,
        "90000123",
        OPERACION_CREAR,
        &payload,
    )
    .await
    .unwrap();

    /* 1er flush: el HTTP expira a los 20 s → estado honesto, sin doble envío. */
    let resumen = BdpPushFlushService::flush(&pool, user, true)
        .await
        .expect("flush con timeout no debe fallar");
    assert_eq!(resumen.procesados, 1);
    assert_eq!(resumen.errores, 1);
    assert_eq!(resumen.sincronizados, 0);

    let (estado, reintentos, ultimo_error): (String, i32, Option<String>) = sqlx::query_as(
        "SELECT estado, reintentos, ultimo_error FROM bdp_push_pendientes \
         WHERE user_id = $1 AND dominio = 'articulo' AND entidad_id = '90000123'",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(estado, ESTADO_ERROR);
    assert_eq!(reintentos, 1);
    let msg = ultimo_error.unwrap_or_default();
    assert!(
        msg.contains("timed out") || msg.contains("timeout") || msg.contains("error sending"),
        "Mensaje honesto esperado: {msg}"
    );

    /* Auditoría: el intento queda registrado como ambiguo (pudo aplicarse). */
    let (resultado, error_mensaje): (String, Option<String>) = sqlx::query_as(
        "SELECT resultado, error_mensaje FROM bdp_audit_log \
         WHERE user_id = $1 AND operacion = 'create_article' \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(resultado, "ambiguo");
    /* reqwest muestra el nivel superior; la causa (timeout 20 s) queda en la
     * cadena de error. El hecho de que el flush tardara >20 s ya lo demuestra. */
    let msg_audit = error_mensaje.unwrap_or_default();
    assert!(
        msg_audit.contains("error sending") || msg_audit.contains("timed out"),
        "Mensaje de auditoría honesto esperado: {msg_audit:?}"
    );

    /* Una sola llamada HTTP en esa pasada: sin doble envío. */
    let envios = server
        .received_requests()
        .await
        .iter()
        .flatten()
        .filter(|req| req.url.path() == "/API/Articles/CreateAndUpdateProfiles")
        .count();
    assert_eq!(envios, 1);

    /* Reintento acotado: agotar REINTENTOS_MAX vía `marcar_resultado` (sin
     * HTTP) y verificar que el siguiente flush no vuelve a enviar. */
    for _ in 0..(REINTENTOS_MAX - 1) {
        BdpPushService::marcar_resultado_por_clave(
            &pool,
            ClavePendiente {
                user_id: user,
                dominio: DOMINIO_ARTICULO,
                entidad_id: "90000123",
                operacion: OPERACION_CREAR,
            },
            ResultadoPendiente {
                estado: ESTADO_ERROR,
                error: Some("timeout simulado"),
                incrementar_reintento: true,
            },
        )
        .await
        .unwrap();
    }
    let reintentos_max: i32 =
        sqlx::query_scalar("SELECT reintentos FROM bdp_push_pendientes WHERE user_id = $1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(reintentos_max, REINTENTOS_MAX);

    let resumen_final = BdpPushFlushService::flush(&pool, user, true).await.unwrap();
    assert_eq!(resumen_final.procesados, 0);
    assert_eq!(resumen_final.errores, 1);
    /* Sin nuevos envíos: el límite detiene el reintento (nunca bucle infinito). */
    let envios_finales = server
        .received_requests()
        .await
        .iter()
        .flatten()
        .filter(|req| req.url.path() == "/API/Articles/CreateAndUpdateProfiles")
        .count();
    assert_eq!(envios_finales, 1);
}

/* [049A-1/S5] Payload inválido (HTTP 422 → rechazo definitivo): la fila
 * encolada se despacha por flush contra un BDP que responde 422 → queda
 * `rechazado` SIN consumir reintentos (no es transitorio: reintentarlo en
 * bucle es inútil y enmascararía un dato local roto), `ultimo_error` honesto
 * con el status, auditoría `error` (definitivo, NO `ambiguo`), y el siguiente
 * flush automático no la toca (solo reintento manual). Cero filas fantasma:
 * el rechazo no borra ni reencola la fila ni la marca como sincronizada. */
#[sqlx::test(migrations = "./migrations")]
async fn flush_payload_invalido_marca_rechazado_sin_reintento(pool: PgPool) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Auth/Login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "AuthSession": { "Token": "token-bdp", "ExpiresIn_InSecconds": 3540 }
        })))
        .mount(&server)
        .await;
    /* Rechazo definitivo del BDP: 422 payload inválido (como el 422 real del
     * BDP con "el código ya existe" o el mensaje engañoso de compose). */
    Mock::given(method("POST"))
        .and(path("/API/Articles/CreateAndUpdateProfiles"))
        .respond_with(ResponseTemplate::new(422).set_body_json(serde_json::json!({
            "ErrorMessage": "El código de artículo ya existe"
        })))
        .mount(&server)
        .await;

    let user = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(user)
        .bind(format!("test-{user}@example.com"))
        .bind("argon2_hash_placeholder")
        .execute(&pool)
        .await
        .unwrap();

    let config = ConfiguracionService::actualizar(
        &pool,
        user,
        &ActualizarConfiguracionRequest {
            modo_operacion: Some("bdp".to_string()),
            bdp_base_url: Some(server.uri()),
            bdp_sync_enabled: Some(true),
            bdp_auto_backup_before_write: Some(true),
            bdp_login: Some("usuario".to_string()),
            bdp_password: Some("secreto".to_string()),
            bdp_integrator_code: Some("INTEGRADOR".to_string()),
            bdp_pos_id: Some(1),
            bdp_employee_id: Some(1),
            bdp_items_profile_id: Some(1),
            ..Default::default()
        },
    )
    .await
    .expect("config modo bdp");

    /* Snapshot completo vigente: prerequisito del arming (auto_arm_inner). */
    let target = BdpBackupService::canonical_target(&config).unwrap();
    let fingerprint = BdpBackupService::connection_fingerprint(&config).unwrap();
    sqlx::query(
        "INSERT INTO bdp_snapshots \
         (user_id, tipo, direccion, trigger_tipo, datos, target_base_url, connection_fingerprint) \
         VALUES ($1, 'completo', 'bdp', 'manual', '{}'::jsonb, $2, $3)",
    )
    .bind(user)
    .bind(&target)
    .bind(&fingerprint)
    .execute(&pool)
    .await
    .unwrap();

    let payload = serde_json::json!({
        "AutomaticCode": false,
        "ArticleData": { "ArtCode": 90_000_123, "ArtDescription": "Plato" },
        "AllProfiles": true
    });
    BdpPushService::encolar(
        &pool,
        user,
        DOMINIO_ARTICULO,
        "90000123",
        OPERACION_CREAR,
        &payload,
    )
    .await
    .unwrap();

    let resumen = BdpPushFlushService::flush(&pool, user, true)
        .await
        .expect("flush con 422 no debe fallar");
    assert_eq!(resumen.procesados, 1);
    assert_eq!(resumen.rechazados, 1);
    assert_eq!(resumen.errores, 0);
    assert_eq!(resumen.sincronizados, 0);

    /* Registro local honesto: `rechazado`, cero reintentos consumidos y error
     * visible con el status del BDP. */
    let (estado, reintentos, ultimo_error): (String, i32, Option<String>) = sqlx::query_as(
        "SELECT estado, reintentos, ultimo_error FROM bdp_push_pendientes \
         WHERE user_id = $1 AND dominio = 'articulo' AND entidad_id = '90000123'",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(estado, ESTADO_RECHAZADO);
    assert_eq!(reintentos, 0);
    let msg = ultimo_error.unwrap_or_default();
    assert!(
        msg.contains("422"),
        "Mensaje honesto con el status esperado: {msg}"
    );
    /* [287A-4] El cuerpo del error se redacta a propósito (no filtra el
     * mensaje de negocio que podría reflejar datos); solo status + tamaño. */
    assert!(
        !msg.contains("El código de artículo ya existe"),
        "El cuerpo del 4xx no debe filtrarse: {msg}"
    );

    /* Auditoría: rechazo definitivo → resultado `error`, NO `ambiguo`. */
    let (resultado, error_mensaje): (String, Option<String>) = sqlx::query_as(
        "SELECT resultado, error_mensaje FROM bdp_audit_log \
         WHERE user_id = $1 AND operacion = 'create_article' \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(resultado, "error");
    assert!(error_mensaje.unwrap_or_default().contains("422"));

    /* Segundo flush automático (sin forzar): la fila rechazada NO se toca
     * (solo reintento manual) — nunca reintento en bucle de un rechazo. */
    let resumen2 = BdpPushFlushService::flush(&pool, user, false)
        .await
        .unwrap();
    assert_eq!(resumen2.procesados, 0);
    assert_eq!(resumen2.rechazados, 1);
    assert_eq!(resumen2.errores, 0);

    /* Una sola llamada HTTP al endpoint de escritura en total. */
    let envios = server
        .received_requests()
        .await
        .iter()
        .flatten()
        .filter(|req| req.url.path() == "/API/Articles/CreateAndUpdateProfiles")
        .count();
    assert_eq!(envios, 1);

    /* Re-edición local (encolar de nuevo) refresca la fila rechazada a
     * `pendiente` (M19): el dato corregido sí vuelve a la cola activa. */
    BdpPushService::encolar(
        &pool,
        user,
        DOMINIO_ARTICULO,
        "90000123",
        OPERACION_CREAR,
        &serde_json::json!({ "AutomaticCode": true }),
    )
    .await
    .unwrap();
    let (estado2, reintentos2): (String, i32) = sqlx::query_as(
        "SELECT estado, reintentos FROM bdp_push_pendientes \
         WHERE user_id = $1 AND dominio = 'articulo' AND entidad_id = '90000123'",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(estado2, ESTADO_PENDIENTE);
    assert_eq!(reintentos2, 0);
}
