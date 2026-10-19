// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
/* [198A-1/F1] Tests de la cola de push unidireccional Glory -> BDP
 * (bdp_push_pendientes). Verifican: upsert de fila activa (M19), política de
 * reintentos (transitorio vs suscripción, D2), orden por dependencia de
 * dominio (M12) y salida de la cola activa al sincronizar. */
use glory_backend::models::{ActualizarConfiguracionRequest, ConfiguracionRestaurante};
use glory_backend::services::bdp_push::{
    payload_cancelar, payload_crear_departamento, payload_crear_familia, payload_inventario,
    payload_propina, payload_puntos, payload_regularizacion, BdpPushService, MarcarResultado,
    DOMINIO_ARTICULO, DOMINIO_DEPARTAMENTO, ESTADO_ERROR, ESTADO_PENDIENTE,
    ESTADO_PENDIENTE_SUSCRIPCION, ESTADO_RECHAZADO, ESTADO_SINCRONIZADO, OPERACION_CREAR,
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
    let payload = serde_json::json!({ "ArtCode": 90_000_123, "ArtDescription": "Plato" });

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
    let payload2 = serde_json::json!({ "ArtCode": 90_000_123, "ArtDescription": "Plato editado" });
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
        MarcarResultado {
            dominio: DOMINIO_ARTICULO,
            entidad_id: "90000123",
            operacion: OPERACION_CREAR,
            estado: ESTADO_ERROR,
            error: Some("timeout"),
            incrementar_reintento: true,
        },
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
        MarcarResultado {
            dominio: DOMINIO_ARTICULO,
            entidad_id: "90000123",
            operacion: OPERACION_CREAR,
            estado: ESTADO_PENDIENTE_SUSCRIPCION,
            error: Some("Subscripción no activada"),
            incrementar_reintento: false,
        },
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
            article: 90_000_123,
            units: Decimal::from_str("4.5").unwrap(),
        },
        BdpStockInfoEntry {
            article: 90_000_124,
            units: Decimal::from_str("0").unwrap(),
        },
    ];
    let payload = payload_inventario(&config, lineas).unwrap();
    assert_eq!(payload["CodReg"], 2);
    assert_eq!(payload["Store"], 1);
    assert!(payload["DateReg"].as_str().is_some());
    let articulos = payload["ArticlesList"].as_array().unwrap();
    assert_eq!(articulos.len(), 2);
    assert_eq!(articulos[0]["Article"], 90_000_123);
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
    let payload = payload_crear_departamento(5, "Cocina", false).unwrap();
    assert_eq!(payload["Code"], 5);
    assert_eq!(payload["Description"], "Cocina");
    assert_eq!(payload["AllProfiles"], true);
    assert_eq!(payload["Overwrite"], false);
    /* W-Q2.3: abreviada máx 10 caracteres y nivel de impresión 1-9. */
    assert_eq!(payload["ShortDescription"], "Cocina");
    assert_eq!(payload["PrinterLevel"], 1);
    let largo = payload_crear_departamento(6, "PRUEBA-149A2-DEP-20260915", false).unwrap();
    assert_eq!(largo["ShortDescription"], "PRUEBA-149");
}

#[test]
fn payload_crear_familia_serializa_code_y_overwrite() {
    let payload = payload_crear_familia(7, "Bebidas", false).unwrap();
    assert_eq!(payload["Code"], 7);
    assert_eq!(payload["Description"], "Bebidas");
    assert_eq!(payload["Overwrite"], false);
}

#[test]
fn payload_regularizacion_serializa_delta_y_almacen() {
    let config = config_con_push();
    let payload =
        payload_regularizacion(&config, 90_000_123, Decimal::from_str("-2").unwrap(), 3).unwrap();
    assert_eq!(payload["Article"], 90_000_123);
    assert_eq!(payload["Units"], "-2");
    assert_eq!(payload["CodReg"], 2);
    /* El almacén es el del ajuste (3), no el por defecto de la configuración (1). */
    assert_eq!(payload["Store"], 3);
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
        MarcarResultado {
            dominio: DOMINIO_ARTICULO,
            entidad_id: "a1",
            operacion: OPERACION_CREAR,
            estado: ESTADO_SINCRONIZADO,
            error: None,
            incrementar_reintento: false,
        },
    )
    .await
    .unwrap();
    assert!(BdpPushService::listar_pendientes(&pool, user)
        .await
        .unwrap()
        .is_empty());
}

