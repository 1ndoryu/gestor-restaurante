// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/*! [21AB-5d] Una modificación ambigua no puede bloquear la venta para siempre.
 *
 * `ensure_no_unresolved` bloquea cualquier escritura de una venta mientras tenga
 * una auditoría `pendiente`/`ambiguo`. El poller reconciliaba `add_payment` e
 * `invoice`, y el `create_order` lo resolvía `recuperar_lista_en_vuelo`, pero
 * `modify_order` no lo miraba nadie: la venta se quedaba muerta.
 *
 * Además, una modificación ambigua de una venta ya facturada no aparece en
 * ninguna de las tres listas de ventas, así que el corte temprano de
 * `poll_pending` ni siquiera llegaba a reconciliar. El total final de la
 * comanda es la firma del cambio: si el TPV lo tiene, se aplicó; si no, la
 * escritura no llegó y la web (que nunca cambió) puede volver a editarse.
 */

use glory_backend::models::ConfiguracionRestaurante;
use glory_backend::repositories::{ConfiguracionRepository, VentaRepository};
use glory_backend::services::{BdpOrderPollerService, ServicioModoOperacion};
use sqlx::PgPool;
use uuid::Uuid;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Cuenta con BDP apuntando al mock que se monte después.
async fn usuario(pool: &PgPool) -> (Uuid, ConfiguracionRestaurante) {
    let user_id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(format!("21ab5d-{user_id}@example.com"))
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("crear usuario de prueba");
    ConfiguracionRepository::obtener_o_crear(pool, user_id)
        .await
        .expect("crear configuración");
    sqlx::query(
        "UPDATE configuracion_restaurante \
         SET bdp_base_url = 'http://127.0.0.1:59999', \
             bdp_login = 'u', bdp_password = 'u', bdp_integrator_code = 'SIM', \
             modo_operacion = 'bdp', bdp_sync_mode = 'unidirectional', \
             bdp_sync_enabled = TRUE, bdp_auto_backup_before_write = TRUE \
         WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(pool)
    .await
    .expect("configurar BDP");
    let config = ConfiguracionRepository::obtener_o_crear(pool, user_id)
        .await
        .expect("releer configuración");
    (user_id, config)
}

/// Venta ya facturada (fuera de todas las listas del poller) con una
/// modificación ambigua abierta, tal como la dejaba el código anterior.
async fn venta_facturada_con_modificacion_ambigua(
    pool: &PgPool,
    user_id: Uuid,
    total_final: f64,
) -> (Uuid, i64) {
    let venta_id = Uuid::new_v4();
    let order_id = 177;
    sqlx::query(
        "INSERT INTO ventas (id, user_id, fecha, turno, canal, metodo_pago, \
                             iva_porcentaje, importe_base, importe_iva, \
                             bdp_synced, bdp_order_id, bdp_invoiced, bdp_order_status) \
         VALUES ($1, $2, CURRENT_DATE, 'mediodia', 'comedor', 'efectivo', \
                 10, 10, 1, TRUE, $3, TRUE, 'invoiced')",
    )
    .bind(venta_id)
    .bind(user_id)
    .bind(order_id)
    .execute(pool)
    .await
    .expect("crear venta facturada");

    sqlx::query(
        "INSERT INTO bdp_audit_log \
           (user_id, operacion, direccion, datos_enviados, resultado, \
            target_base_url, target_entity_type, target_entity_id) \
         VALUES ($1, 'modify_order', 'glory_to_bdp', $2, 'ambiguo', \
                 'http://127.0.0.1:59999', 'venta', $3)",
    )
    .bind(user_id)
    .bind(serde_json::json!({
        "venta_id": venta_id, "operacion": "update_order",
        "order_id": order_id, "lineas_ajuste": 1, "total_final": total_final
    }))
    .bind(venta_id)
    .execute(pool)
    .await
    .expect("crear auditoría ambigua de modificación");
    (venta_id, order_id)
}

/// BDP de mentira que contesta `GetOrder` con el total que se le diga.
async fn bdp_con_total(total: f64) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            move |request: &wiremock::Request| match request.url.path() {
                "/Auth/Login" => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "ErrorMessage": "",
                    "AuthSession": { "Token": "token-bdp", "ExpiresIn_InSecconds": 3540 }
                })),
                "/API/Orders/Get" => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "Order": { "OrderId": 177, "Status": 1, "Total": total, "Items": [] },
                    "Status": 1, "ErrorMessage": ""
                })),
                _ => ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "ErrorMessage": "" })),
            },
        )
        .mount(&server)
        .await;
    server
}

