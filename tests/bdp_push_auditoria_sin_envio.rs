// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/*! [21AB-5g] Una escritura que no llegó al TPV no puede dejar su auditoría abierta.
 *
 * `authorize` abre la auditoría en `pendiente`; `ledger::preparar` (que lee
 * `GetListStock`) y `preparado.abrir` corren después y antes del HTTP. Cuando
 * cualquiera de los dos fallaba, `flush` devolvía el error sin cerrar la
 * auditoría: quedaba `pendiente` para siempre y bloqueaba ese artículo y
 * almacén, además de la fila de la cola. Caso real: traspaso al almacén 0, que
 * el demo rechaza con `[200200] EL ALMACÉN 0 NO EXISTE` (medido el 22/10).
 */

use glory_backend::models::ActualizarConfiguracionRequest;
use glory_backend::services::bdp_push::{BdpPushService, DOMINIO_STOCK, OPERACION_TRASPASAR};
use glory_backend::services::{
    entidad_uuid, BdpBackupService, BdpPushFlushService, ConfiguracionService,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Cuenta en modo BDP con snapshot vigente y almacén por defecto.
async fn usuario_listo(pool: &PgPool, base_url: &str) -> Uuid {
    let user = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(user)
        .bind(format!("test-{user}@example.com"))
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .unwrap();
    let config = ConfiguracionService::actualizar(
        pool,
        user,
        &ActualizarConfiguracionRequest {
            modo_operacion: Some("bdp".to_string()),
            bdp_base_url: Some(base_url.to_string()),
            bdp_sync_enabled: Some(true),
            bdp_auto_backup_before_write: Some(true),
            bdp_login: Some("usuario".to_string()),
            bdp_password: Some("secreto".to_string()),
            bdp_integrator_code: Some("INTEGRADOR".to_string()),
            bdp_pos_id: Some(1),
            bdp_employee_id: Some(1),
            bdp_items_profile_id: Some(1),
            bdp_almacen_default: Some(1),
            bdp_codreg_default: Some(1),
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
    .execute(pool)
    .await
    .unwrap();
    user
}

/// Traspaso desde el almacén 0, como lo dejó el handler real.
fn traspaso() -> serde_json::Value {
    json!({
        "Article": 10001, "sD1": "", "sD2": "", "sD3": "",
        "Units": 1, "CodTransfer": 1, "StoreFrom": 0, "StoreTo": 1,
        "DateTransfer": "2026-10-22"
    })
}

async fn encolar_traspaso(pool: &PgPool, user: Uuid, entidad_id: &str) {
    BdpPushService::encolar(
        pool,
        user,
        DOMINIO_STOCK,
        entidad_id,
        OPERACION_TRASPASAR,
        &traspaso(),
    )
    .await
    .unwrap();
}

/// El almacén 0 no existe en BDP: `GetListStock` vuelve sin líneas y con
/// `[200200]`, así que la lectura previa falla. La escritura no salió; la
/// auditoría se cierra en `error` con el motivo y el par no queda bloqueado.
#[sqlx::test(migrations = "./migrations")]
async fn la_lectura_previa_fallida_cierra_la_auditoria_en_error(pool: PgPool) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(|request: &wiremock::Request| match request.url.path() {
            "/Auth/Login" => ResponseTemplate::new(200).set_body_json(json!({
                "ErrorMessage": "",
                "AuthSession": { "Token": "token-bdp", "ExpiresIn_InSecconds": 3540 }
            })),
            /* El demo, literal: sin líneas y con el error de negocio. */
            "/API/Warehouse/GetListStock" => ResponseTemplate::new(200).set_body_json(json!({
                "Stock": [],
                "ErrorMessage": "[200200]-EL ALMACÉN 0 NO EXISTE"
            })),
            _ => ResponseTemplate::new(200).set_body_json(json!({ "ErrorMessage": "" })),
        })
        .mount(&server)
        .await;

    let user = usuario_listo(&pool, &server.uri()).await;
    let entidad = "10001:0>1:fila-de-prueba";
    encolar_traspaso(&pool, user, entidad).await;

    let resumen = BdpPushFlushService::flush(&pool, user, true).await.unwrap();
    assert_eq!(resumen.errores, 1);
    assert_eq!(resumen.sincronizados, 0, "no hubo HTTP de escritura");

    let (resultado, mensaje): (String, Option<String>) = sqlx::query_as(
        "SELECT resultado, error_mensaje FROM bdp_audit_log \
         WHERE user_id = $1 AND target_entity_type = 'stock' AND target_entity_id = $2 \
           AND operacion = 'transfer_stock'",
    )
    .bind(user)
    .bind(entidad_uuid(DOMINIO_STOCK, entidad))
    .fetch_one(&pool)
    .await
    .expect("leer la auditoría del traspaso");
    assert_eq!(
        resultado, "error",
        "una auditoría abierta bloquea el artículo y almacén para siempre"
    );
    let mensaje = mensaje.unwrap_or_default();
    assert!(
        mensaje.starts_with("no se envió a BDP:"),
        "el motivo dice que no se envió, no que el TPV dijo que no: {mensaje}"
    );
    assert!(mensaje.contains("ALMACÉN 0 NO EXISTE"), "motivo: {mensaje}");

    /* Ninguna fila del libro: la lectura previa falló antes de abrirlo. */
    let filas: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bdp_stock_ledger WHERE user_id = $1")
        .bind(user)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(filas, 0);
}

/// El bloqueo del par artículo y almacén deja la auditoría sin enviar igual:
/// `abrir` también va antes del HTTP.
#[sqlx::test(migrations = "./migrations")]
async fn el_bloqueo_del_par_cierra_la_auditoria_en_error(pool: PgPool) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(|request: &wiremock::Request| match request.url.path() {
            "/Auth/Login" => ResponseTemplate::new(200).set_body_json(json!({
                "ErrorMessage": "",
                "AuthSession": { "Token": "token-bdp", "ExpiresIn_InSecconds": 3540 }
            })),
            "/API/Warehouse/GetListStock" => ResponseTemplate::new(200).set_body_json(json!({
                "Stock": [{ "Article": 10001, "Altern": 0, "Units": 7.0,
                            "ErrorMessage": "", "Store": 1 }],
                "ErrorMessage": ""
            })),
            _ => ResponseTemplate::new(200).set_body_json(json!({ "ErrorMessage": "" })),
        })
        .mount(&server)
        .await;

    let user = usuario_listo(&pool, &server.uri()).await;
    /* Otra operación del mismo par sigue pendiente en el libro. */
    let documento = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO bdp_stock_ledger \
         (id, user_id, articulo_codigo, almacen, operacion, variacion, stock_previo, \
          stock_esperado, id_local, origen, documento_id) \
         VALUES ($1, $2, '10001', 0, 'traspasar', 1, 7, 8, 'otra-operacion', 'web', $3)",
    )
    .bind(documento)
    .bind(user)
    .bind(documento.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let entidad = "10001:0>1:fila-de-prueba";
    encolar_traspaso(&pool, user, entidad).await;
    let resumen = BdpPushFlushService::flush(&pool, user, true).await.unwrap();
    assert_eq!(resumen.errores, 1);

    let (resultado, mensaje): (String, Option<String>) = sqlx::query_as(
        "SELECT resultado, error_mensaje FROM bdp_audit_log \
         WHERE user_id = $1 AND target_entity_type = 'stock' AND target_entity_id = $2 \
           AND operacion = 'transfer_stock'",
    )
    .bind(user)
    .bind(entidad_uuid(DOMINIO_STOCK, entidad))
    .fetch_one(&pool)
    .await
    .expect("leer la auditoría del traspaso");
    assert_eq!(resultado, "error");
    let mensaje = mensaje.unwrap_or_default();
    assert!(
        mensaje.contains("otra operación de stock pendiente"),
        "motivo: {mensaje}"
    );
}
