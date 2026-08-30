/* [198A-1/F1] Tests de la cola de push unidireccional Glory -> BDP
 * (bdp_push_pendientes). Verifican: upsert de fila activa (M19), política de
 * reintentos (transitorio vs suscripción, D2), orden por dependencia de
 * dominio (M12) y salida de la cola activa al sincronizar. */
use glory_backend::models::{ActualizarConfiguracionRequest, ConfiguracionRestaurante};
use glory_backend::services::bdp_push::{
    payload_cancelar, payload_crear_departamento, payload_crear_familia, payload_inventario,
    payload_propina, payload_puntos, payload_regularizacion, BdpPushService, DOMINIO_ARTICULO,
    DOMINIO_DEPARTAMENTO, ESTADO_ERROR, ESTADO_PENDIENTE, ESTADO_PENDIENTE_SUSCRIPCION,
    ESTADO_SINCRONIZADO, OPERACION_CREAR,
};
use glory_backend::services::bdp_weblink_catalog::BdpStockInfoEntry;
use glory_backend::services::{BdpBackupService, BdpPushFlushService, ConfiguracionService};
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[sqlx::test(migrations = "./migrations")]
async fn encolar_crea_y_refresca_fila_activa_sin_duplicar(pool: PgPool) {
    let user = Uuid::new_v4();
    let payload = serde_json::json!({ "ArtCode": 90000123, "ArtDescription": "Plato" });

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
    let pendientes = BdpPushService::listar_pendientes(&pool, user)
        .await
        .unwrap();
    assert_eq!(pendientes.len(), 1);
    assert_eq!(pendientes[0].dominio, DOMINIO_ARTICULO);
    assert_eq!(pendientes[0].estado, ESTADO_PENDIENTE);

    /* Refrescar el payload no debe duplicar la fila activa (M19). */
    let payload2 = serde_json::json!({ "ArtCode": 90000123, "ArtDescription": "Plato editado" });
    BdpPushService::encolar(
        &pool,
        user,
        DOMINIO_ARTICULO,
        "90000123",
        OPERACION_CREAR,
        &payload2,
    )
    .await
    .unwrap();
    let pendientes = BdpPushService::listar_pendientes(&pool, user)
        .await
        .unwrap();
    assert_eq!(pendientes.len(), 1);
    assert_eq!(pendientes[0].payload["ArtDescription"], "Plato editado");
}

#[sqlx::test(migrations = "./migrations")]
async fn suscripcion_no_consume_reintentos(pool: PgPool) {
    let user = Uuid::new_v4();
    let payload = serde_json::json!({});
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

    /* Error transitorio: incrementa reintentos. */
    BdpPushService::marcar_resultado(
        &pool,
        user,
        DOMINIO_ARTICULO,
        "90000123",
        OPERACION_CREAR,
        ESTADO_ERROR,
        Some("timeout"),
        true,
    )
    .await
    .unwrap();
    let pendientes = BdpPushService::listar_pendientes(&pool, user)
        .await
        .unwrap();
    assert_eq!(pendientes[0].reintentos, 1);

    /* Bloqueo por suscripción: NO incrementa (D2 resuelta). */
    BdpPushService::marcar_resultado(
        &pool,
        user,
        DOMINIO_ARTICULO,
        "90000123",
        OPERACION_CREAR,
        ESTADO_PENDIENTE_SUSCRIPCION,
        Some("Subscripción no activada"),
        false,
    )
    .await
    .unwrap();
    let pendientes = BdpPushService::listar_pendientes(&pool, user)
        .await
        .unwrap();
    assert_eq!(pendientes[0].reintentos, 1);
    assert_eq!(pendientes[0].estado, ESTADO_PENDIENTE_SUSCRIPCION);
}

#[sqlx::test(migrations = "./migrations")]
async fn orden_por_dependencia_departamento_antes_de_articulo(pool: PgPool) {
    let user = Uuid::new_v4();
    let payload = serde_json::json!({});
    BdpPushService::encolar(
        &pool,
        user,
        DOMINIO_ARTICULO,
        "a1",
        OPERACION_CREAR,
        &payload,
    )
    .await
    .unwrap();
    BdpPushService::encolar(
        &pool,
        user,
        DOMINIO_DEPARTAMENTO,
        "d1",
        OPERACION_CREAR,
        &payload,
    )
    .await
    .unwrap();

    let pendientes = BdpPushService::listar_pendientes(&pool, user)
        .await
        .unwrap();
    assert_eq!(pendientes[0].dominio, DOMINIO_DEPARTAMENTO);
    assert_eq!(pendientes[1].dominio, DOMINIO_ARTICULO);
}

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