async fn apuntar_al_mock(
    pool: &PgPool,
    user_id: Uuid,
    server: &MockServer,
) -> ConfiguracionRestaurante {
    sqlx::query("UPDATE configuracion_restaurante SET bdp_base_url = $2 WHERE user_id = $1")
        .bind(user_id)
        .bind(server.uri())
        .execute(pool)
        .await
        .expect("apuntar al BDP disponible");
    ConfiguracionRepository::obtener_o_crear(pool, user_id)
        .await
        .expect("releer configuración")
}

async fn resultado_de(pool: &PgPool, venta_id: Uuid) -> String {
    sqlx::query_scalar(
        "SELECT resultado FROM bdp_audit_log \
         WHERE target_entity_id = $1 AND operacion = 'modify_order'",
    )
    .bind(venta_id)
    .fetch_one(pool)
    .await
    .expect("leer auditoría")
}

/// El TPV tiene el total final: la modificación se aplicó y la auditoría se
/// cierra en éxito, aunque la venta esté facturada y no salga en ninguna lista.
#[sqlx::test(migrations = "./migrations")]
async fn el_poller_cierra_en_exito_la_modificacion_que_bdp_aplico(pool: PgPool) {
    let (user_id, _) = usuario(&pool).await;
    let (venta_id, _) = venta_facturada_con_modificacion_ambigua(&pool, user_id, 3.15).await;
    let server = bdp_con_total(3.15).await;
    let config = apuntar_al_mock(&pool, user_id, &server).await;

    let cerradas =
        BdpOrderPollerService::poll_pending(&pool, user_id, &config, &ServicioModoOperacion::new())
            .await
            .expect("el polling no revienta");

    assert_eq!(cerradas, 1, "lo reconciliado se cuenta");
    assert_eq!(resultado_de(&pool, venta_id).await, "exito");

    /* El guard deja de bloquear: la venta vuelve a ser editable. */
    let abiertas: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bdp_audit_log \
         WHERE user_id = $1 AND target_entity_id = $2 \
           AND resultado IN ('pendiente', 'ambiguo')",
    )
    .bind(user_id)
    .bind(venta_id)
    .fetch_one(&pool)
    .await
    .expect("contar auditorías abiertas");
    assert_eq!(abiertas, 0);
}

/// El TPV conserva otro total: la escritura no llegó. Se cierra en `error` (la
/// web nunca cambió, no hay nada que deshacer) y la venta deja de estar
/// bloqueada.
#[sqlx::test(migrations = "./migrations")]
async fn el_poller_cierra_en_error_la_modificacion_que_no_llego(pool: PgPool) {
    let (user_id, _) = usuario(&pool).await;
    let (venta_id, _) = venta_facturada_con_modificacion_ambigua(&pool, user_id, 3.15).await;
    let server = bdp_con_total(1.05).await;
    let config = apuntar_al_mock(&pool, user_id, &server).await;

    BdpOrderPollerService::poll_pending(&pool, user_id, &config, &ServicioModoOperacion::new())
        .await
        .expect("el polling no revienta");

    assert_eq!(
        resultado_de(&pool, venta_id).await,
        "error",
        "un total distinto es un fallo real, no un éxito"
    );
    let abiertas: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bdp_audit_log \
         WHERE user_id = $1 AND target_entity_id = $2 \
           AND resultado IN ('pendiente', 'ambiguo')",
    )
    .bind(user_id)
    .bind(venta_id)
    .fetch_one(&pool)
    .await
    .expect("contar auditorías abiertas");
    assert_eq!(abiertas, 0, "la venta no puede quedar bloqueada");
}

