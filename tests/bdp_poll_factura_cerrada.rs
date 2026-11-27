// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/*! [21AB-5g] Una auditoría de facturación no puede quedarse abierta para siempre.
 *
 * `ensure_no_unresolved` bloquea toda escritura de la venta mientras tenga una
 * auditoría `pendiente`/`ambiguo`. `reconcile_invoice` leía el `Status` dentro
 * de `Order`, pero el demo contesta `{"Order": null, "Status": 3}` cuando la
 * comanda ya está facturada (medido el 27/11 en las comandas 43 y 47) y
 * `[301000]` cuando no existe (la 85). El primer caso daba `-1` y el segundo
 * moría en la rama de error sin cerrar nada: cuatro auditorías bloqueadas.
 *
 * Las cuatro salidas se prueban aquí, con el TPV de mentira contestando lo que
 * el real contestó.
 */

use glory_backend::models::ConfiguracionRestaurante;
use glory_backend::repositories::{ConfiguracionRepository, VentaRepository};
use glory_backend::services::{BdpOrderPollerService, ServicioModoOperacion};
use sqlx::PgPool;
use uuid::Uuid;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// Cuenta en modo BDP. El mock se monta después, porque su puerto no se conoce.
async fn usuario(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(format!("21ab5g-{user_id}@example.com"))
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
    user_id
}

/// Venta aceptada por el TPV con la factura en vuelo: la auditoría quedó
/// `pendiente` y el proceso que la abrió ya no existe.
async fn venta_con_factura_pendiente(pool: &PgPool, user_id: Uuid) -> (Uuid, i64) {
    let venta_id = Uuid::new_v4();
    let order_id = 43;
    sqlx::query(
        "INSERT INTO ventas (id, user_id, fecha, turno, canal, metodo_pago, \
                             iva_porcentaje, importe_base, importe_iva, \
                             bdp_synced, bdp_order_id, bdp_order_status) \
         VALUES ($1, $2, CURRENT_DATE, 'mediodia', 'comedor', 'efectivo', \
                 10, 10, 1, TRUE, $3, 'accepted')",
    )
    .bind(venta_id)
    .bind(user_id)
    .bind(order_id)
    .execute(pool)
    .await
    .expect("crear venta aceptada");

    sqlx::query(
        "INSERT INTO bdp_audit_log \
           (user_id, operacion, direccion, datos_enviados, resultado, \
            target_base_url, target_entity_type, target_entity_id) \
         VALUES ($1, 'invoice', 'glory_to_bdp', $2, 'pendiente', \
                 'http://127.0.0.1:59999', 'venta', $3)",
    )
    .bind(user_id)
    .bind(serde_json::json!({ "venta_id": venta_id, "order_id": order_id }))
    .bind(venta_id)
    .execute(pool)
    .await
    .expect("crear auditoría de factura");

    (venta_id, order_id)
}

/// TPV de mentira: contesta `GetOrder` con lo que se le diga y `Health` con el
/// estado que se le pida. `order_response` es el cuerpo literal del demo.
async fn bdp(order_response: serde_json::Value, vivo: bool) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(move |request: &Request| match request.url.path() {
            "/Auth/Login" => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ErrorMessage": "",
                "AuthSession": { "Token": "token-bdp", "ExpiresIn_InSecconds": 3540 }
            })),
            "/Service/Health" => {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "IsAlive": vivo }))
            }
            "/API/Orders/Get" => ResponseTemplate::new(200).set_body_json(order_response.clone()),
            _ => {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "ErrorMessage": "" }))
            }
        })
        .mount(&server)
        .await;
    server
}

