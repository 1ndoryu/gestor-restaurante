// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
/* [198A-1/F1] Flush de la cola Glory -> BDP contra un mock: standalone no envía,
 * modo BDP envía y marca sincronizada, enriquecimiento de modificar artículo y
 * suscripción inactiva. */
use glory_backend::models::ActualizarConfiguracionRequest;
use glory_backend::services::bdp_push::{
    payload_cancelar, BdpPushService, DOMINIO_ARTICULO, DOMINIO_VENTA, ESTADO_PENDIENTE,
    ESTADO_PENDIENTE_SUSCRIPCION, OPERACION_CANCELAR, OPERACION_CREAR, OPERACION_MODIFICAR,
};
use glory_backend::services::{BdpBackupService, BdpPushFlushService, ConfiguracionService};
use sqlx::PgPool;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/* ===== [198A-1/F1] Invariante central: standalone no envía nada ===== */

#[sqlx::test(migrations = "./migrations")]
async fn flush_en_standalone_no_envia_ni_consume_la_cola(pool: PgPool) {
    /* Invariante central del plan: en modo standalone el worker de flush es un
     * no-op — no envía nada a BDP aunque haya credenciales válidas y una fila
     * pendiente, y deja la cola intacta (independencia total). */
    let server = MockServer::start().await;
    let user = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(user)
        .bind(format!("test-{user}@example.com"))
        .bind("argon2_hash_placeholder")
        .execute(&pool)
        .await
        .unwrap();

    /* Credenciales + sync activos pero modo standalone: si el worker intentara
     * enviar, golpearía `server`. El modo standalone debe bloquearlo antes. */
    ConfiguracionService::actualizar(
        &pool,
        user,
        &ActualizarConfiguracionRequest {
            modo_operacion: Some("standalone".to_string()),
            bdp_base_url: Some(server.uri()),
            bdp_sync_enabled: Some(true),
            bdp_login: Some("u".to_string()),
            bdp_password: Some("p".to_string()),
            bdp_integrator_code: Some("i".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("config standalone con credenciales");

    let payload = serde_json::json!({ "ArtCode": 90000123 });
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
        .expect("flush standalone no debe fallar");

    /* No se envía nada: cero sincronizados y cero requests hacia BDP. */
    assert_eq!(resumen.sincronizados, 0);
    assert!(resumen.omitidos_standalone > 0);
    assert!(server.received_requests().await.unwrap().is_empty());

    /* La fila pendiente queda intacta: sigue activa, sin reintentos. */
    let pendientes = BdpPushService::listar_pendientes(&pool, user)
        .await
        .unwrap();
    assert_eq!(pendientes.len(), 1);
    assert_eq!(pendientes[0].estado, ESTADO_PENDIENTE);
    assert_eq!(pendientes[0].reintentos, 0);
}

/* ===== [198A-1/F1] Camino feliz end-to-end: modo BDP envía y sincroniza ===== */

#[sqlx::test(migrations = "./migrations")]
async fn flush_en_modo_bdp_envia_y_marca_sincronizada(pool: PgPool) {
    /* Camino feliz end-to-end: en modo BDP, una fila encolada se despacha por
     * HTTP al destino correcto (método, path y payload) y queda sincronizada. */
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
        .and(path("/API/Articles/CreateAndUpdateProfiles"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
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
        "ArticleData": { "ArtCode": 90000123, "ArtDescription": "Plato" },
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
        .expect("flush en modo BDP no debe fallar");

    assert_eq!(resumen.sincronizados, 1);
    assert_eq!(resumen.errores, 0);

    /* La fila ya no está activa (salió de la cola al sincronizarse). */
    assert!(BdpPushService::listar_pendientes(&pool, user)
        .await
        .unwrap()
        .is_empty());

    /* Verificar la petición real: método, path y payload. */
    let requests = server.received_requests().await.unwrap();
    let create = requests
        .iter()
        .find(|r| r.url.path() == "/API/Articles/CreateAndUpdateProfiles")
        .expect("debe existir la petición de creación de artículo");
    assert_eq!(create.method.as_str(), "POST");
    let body: serde_json::Value = create.body_json().expect("body JSON válido");
    assert_eq!(body["AutomaticCode"], false);
    assert_eq!(body["AllProfiles"], true);
    assert_eq!(body["ArticleData"]["ArtCode"], 90000123);
    assert_eq!(body["ArticleData"]["ArtDescription"], "Plato");
}

/* [049A-1/Q2.2] ModifyArticle debe enriquecer el ArticleData parcial que ya
 * estaba en la cola. El contrato del BDP exige el objeto completo: GetArticle
 * aporta los campos no editados y el patch local solo sobrescribe los suyos.
 * La prueba comprueba además que el payload auditado y el enviado coinciden
 * indirectamente mediante el éxito del mismo flush y el body HTTP observado. */
#[sqlx::test(migrations = "./migrations")]
async fn flush_modificar_articulo_enriquece_payload_con_get_article(pool: PgPool) {
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
        .and(path("/API/Articles/Get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "ArticleData": {
                "ArtCode": 90000123,
                "ArtDescription": "Original BDP",
                "DeptCode": 7,
                "Price5": 42.5,
                "WebArticle": true
            }
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/API/Articles/ModifyAndUpdateProfiles"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": ""
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
        "ArticleData": {
            "ArtCode": 90000123,
            "ArtDescription": "Editado localmente"
        },
        "AllProfiles": false,
        "ProfilesList": [{ "Profile": 1 }]
    });
    BdpPushService::encolar(
        &pool,
        user,
        DOMINIO_ARTICULO,
        "90000123",
        OPERACION_MODIFICAR,
        &payload,
    )
    .await
    .unwrap();

    let resumen = BdpPushFlushService::flush(&pool, user, true)
        .await
        .expect("flush de modificación no debe fallar");
    assert_eq!(resumen.sincronizados, 1);
    assert_eq!(resumen.errores, 0);

    let requests = server.received_requests().await.unwrap();
    let get = requests
        .iter()
        .find(|r| r.url.path() == "/API/Articles/Get")
        .expect("debe consultar GetArticle antes de modificar");
    let get_body: serde_json::Value = get.body_json().expect("body GetArticle válido");
    assert_eq!(get_body["ArtCode"], 90000123);

    let modify = requests
        .iter()
        .find(|r| r.url.path() == "/API/Articles/ModifyAndUpdateProfiles")
        .expect("debe enviar ModifyAndUpdateProfiles");
    let modify_body: serde_json::Value = modify.body_json().expect("body Modify válido");
    assert_eq!(
        modify_body["ArticleData"]["ArtDescription"],
        "Editado localmente"
    );
    assert_eq!(modify_body["ArticleData"]["Price5"], 42.5);
    assert_eq!(modify_body["ArticleData"]["DeptCode"], 7);
    assert_eq!(modify_body["AllProfiles"], false);
    assert_eq!(modify_body["ProfilesList"][0]["Profile"], 1);
}

/* [049A-1/Q2.2] Una lectura previa fallida no debe dejar un armado huérfano:
 * `armar_push` ocurre antes de GetArticle, pero la compensación devuelve el
 * modo a solo lectura y permite que la fila se reintente después. */
#[sqlx::test(migrations = "./migrations")]
async fn flush_modificar_articulo_fallo_get_cancela_armado(pool: PgPool) {
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
        .and(path("/API/Articles/Get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "ArticleData": null
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

    BdpPushService::encolar(
        &pool,
        user,
        DOMINIO_ARTICULO,
        "90000123",
        OPERACION_MODIFICAR,
        &serde_json::json!({
            "ArticleData": {
                "ArtCode": 90000123,
                "ArtDescription": "Editado localmente"
            }
        }),
    )
    .await
    .unwrap();

    let resumen = BdpPushFlushService::flush(&pool, user, true)
        .await
        .expect("el fallo de GetArticle debe clasificarse en el flush");
    assert_eq!(resumen.procesados, 1);
    assert_eq!(resumen.sincronizados, 0);
    assert_eq!(resumen.errores, 1);

    let armados: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM bdp_write_arming WHERE user_id = $1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(armados, 0, "no debe quedar un armado huérfano");

    let modo: String = sqlx::query_scalar(
        "SELECT bdp_sync_mode FROM configuracion_restaurante WHERE user_id = $1",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(modo, "read_only");
}

/* [049A-1/S3] Suscripción inactiva end-to-end: una fila de `venta/cancelar`
 * encolada se despacha por flush contra un BDP que responde "Subscripción no
 * activada" → la fila queda `pendiente_suscripcion` SIN consumir reintentos
 * (D2) y con el error honesto en `ultimo_error`; el siguiente flush automático
 * no la toca (solo reintento manual). Cero daño: el resumen no lo cuenta como
 * error transitorio ni reintenta en bucle. */
#[sqlx::test(migrations = "./migrations")]
async fn flush_suscripcion_inactiva_marca_pendiente_sin_reintentos(pool: PgPool) {
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
        .and(path("/API/Orders/Cancel"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "Subscripción no activada"
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

    /* Q2.11 — cancelación de una comanda con bdp_order_id conocido. */
    let payload = payload_cancelar(&config, 5330).unwrap();
    BdpPushService::encolar(
        &pool,
        user,
        DOMINIO_VENTA,
        "5330",
        OPERACION_CANCELAR,
        &payload,
    )
    .await
    .unwrap();

    let resumen = BdpPushFlushService::flush(&pool, user, true)
        .await
        .expect("flush con suscripción inactiva no debe fallar");
    assert_eq!(resumen.procesados, 1);
    assert_eq!(resumen.pendientes_suscripcion, 1);
    assert_eq!(resumen.sincronizados, 0);
    assert_eq!(resumen.errores, 0);

    /* Registro local honesto: estado `pendiente_suscripcion`, cero reintentos
     * consumidos (D2) y error visible para el usuario. */
    let (estado, reintentos, ultimo_error): (String, i32, Option<String>) = sqlx::query_as(
        "SELECT estado, reintentos, ultimo_error FROM bdp_push_pendientes \
             WHERE user_id = $1 AND dominio = 'venta' AND entidad_id = '5330'",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(estado, ESTADO_PENDIENTE_SUSCRIPCION);
    assert_eq!(reintentos, 0);
    assert!(ultimo_error.unwrap_or_default().contains("no activada"));

    /* Segundo flush automático (sin forzar): la fila no se toca (solo manual). */
    let resumen2 = BdpPushFlushService::flush(&pool, user, false)
        .await
        .unwrap();
    assert_eq!(resumen2.procesados, 0);
    assert_eq!(resumen2.pendientes_suscripcion, 1);
    assert_eq!(resumen2.errores, 0);
}