/* ===== [198A-1/F1] Payloads encolados (unit, sin DB) ===== */

fn config_con_push() -> ConfiguracionRestaurante {
    ConfiguracionRestaurante {
        bdp_almacen_default: 1,
        bdp_codreg_default: 2,
        bdp_tav_map: serde_json::json!({ "21": 5, "10": 4 }),
        ..Default::default()
    }
}

#[test]
fn payload_inventario_serializa_lote_pascal_case() {
    let config = config_con_push();
    let lineas = vec![
        BdpStockInfoEntry {
            article: 90000123,
            units: Decimal::from_str("4.5").unwrap(),
        },
        BdpStockInfoEntry {
            article: 90000124,
            units: Decimal::from_str("0").unwrap(),
        },
    ];
    let payload = payload_inventario(&config, lineas).unwrap();
    assert_eq!(payload["CodReg"], 2);
    assert_eq!(payload["Store"], 1);
    assert!(payload["DateReg"].as_str().is_some());
    let articulos = payload["ArticlesList"].as_array().unwrap();
    assert_eq!(articulos.len(), 2);
    assert_eq!(articulos[0]["Article"], 90000123);
    /* Decimal serializa como string (rust_decimal serde-with-str). */
    assert_eq!(articulos[0]["Units"], "4.5");
    assert_eq!(articulos[1]["Units"], "0");
}

#[test]
fn payload_propina_serializa_order_identifier_y_suma() {
    let payload = payload_propina(12345, Decimal::from_str("3.50").unwrap(), true).unwrap();
    assert_eq!(payload["OrderIdentifier"]["OrderId"], 12345);
    assert_eq!(payload["Amount"], "3.50");
    assert_eq!(payload["AddTip"], true);
}

#[test]
fn payload_cancelar_serializa_pos_y_order_identifier() {
    let config = ConfiguracionRestaurante {
        bdp_pos_id: 31,
        ..Default::default()
    };
    let payload = payload_cancelar(&config, 5330).unwrap();
    assert_eq!(payload["PosId"], 31);
    assert_eq!(payload["OrderIdentifier"]["OrderId"], 5330);
    /* M26: sin Room/Table/Market (el local solo guarda bdp_order_id). */
    assert!(payload["OrderIdentifier"].get("RoomNumber").is_none());
    assert!(payload["OrderIdentifier"].get("TableNumber").is_none());
    assert!(payload["OrderIdentifier"].get("MarketId").is_none());
}

#[test]
fn payload_puntos_serializa_cliente_y_motivo() {
    let payload = payload_puntos(999, Decimal::from_str("-10").unwrap(), "canje").unwrap();
    assert_eq!(payload["Customer"], 999);
    assert_eq!(payload["PointsAdded"], "-10");
    assert_eq!(payload["Reason"], "canje");
}

#[test]
fn payload_crear_departamento_serializa_all_profiles() {
    let payload = payload_crear_departamento(5, "Cocina").unwrap();
    assert_eq!(payload["Code"], 5);
    assert_eq!(payload["Description"], "Cocina");
    assert_eq!(payload["AllProfiles"], true);
    assert_eq!(payload["Overwrite"], false);
}

#[test]
fn payload_crear_familia_serializa_code_y_overwrite() {
    let payload = payload_crear_familia(7, "Bebidas").unwrap();
    assert_eq!(payload["Code"], 7);
    assert_eq!(payload["Description"], "Bebidas");
    assert_eq!(payload["Overwrite"], false);
}

#[test]
fn payload_regularizacion_serializa_delta_y_almacen() {
    let config = config_con_push();
    let payload =
        payload_regularizacion(&config, 90000123, Decimal::from_str("-2").unwrap()).unwrap();
    assert_eq!(payload["Article"], 90000123);
    assert_eq!(payload["Units"], "-2");
    assert_eq!(payload["CodReg"], 2);
    assert_eq!(payload["Store"], 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn sincronizado_sale_de_la_cola_activa(pool: PgPool) {
    let user = Uuid::new_v4();
    let payload = serde_json::json!({});
    BdpPushService::encolar(
        &pool,
        user,
        DOMINIO_ARTICULO,
        "a1",
        OPERACION_CREAR,
        &payload,
    )
    .await
    .unwrap();
    BdpPushService::marcar_resultado(
        &pool,
        user,
        DOMINIO_ARTICULO,
        "a1",
        OPERACION_CREAR,
        ESTADO_SINCRONIZADO,
        None,
        false,
    )
    .await
    .unwrap();
    assert!(BdpPushService::listar_pendientes(&pool, user)
        .await
        .unwrap()
        .is_empty());
}