async fn apuntar_al_tpv(
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

async fn auditar(pool: &PgPool, user_id: Uuid, venta_id: Uuid) -> (String, Option<String>) {
    sqlx::query_as(
        "SELECT resultado, error_mensaje FROM bdp_audit_log \
         WHERE user_id = $1 AND target_entity_id = $2 AND operacion = 'invoice'",
    )
    .bind(user_id)
    .bind(venta_id)
    .fetch_one(pool)
    .await
    .expect("leer auditoría de factura")
}

async fn facturada(pool: &PgPool, venta_id: Uuid) -> bool {
    sqlx::query_scalar("SELECT bdp_invoiced FROM ventas WHERE id = $1")
        .bind(venta_id)
        .fetch_one(pool)
        .await
        .expect("leer estado de facturación")
}

/// El caso medido en el demo: `{"Order": null, "Status": 3}`. La comanda está
/// facturada aunque `Order` venga nulo; se cierra en `exito` y la venta lo
/// refleja.
#[sqlx::test(migrations = "./migrations")]
async fn la_factura_emitida_cierra_en_exito_aunque_order_venga_nulo(pool: PgPool) {
    let user_id = usuario(&pool).await;
    let (venta_id, _) = venta_con_factura_pendiente(&pool, user_id).await;
    let server = bdp(
        serde_json::json!({ "Order": null, "Status": 3, "ErrorMessage": "" }),
        true,
    )
    .await;
    let config = apuntar_al_tpv(&pool, user_id, &server).await;

    BdpOrderPollerService::poll_pending(&pool, user_id, &config, &ServicioModoOperacion::new())
        .await
        .expect("el polling no revienta");

    assert_eq!(auditar(&pool, user_id, venta_id).await.0, "exito");
    assert!(facturada(&pool, venta_id).await, "la venta queda facturada");
}

/// El caso de la facturada que ya no existe: `[301000]` con el TPV vivo. No
/// pudo emitirse factura sobre una comanda inexistente; se cierra en `error`.
#[sqlx::test(migrations = "./migrations")]
async fn la_comanda_inexistente_cierra_en_error_con_el_tpv_vivo(pool: PgPool) {
    let user_id = usuario(&pool).await;
    let (venta_id, _) = venta_con_factura_pendiente(&pool, user_id).await;
    let server = bdp(
        serde_json::json!({
            "Order": null, "Status": 0,
            "ErrorMessage": "[301000]-NO EXISTE NINGUNA COMANDA CON ESTE ID"
        }),
        true,
    )
    .await;
    let config = apuntar_al_tpv(&pool, user_id, &server).await;

    BdpOrderPollerService::poll_pending(&pool, user_id, &config, &ServicioModoOperacion::new())
        .await
        .expect("el polling no revienta");

    let (resultado, mensaje) = auditar(&pool, user_id, venta_id).await;
    assert_eq!(resultado, "error");
    let mensaje = mensaje.unwrap_or_default();
    assert!(mensaje.contains("no tiene la comanda"), "motivo: {mensaje}");
    assert!(
        mensaje.contains("la factura no se emitió"),
        "motivo: {mensaje}"
    );
    assert!(!facturada(&pool, venta_id).await);
}

/// El mismo `[301000]` con el TPV no sano no prueba nada: el texto puede venir
/// de una sesión o una configuración rota. La auditoría sigue abierta.
#[sqlx::test(migrations = "./migrations")]
async fn el_mismo_error_con_el_tpv_caido_no_cierra_nada(pool: PgPool) {
    let user_id = usuario(&pool).await;
    let (venta_id, _) = venta_con_factura_pendiente(&pool, user_id).await;
    let server = bdp(
        serde_json::json!({
            "Order": null, "Status": 0,
            "ErrorMessage": "[301000]-NO EXISTE NINGUNA COMANDA CON ESTE ID"
        }),
        false,
    )
    .await;
    let config = apuntar_al_tpv(&pool, user_id, &server).await;

    BdpOrderPollerService::poll_pending(&pool, user_id, &config, &ServicioModoOperacion::new())
        .await
        .expect("el polling no revienta");

    assert_eq!(
        auditar(&pool, user_id, venta_id).await.0,
        "pendiente",
        "sin testigo del TPV no se decide"
    );
}

/// La comanda existe y está abierta (el caso de la 40, medida el 27/11): la
/// factura no se emitió. Se cierra en `error` y la venta deja de estar bloqueada.
#[sqlx::test(migrations = "./migrations")]
async fn la_comanda_abierta_cierra_en_error_sin_facturar(pool: PgPool) {
    let user_id = usuario(&pool).await;
    let (venta_id, order_id) = venta_con_factura_pendiente(&pool, user_id).await;
    let server = bdp(
        serde_json::json!({
            "Order": { "OrderId": order_id, "Status": 1, "Total": 3.15, "InvoiceNumber": null },
            "Status": 1, "ErrorMessage": ""
        }),
        true,
    )
    .await;
    let config = apuntar_al_tpv(&pool, user_id, &server).await;

    BdpOrderPollerService::poll_pending(&pool, user_id, &config, &ServicioModoOperacion::new())
        .await
        .expect("el polling no revienta");

    let (resultado, mensaje) = auditar(&pool, user_id, venta_id).await;
    assert_eq!(resultado, "error");
    let mensaje = mensaje.unwrap_or_default();
    assert!(mensaje.contains("no está facturada"), "motivo: {mensaje}");
    assert!(!facturada(&pool, venta_id).await);
}

/// El cobro ambiguo sobre una comanda inexistente se cierra igual: la rama de
/// `add_payment` comparte predicado con la factura.
#[sqlx::test(migrations = "./migrations")]
async fn el_cobro_sobre_comanda_inexistente_cierra_en_error(pool: PgPool) {
    let user_id = usuario(&pool).await;
    let (venta_id, _) = venta_con_factura_pendiente(&pool, user_id).await;
    sqlx::query(
        "INSERT INTO bdp_audit_log \
           (user_id, operacion, direccion, datos_enviados, resultado, \
            target_base_url, target_entity_type, target_entity_id) \
         VALUES ($1, 'add_payment', 'glory_to_bdp', $2, 'ambiguo', \
                 'http://127.0.0.1:59999', 'venta', $3)",
    )
    .bind(user_id)
    .bind(serde_json::json!({ "venta_id": venta_id, "tender_id": 1, "amount": 3.15 }))
    .bind(venta_id)
    .execute(&pool)
    .await
    .expect("crear auditoría de cobro");
    let server = bdp(
        serde_json::json!({
            "Order": null, "Status": 0,
            "ErrorMessage": "[301000]-NO EXISTE NINGUNA COMANDA CON ESTE ID"
        }),
        true,
    )
    .await;
    let config = apuntar_al_tpv(&pool, user_id, &server).await;

    BdpOrderPollerService::poll_pending(&pool, user_id, &config, &ServicioModoOperacion::new())
        .await
        .expect("el polling no revienta");

    let (resultado, mensaje): (String, Option<String>) = sqlx::query_as(
        "SELECT resultado, error_mensaje FROM bdp_audit_log \
         WHERE user_id = $1 AND target_entity_id = $2 AND operacion = 'add_payment'",
    )
    .bind(user_id)
    .bind(venta_id)
    .fetch_one(&pool)
    .await
    .expect("leer auditoría de cobro");
    assert_eq!(resultado, "error");
    assert!(mensaje
        .unwrap_or_default()
        .contains("el cobro no se aplicó"));
}

/// Testigo del mismo barrido: la venta sí entra en la lista de pendientes, así
/// que los cierres de arriba vienen de la reconciliación y no de otro camino.
#[sqlx::test(migrations = "./migrations")]
async fn la_venta_del_caso_esta_en_la_lista_de_pendientes(pool: PgPool) {
    let user_id = usuario(&pool).await;
    let (venta_id, _) = venta_con_factura_pendiente(&pool, user_id).await;
    let pendientes = VentaRepository::list_bdp_pending(&pool, user_id)
        .await
        .expect("pendientes");
    assert!(
        pendientes.iter().any(|venta| venta.id == venta_id),
        "si no estuviera, la prueba mediría otra cosa"
    );
}