/* [049A-1/S8] Cola — reintento manual único desde Sincronización. Escenario
 * del plan: fila pendiente → el flush automático (modalidad `manual`, sin
 * forzar) NO envía nada (sin auto-flush) → el reintento manual individual
 * (`reintentar_uno`, el mecanismo de la UI Sincronización) procesa la fila.
 *
 * Diseño honesto al guard fail-closed: un 5xx deja la intención `ambiguo`
 * (pudo aplicarse) y `authorize` bloquea el reenvío hasta reconciliar (H-W-2)
 * — por eso aquí el reintento manual se ejercita sobre un rechazo **4xx
 * definitivo** (auditoría `error`, NO `ambiguo`, sin bloqueo anti-duplicación)
 * que tras re-edición local (M19) vuelve a `pendiente` y puede reintentarse.
 * Resultado: cada reintento manual hace UNA llamada HTTP, el flush automático
 * ninguna, y el estado por ítem queda correcto (`pendiente` → `rechazado`
 * visible → `pendiente` → `sincronizado`, fuera de la cola activa). */
#[sqlx::test(migrations = "./migrations")]
async fn cola_reintento_manual_uno_error_visible_sin_auto_flush(pool: PgPool) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Auth/Login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "AuthSession": { "Token": "token-bdp", "ExpiresIn_InSecconds": 3540 }
        })))
        .mount(&server)
        .await;
    /* El BDP rechaza de forma definitiva (422) el primer envío y acepta (200)
     * el reintento tras corregir el dato. El 422 se consume UNA sola vez. */
    Mock::given(method("POST"))
        .and(path("/API/Articles/CreateAndUpdateProfiles"))
        .respond_with(ResponseTemplate::new(422).set_body_json(serde_json::json!({
            "ErrorMessage": "El código de artículo ya existe"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/API/Articles/CreateAndUpdateProfiles"))
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

    /* Config modo BDP con `push_modalidad = manual`: el flush automático
     * (sin forzar) no debe enviar nada (sin auto-flush); el reintento manual
     * individual es el único mecanismo. */
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
            push_modalidad: Some("manual".to_string()),
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

    /* Leer el id de la fila encolada (para el reintento individual). */
    let fila_id = BdpPushService::listar_pendientes(&pool, user)
        .await
        .unwrap()
        .into_iter()
        .find(|f| f.dominio == DOMINIO_ARTICULO && f.entidad_id == "90000123")
        .expect("fila encolada")
        .id;

    /* 1) Flush automático (sin forzar) en modalidad manual: sin auto-flush.
     *    La fila sigue pendiente y NO se hace ninguna llamada HTTP. */
    let resumen_auto = BdpPushFlushService::flush(&pool, user, false)
        .await
        .expect("flush automático en manual no debe fallar");
    assert_eq!(resumen_auto.omitidos_manual, 1);
    assert_eq!(resumen_auto.procesados, 0);
    let envios_auto = server
        .received_requests()
        .await
        .iter()
        .flatten()
        .filter(|req| req.url.path() == "/API/Articles/CreateAndUpdateProfiles")
        .count();
    assert_eq!(
        envios_auto, 0,
        "el flush automático no envía en modo manual"
    );

    /* 2) Reintento manual individual contra un BDP que rechaza (422):
     *    error visible y definitivo. Estado `rechazado`, cero reintentos
     *    consumidos (4xx no es transitorio), auditoría `error` (NO `ambiguo`,
     *    por eso el reintento posterior no queda bloqueado) y UNA sola
     *    llamada HTTP. */
    let resumen_err = BdpPushFlushService::reintentar_uno(&pool, user, fila_id)
        .await
        .expect("reintento manual no debe fallar");
    assert_eq!(resumen_err.procesados, 1);
    assert_eq!(resumen_err.rechazados, 1);
    assert_eq!(resumen_err.errores, 0);
    assert_eq!(resumen_err.sincronizados, 0);
    let (estado, reintentos, ultimo_error): (String, i32, Option<String>) = sqlx::query_as(
        "SELECT estado, reintentos, ultimo_error FROM bdp_push_pendientes \
         WHERE id = $1 AND user_id = $2",
    )
    .bind(fila_id)
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(estado, ESTADO_RECHAZADO);
    assert_eq!(reintentos, 0);
    assert!(
        ultimo_error.as_deref().unwrap_or("").contains("422"),
        "Error visible con el status esperado: {ultimo_error:?}"
    );
    /* Auditoría: rechazo definitivo → `error`, nunca `ambiguo`. */
    let (resultado_audit,): (String,) = sqlx::query_as(
        "SELECT resultado FROM bdp_audit_log \
         WHERE user_id = $1 AND operacion = 'create_article' \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(resultado_audit, "error");
    let envios_1 = server
        .received_requests()
        .await
        .iter()
        .flatten()
        .filter(|req| req.url.path() == "/API/Articles/CreateAndUpdateProfiles")
        .count();
    assert_eq!(envios_1, 1, "el reintento manual hace UNA llamada HTTP");

    /* 3) Re-edición local (encolar de nuevo con el dato corregido) refresca la
     *    fila `rechazada` a `pendiente` (M19): vuelve a la cola activa. */
    let payload_corregido = serde_json::json!({
        "AutomaticCode": false,
        "ArticleData": { "ArtCode": 90_000_123, "ArtDescription": "Plato corregido" },
        "AllProfiles": true
    });
    BdpPushService::encolar(
        &pool,
        user,
        DOMINIO_ARTICULO,
        "90000123",
        OPERACION_CREAR,
        &payload_corregido,
    )
    .await
    .unwrap();
    let (estado_reeditado,): (String,) = sqlx::query_as(
        "SELECT estado FROM bdp_push_pendientes \
         WHERE id = $1 AND user_id = $2",
    )
    .bind(fila_id)
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(estado_reeditado, ESTADO_PENDIENTE);

    /* 4) El flush automático sigue sin tocar la fila (modalidad manual: sin
     *    auto-flush; el reintento manual es el único mecanismo). */
    let resumen_auto2 = BdpPushFlushService::flush(&pool, user, false)
        .await
        .unwrap();
    assert_eq!(resumen_auto2.omitidos_manual, 1);
    assert_eq!(resumen_auto2.procesados, 0);
    let envios_2 = server
        .received_requests()
        .await
        .iter()
        .flatten()
        .filter(|req| req.url.path() == "/API/Articles/CreateAndUpdateProfiles")
        .count();
    assert_eq!(envios_2, 1, "el flush automático no reintenta la fila");

    /* 5) Reintento manual individual, ahora con el BDP aceptando (200):
     *    sincroniza y la fila sale de la cola activa. Una única llamada HTTP. */
    let resumen_ok = BdpPushFlushService::reintentar_uno(&pool, user, fila_id)
        .await
        .expect("reintento manual tras corregir no debe fallar");
    assert_eq!(resumen_ok.procesados, 1);
    assert_eq!(resumen_ok.sincronizados, 1);
    assert_eq!(resumen_ok.errores, 0);
    /* La fila sale de la cola activa al sincronizar. */
    let restantes = BdpPushService::listar_pendientes(&pool, user)
        .await
        .unwrap();
    assert!(
        restantes.iter().all(|f| f.id != fila_id),
        "la fila sincronizada no debe seguir en la cola activa"
    );
    let (resultado_audit_ok,): (String,) = sqlx::query_as(
        "SELECT resultado FROM bdp_audit_log \
         WHERE user_id = $1 AND operacion = 'create_article' \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(resultado_audit_ok, "exito");
    let envios_3 = server
        .received_requests()
        .await
        .iter()
        .flatten()
        .filter(|req| req.url.path() == "/API/Articles/CreateAndUpdateProfiles")
        .count();
    assert_eq!(envios_3, 2, "total: dos reintentos manuales, dos llamadas");
}