/// Sin poder preguntar a BDP no se decide nada: la auditoría sigue abierta.
#[sqlx::test(migrations = "./migrations")]
async fn sin_respuesta_de_bdp_la_modificacion_sigue_ambigua(pool: PgPool) {
    let (user_id, config) = usuario(&pool).await;
    let (venta_id, _) = venta_facturada_con_modificacion_ambigua(&pool, user_id, 3.15).await;

    /* El puerto 59999 no escucha: el `GetOrder` falla por transporte. */
    BdpOrderPollerService::poll_pending(&pool, user_id, &config, &ServicioModoOperacion::new())
        .await
        .expect("el polling no revienta con el receptor caído");

    assert_eq!(
        resultado_de(&pool, venta_id).await,
        "ambiguo",
        "sin comprobar en BDP no se cierra nada"
    );
}

/// [21AB-5g] BDP dice que la comanda no existe (`[301000]`, con `Health` vivo):
/// la modificación tampoco pudo aplicarse. Se cierra en `error` en vez de
/// quedarse `ambiguo` para siempre.
#[sqlx::test(migrations = "./migrations")]
async fn el_poller_cierra_en_error_la_modificacion_de_una_comanda_inexistente(pool: PgPool) {
    let (user_id, _) = usuario(&pool).await;
    let (venta_id, _) = venta_facturada_con_modificacion_ambigua(&pool, user_id, 3.15).await;

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(|request: &wiremock::Request| match request.url.path() {
            "/Auth/Login" => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ErrorMessage": "",
                "AuthSession": { "Token": "token-bdp", "ExpiresIn_InSecconds": 3540 }
            })),
            "/Service/Health" => {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "IsAlive": true }))
            }
            "/API/Orders/Get" => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Order": null, "Status": 0,
                "ErrorMessage": "[301000]-NO EXISTE NINGUNA COMANDA CON ESTE ID"
            })),
            _ => {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "ErrorMessage": "" }))
            }
        })
        .mount(&server)
        .await;
    let config = apuntar_al_mock(&pool, user_id, &server).await;

    BdpOrderPollerService::poll_pending(&pool, user_id, &config, &ServicioModoOperacion::new())
        .await
        .expect("el polling no revienta");

    assert_eq!(
        resultado_de(&pool, venta_id).await,
        "error",
        "una comanda inexistente no admite una modificación aplicada"
    );
}

/// La venta facturada no entra en ninguna lista del poller: sin la comprobación
/// de auditorías abiertas, el corte temprano saltaría el turno entero.
#[sqlx::test(migrations = "./migrations")]
async fn una_venta_facturada_con_auditoria_abierta_no_salta_el_turno(pool: PgPool) {
    let (user_id, _) = usuario(&pool).await;
    venta_facturada_con_modificacion_ambigua(&pool, user_id, 3.15).await;

    assert!(
        VentaRepository::list_bdp_pending(&pool, user_id)
            .await
            .expect("pendientes")
            .is_empty(),
        "una venta facturada no es «pendiente»"
    );
    assert!(
        VentaRepository::list_bdp_orphaned(&pool, user_id)
            .await
            .expect("huérfanas")
            .is_empty(),
        "sí tiene OrderId: no es huérfana"
    );

    let server = bdp_con_total(3.15).await;
    let config = apuntar_al_mock(&pool, user_id, &server).await;
    let cerradas =
        BdpOrderPollerService::poll_pending(&pool, user_id, &config, &ServicioModoOperacion::new())
            .await
            .expect("el polling no revienta");
    assert_eq!(cerradas, 1, "el turno no se puede saltar");
}
