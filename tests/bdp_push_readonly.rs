// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
/* [10AA-4] Solo lectura significa solo lectura, también para el barrido
 * automático.
 *
 * El worker de push seleccionaba las cuentas por `push_modalidad = 'automatico'`
 * y nunca miraba `bdp_sync_mode`. Como esa columna nace en `read_only` y
 * `push_modalidad` nace en `automatico` (migración 20260821000000), toda cuenta
 * recién creada escribía en el TPV en el primer barrido, y de paso `auto_arm_inner`
 * cambiaba el modo a `unidirectional` por su cuenta. Este test fija la regla
 * contraria: con `read_only` no sale un solo byte de escritura hacia el TPV y el
 * modo guardado no cambia. */

use glory_backend::models::ActualizarConfiguracionRequest;
use glory_backend::services::bdp_push::{BdpPushService, DOMINIO_ARTICULO, OPERACION_CREAR};
use glory_backend::services::{BdpPushFlushService, ConfiguracionService};
use sqlx::PgPool;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/* Cuenta lista para escribir salvo por el modo: push automático, sync activo,
 * backup pre-escritura, credenciales completas y una fila en la cola. */
async fn cuenta_con_fila_encolada(pool: &PgPool, base_url: &str) -> Uuid {
    let user = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(user)
        .bind(format!("test-{user}@example.com"))
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .unwrap();

    ConfiguracionService::actualizar(
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
            push_modalidad: Some("automatico".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("config de la cuenta");

    BdpPushService::encolar(
        pool,
        user,
        DOMINIO_ARTICULO,
        "90000123",
        OPERACION_CREAR,
        &serde_json::json!({
            "AutomaticCode": false,
            "ArticleData": { "ArtCode": 90_000_123, "ArtDescription": "Plato" },
            "AllProfiles": true
        }),
    )
    .await
    .unwrap();

    user
}

async fn modo_guardado(pool: &PgPool, user: Uuid) -> String {
    sqlx::query_scalar("SELECT bdp_sync_mode FROM configuracion_restaurante WHERE user_id = $1")
        .bind(user)
        .fetch_one(pool)
        .await
        .unwrap()
}

/* ===== El barrido periódico no escribe con la cuenta en solo lectura ===== */

#[sqlx::test(migrations = "./migrations")]
async fn barrido_automatico_en_solo_lectura_no_escribe_ni_cambia_el_modo(pool: PgPool) {
    let server = MockServer::start().await;
    /* Si algo llegara a enviarse, el mock lo registraría. */
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&server)
        .await;

    let user = cuenta_con_fila_encolada(&pool, &server.uri()).await;
    /* El default de la columna ya es `read_only`; se fija explícito para que el
     * test no dependa de esa migración. */
    sqlx::query(
        "UPDATE configuracion_restaurante SET bdp_sync_mode = 'read_only' WHERE user_id = $1",
    )
    .bind(user)
    .execute(&pool)
    .await
    .unwrap();

    BdpPushFlushService::sincronizar_cuentas_automaticas(&pool).await;

    let peticiones = server.received_requests().await.unwrap();
    assert!(
        peticiones.is_empty(),
        "en solo lectura no debe salir ninguna petición al TPV; salieron {}: {:?}",
        peticiones.len(),
        peticiones
            .iter()
            .map(|r| r.url.path().to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        modo_guardado(&pool, user).await,
        "read_only",
        "el barrido no debe cambiar el modo de sincronización"
    );

    /* La fila sigue esperando, sin consumir reintentos: se enviará cuando la
     * cuenta pase de verdad a un modo de escritura. */
    let pendientes = BdpPushService::listar_pendientes(&pool, user)
        .await
        .unwrap();
    assert_eq!(pendientes.len(), 1);
    assert_eq!(pendientes[0].reintentos, 0);
}

/* ===== El botón manual sigue siendo el permiso de una operación =====
 * Con la cuenta en solo lectura, el barrido no escribe (test anterior), pero la
 * acción explícita de una persona sí: pide un snapshot vigente, arma UNA
 * operación y `cerrar_armado` devuelve el modo a `read_only`. Es el kill switch
 * documentado; lo que se fija aquí es que no se queda escribiendo: tras ese
 * envío, el barrido periódico vuelve a no tocar el TPV. */

#[sqlx::test(migrations = "./migrations")]
async fn el_boton_manual_arma_una_operacion_y_vuelve_a_solo_lectura(pool: PgPool) {
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

    let user = cuenta_con_fila_encolada(&pool, &server.uri()).await;
    let config = ConfiguracionService::obtener(&pool, user).await.unwrap();
    let target = glory_backend::services::BdpBackupService::canonical_target(&config).unwrap();
    let fingerprint =
        glory_backend::services::BdpBackupService::connection_fingerprint(&config).unwrap();
    sqlx::query(
        "INSERT INTO bdp_snapshots          (user_id, tipo, direccion, trigger_tipo, datos, target_base_url, connection_fingerprint)          VALUES ($1, 'completo', 'bdp', 'manual', '{}'::jsonb, $2, $3)",
    )
    .bind(user)
    .bind(&target)
    .bind(&fingerprint)
    .execute(&pool)
    .await
    .unwrap();
    let resumen = BdpPushFlushService::flush(&pool, user, true).await.unwrap();
    assert_eq!(resumen.sincronizados, 1, "resumen: {resumen:?}");
    assert_eq!(
        modo_guardado(&pool, user).await,
        "read_only",
        "el permiso se gasta con la operación"
    );

    /* Fila nueva después: si el permiso sobreviviera al envío, el barrido la
     * cogería. Se encola después del flush para que el envío no la toque. */
    BdpPushService::encolar(
        &pool,
        user,
        DOMINIO_ARTICULO,
        "90000124",
        OPERACION_CREAR,
        &serde_json::json!({
            "AutomaticCode": false,
            "ArticleData": { "ArtCode": 90_000_124, "ArtDescription": "Segundo" },
            "AllProfiles": true
        }),
    )
    .await
    .unwrap();

    /* Y el barrido de fondo, que es lo que corre solo, no sigue escribiendo. */
    let antes = server.received_requests().await.unwrap().len();
    BdpPushFlushService::sincronizar_cuentas_automaticas(&pool).await;
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        antes,
        "tras gastar el permiso, el barrido no vuelve a llamar al TPV"
    );
    assert_eq!(modo_guardado(&pool, user).await, "read_only");
}

/* ===== Contraste: con un modo de escritura sí sale la petición =====
 * Sin este positivo, el test de arriba pasaría también si el flush estuviera
 * roto por cualquier otro motivo. */

#[sqlx::test(migrations = "./migrations")]
async fn contraste_con_modo_unidireccional_si_envia(pool: PgPool) {
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

    let user = cuenta_con_fila_encolada(&pool, &server.uri()).await;
    /* Snapshot completo vigente de esta conexión: prerequisito del armado. */
    let config = ConfiguracionService::obtener(&pool, user).await.unwrap();
    let target = glory_backend::services::BdpBackupService::canonical_target(&config).unwrap();
    let fingerprint =
        glory_backend::services::BdpBackupService::connection_fingerprint(&config).unwrap();
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
    sqlx::query(
        "UPDATE configuracion_restaurante SET bdp_sync_mode = 'unidirectional' WHERE user_id = $1",
    )
    .bind(user)
    .execute(&pool)
    .await
    .unwrap();

    let resumen = BdpPushFlushService::flush(&pool, user, true).await.unwrap();

    assert_eq!(resumen.sincronizados, 1, "resumen: {resumen:?}");
    assert!(server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .any(|r| r.url.path() == "/API/Articles/CreateAndUpdateProfiles"));
    /* El armado se consume y la cuenta vuelve sola a solo lectura. */
    assert_eq!(modo_guardado(&pool, user).await, "read_only");
}
