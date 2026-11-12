// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/*! [D23b] La venta cuyo acuse se perdió tiene que volver a mirarse.
 *
 * Caso real reproducido en la 89: BDP crea la comanda y corta la conexión antes
 * de responder, así que Glory nunca recibe el `OrderId`. La auditoría queda
 * `ambiguo`, la venta `bdp_synced=false` y `bdp_order_id` nulo. Con eso, la venta
 * no entraba en `list_bdp_pending` (exige `bdp_synced=TRUE`) ni en
 * `list_bdp_orphaned` (exige `bdp_order_id`), y `poll_pending` salía por el corte
 * temprano sin consultar a BDP jamás: el aviso se perdía para siempre aunque
 * fuera localizable por `MarketplaceOrderId`.
 *
 * El test reproduce ese estado exacto y comprueba que el polling lo recupera. */

use glory_backend::models::ConfiguracionRestaurante;
use glory_backend::repositories::{ConfiguracionRepository, VentaRepository};
use glory_backend::services::{BdpOrderPollerService, BdpSyncService, ServicioModoOperacion};
use sqlx::PgPool;
use uuid::Uuid;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn usuario_con_bdp_roto(pool: &PgPool) -> (Uuid, ConfiguracionRestaurante) {
    let user_id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(format!("d23b-{user_id}@example.com"))
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("crear usuario de prueba");

    ConfiguracionRepository::obtener_o_crear(pool, user_id)
        .await
        .expect("crear configuración");
    /* Puerto cerrado a propósito: cualquier intento de red falla, que es
     * justo el «receptor inaccesible» del caso D23b. Las credenciales sí van
     * rellenas: son lo que `BdpWeblinkClient::ensure_configured` exige antes de
     * intentar el `Login`; vacías, el fallo sería de configuración y no de red,
     * y el test no probaría lo que dice probar. */
    sqlx::query(
        "UPDATE configuracion_restaurante \
         SET bdp_base_url = 'http://127.0.0.1:59999', \
             bdp_login = 'd23b', bdp_password = 'd23b', bdp_integrator_code = 'D23B', \
             modo_operacion = 'bdp', bdp_sync_mode = 'unidirectional', \
             bdp_sync_enabled = TRUE, bdp_auto_backup_before_write = TRUE \
         WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(pool)
    .await
    .expect("configurar BDP inalcanzable");

    let config = ConfiguracionRepository::obtener_o_crear(pool, user_id)
        .await
        .expect("releer configuración");
    (user_id, config)
}

/// Deja la venta en el estado del acuse perdido: comanda enviada, auditoría
/// abierta y sin `bdp_order_id` local.
async fn venta_con_acuse_perdido(pool: &PgPool, user_id: Uuid) -> Uuid {
    let venta_id = Uuid::new_v4();
    let marketplace_id = BdpSyncService::marketplace_order_id(venta_id);

    sqlx::query(
        "INSERT INTO ventas (id, user_id, fecha, turno, canal, metodo_pago, \
                             iva_porcentaje, importe_base, importe_iva, bdp_synced) \
         VALUES ($1, $2, CURRENT_DATE, 'noche', 'comedor', 'efectivo', 10, 10, 1, FALSE)",
    )
    .bind(venta_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("crear venta de prueba");

    sqlx::query(
        "INSERT INTO bdp_audit_log \
           (user_id, operacion, direccion, datos_enviados, resultado, \
            target_base_url, target_entity_type, target_entity_id) \
         VALUES ($1, 'create_order', 'glory_to_bdp', $2, 'ambiguo', \
                 'http://127.0.0.1:59999', 'venta', $3)",
    )
    .bind(user_id)
    .bind(serde_json::json!({ "MarketplaceOrderId": marketplace_id }))
    .bind(venta_id)
    .execute(pool)
    .await
    .expect("crear auditoría ambigua");
    venta_id
}

/// La venta con el acuse perdido es «localizable»: tiene una auditoría abierta
/// que guarda su `MarketplaceOrderId`, que es lo que se pregunta a BDP. Debe
/// aparecer en la lista nueva para que el polling no la salte.
#[sqlx::test(migrations = "./migrations")]
async fn la_venta_con_acuse_perdido_entra_en_la_lista_en_vuelo(pool: PgPool) {
    let (user_id, _) = usuario_con_bdp_roto(&pool).await;
    let venta_id = venta_con_acuse_perdido(&pool, user_id).await;

    let en_vuelo = VentaRepository::list_bdp_en_vuelo(&pool, user_id)
        .await
        .expect("listar ventas en vuelo");
    assert_eq!(en_vuelo.len(), 1, "la venta tiene que ser localizable");
    assert_eq!(en_vuelo[0].id, venta_id);

    /* Las otras dos listas siguen sin verla: por eso hacía falta esta. */
    assert!(VentaRepository::list_bdp_pending(&pool, user_id)
        .await
        .expect("pendientes")
        .is_empty());
    assert!(VentaRepository::list_bdp_orphaned(&pool, user_id)
        .await
        .expect("huérfanas")
        .is_empty());
}

/// El corte temprano de `poll_pending` ya no se lleva por delante el intento:
/// con el receptor aún caído, la venta se mira, el `GetOrder` falla y la
/// auditoría sigue abierta (no se da por recuperado lo que no se ha podido
/// comprobar).
#[sqlx::test(migrations = "./migrations")]
async fn el_polling_mira_la_venta_aunque_el_receptor_siga_caido(pool: PgPool) {
    let (user_id, config) = usuario_con_bdp_roto(&pool).await;
    let venta_id = venta_con_acuse_perdido(&pool, user_id).await;
    let servicio = ServicioModoOperacion::new();

    let _ = BdpOrderPollerService::poll_pending(&pool, user_id, &config, &servicio)
        .await
        .expect("el polling no debe reventar con el receptor caído");

    let (resultado,): (String,) = sqlx::query_as(
        "SELECT resultado FROM bdp_audit_log \
         WHERE user_id = $1 AND target_entity_id = $2 AND operacion = 'create_order'",
    )
    .bind(user_id)
    .bind(venta_id)
    .fetch_one(&pool)
    .await
    .expect("leer auditoría");
    assert_eq!(
        resultado, "ambiguo",
        "sin poder comprobar en BDP no se da por recuperado"
    );

    let venta = VentaRepository::find_by_id(&pool, venta_id, user_id)
        .await
        .expect("leer venta")
        .expect("la venta existe");
    assert!(!venta.bdp_synced, "sigue pendiente de confirmar");
}

/// El circuito entero: con el receptor restaurado, el polling encuentra la
/// comanda por `MarketplaceOrderId`, la recupera y lo **cuenta**. Si el contador
/// no suma, la pantalla dice «0 actualizadas» sobre una venta ya recuperada.
#[sqlx::test(migrations = "./migrations")]
async fn con_el_receptor_arriba_la_recupera_y_lo_cuenta(pool: PgPool) {
    /* BDP de mentira que contesta GetOrder con el OrderId de la comanda. */
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(|request: &wiremock::Request| {
            let cuerpo: serde_json::Value =
                serde_json::from_slice(&request.body).unwrap_or_default();
            match request.url.path() {
                "/Auth/Login" => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "ErrorMessage": "",
                    "AuthSession": { "Token": "token-bdp", "ExpiresIn_InSecconds": 3540 }
                })),
                "/API/Orders/Get" => {
                    let pedido = cuerpo
                        .get("OrderIdentifier")
                        .and_then(|i| i.get("MarketplaceOrderId"))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({
                        "Order": {
                            "OrderId": 4242,
                            "MarketplaceOrderId": pedido,
                            "Status": 1,
                            "Payments": []
                        },
                        "Status": 1,
                        "ErrorMessage": ""
                    }))
                }
                _ => ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "ErrorMessage": "" })),
            }
        })
        .mount(&server)
        .await;

    let (user_id, _) = usuario_con_bdp_roto(&pool).await;
    sqlx::query("UPDATE configuracion_restaurante SET bdp_base_url = $2 WHERE user_id = $1")
        .bind(user_id)
        .bind(server.uri())
        .execute(&pool)
        .await
        .expect("apuntar al BDP disponible");
    let config = ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("releer configuración");

    let venta_id = venta_con_acuse_perdido(&pool, user_id).await;
    let recuperadas =
        BdpOrderPollerService::poll_pending(&pool, user_id, &config, &ServicioModoOperacion::new())
            .await
            .expect("el polling tiene que poder reconciliar");

    assert_eq!(recuperadas, 1, "lo recuperado se cuenta");
    let venta = VentaRepository::find_by_id(&pool, venta_id, user_id)
        .await
        .expect("leer venta")
        .expect("la venta existe");
    assert!(
        venta.bdp_synced,
        "la comanda existe en BDP: queda recuperada"
    );
    assert_eq!(venta.bdp_order_id, Some(4242));

    let (resultado,): (String,) = sqlx::query_as(
        "SELECT resultado FROM bdp_audit_log \
         WHERE user_id = $1 AND target_entity_id = $2 AND operacion = 'create_order'",
    )
    .bind(user_id)
    .bind(venta_id)
    .fetch_one(&pool)
    .await
    .expect("leer auditoría");
    assert_eq!(resultado, "exito", "el guard deja de bloquear la venta");
}

/// Un 404 de `GetOrder` **no** es la respuesta de negocio «no la tengo»: BDP no
/// contesta así en su contrato (el simulador devuelve 200 con `ErrorMessage`).
/// Sin esa evidencia no se reenvía: la auditoría sigue abierta y la venta
/// pendiente.
#[sqlx::test(migrations = "./migrations")]
async fn un_404_de_get_order_no_autoriza_a_reenviar(pool: PgPool) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(|request: &wiremock::Request| match request.url.path() {
            "/Auth/Login" => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ErrorMessage": "",
                "AuthSession": { "Token": "token-bdp", "ExpiresIn_InSecconds": 3540 }
            })),
            _ => ResponseTemplate::new(404)
                .set_body_json(serde_json::json!({ "ErrorMessage": "comanda inexistente" })),
        })
        .mount(&server)
        .await;

    let (user_id, _) = usuario_con_bdp_roto(&pool).await;
    sqlx::query("UPDATE configuracion_restaurante SET bdp_base_url = $2 WHERE user_id = $1")
        .bind(user_id)
        .bind(server.uri())
        .execute(&pool)
        .await
        .expect("apuntar al BDP disponible");
    let config = ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("releer configuración");

    let venta_id = venta_con_acuse_perdido(&pool, user_id).await;
    BdpOrderPollerService::poll_pending(&pool, user_id, &config, &ServicioModoOperacion::new())
        .await
        .expect("el polling no revienta");

    let venta = VentaRepository::find_by_id(&pool, venta_id, user_id)
        .await
        .expect("leer venta")
        .expect("la venta existe");
    assert!(!venta.bdp_synced, "no consta en BDP: no se marca");
    let (resultado,): (String,) = sqlx::query_as(
        "SELECT resultado FROM bdp_audit_log \
         WHERE user_id = $1 AND target_entity_id = $2 AND operacion = 'create_order'",
    )
    .bind(user_id)
    .bind(venta_id)
    .fetch_one(&pool)
    .await
    .expect("leer auditoría");
    assert_eq!(resultado, "ambiguo");
}

/// [M21] El caso que pidió el cliente: BDP dice por `GetOrder` que no tiene la
/// comanda (`ErrorMessage` de negocio con `Health` vivo). Entonces se cierra la
/// auditoría muerta y se envía **una sola vez**; la venta sale del limbo.
#[sqlx::test(migrations = "./migrations")]
async fn si_bdp_dice_que_no_la_tiene_se_reenvia_una_vez(pool: PgPool) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(|request: &wiremock::Request| {
            let cuerpo: serde_json::Value =
                serde_json::from_slice(&request.body).unwrap_or_default();
            match request.url.path() {
                "/Auth/Login" => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "ErrorMessage": "",
                    "AuthSession": { "Token": "token-bdp", "ExpiresIn_InSecconds": 3540 }
                })),
                "/Service/Health" => {
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({ "IsAlive": true }))
                }
                /* La comanda no está: error de negocio, como el TPV real. */
                "/API/Orders/Get" => ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "ErrorMessage": "comanda inexistente" })),
                "/API/Orders/Create" => {
                    let pedido = cuerpo
                        .get("Order")
                        .and_then(|o| o.get("MarketplaceOrderId"))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({
                        "OrderId": 5555,
                        "Order": { "OrderId": 5555, "MarketplaceOrderId": pedido },
                        "ErrorMessage": ""
                    }))
                }
                _ => ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "ErrorMessage": "" })),
            }
        })
        .mount(&server)
        .await;

    let (user_id, _) = usuario_con_bdp_roto(&pool).await;
    /* Modo `automatic` con la categoría de comandas encendida: sin esto el
     * reenvío que se prueba aquí se quedaría en el armado y no llegaría a BDP. */
    sqlx::query(
        "UPDATE configuracion_restaurante SET bdp_base_url = $2, bdp_sync_mode = 'automatic' \
         WHERE user_id = $1",
    )
    .bind(user_id)
    .bind(server.uri())
    .execute(&pool)
    .await
    .expect("apuntar al BDP disponible");
    glory_backend::services::fijar_permiso(&pool, user_id, "comandas", true)
        .await
        .expect("encender la categoría de comandas");
    let config = ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("releer configuración");

    let venta_id = venta_con_acuse_perdido(&pool, user_id).await;
    let recuperadas =
        BdpOrderPollerService::poll_pending(&pool, user_id, &config, &ServicioModoOperacion::new())
            .await
            .expect("el polling no revienta");
    assert_eq!(recuperadas, 1, "la venta se reabrió y se envió");

    let venta = VentaRepository::find_by_id(&pool, venta_id, user_id)
        .await
        .expect("leer venta")
        .expect("la venta existe");
    assert!(venta.bdp_synced, "el reenvío confirmó la comanda");
    assert_eq!(venta.bdp_order_id, Some(5555));

    /* La auditoría vieja queda cerrada como `error` (no se reenvió «a ciegas ni
     * dos veces»: BDP dijo que no la tenía) y la nueva como `exito`. */
    let abiertas: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bdp_audit_log \
         WHERE user_id = $1 AND target_entity_id = $2 AND operacion = 'create_order' \
           AND resultado IN ('pendiente', 'ambiguo')",
    )
    .bind(user_id)
    .bind(venta_id)
    .fetch_one(&pool)
    .await
    .expect("contar auditorías abiertas");
    assert_eq!(abiertas, 0, "ninguna auditoría queda bloqueando la venta");

    let envios = server
        .received_requests()
        .await
        .iter()
        .flatten()
        .filter(|req| req.url.path() == "/API/Orders/Create")
        .count();
    assert_eq!(envios, 1, "se envía una sola vez");
}

/// El límite del reenvío: con el TPV caído no hay evidencia de nada, así que la
/// auditoría sigue abierta y **no** se envía. Reenviar a ciegas es justo lo que
/// esta pieza evita.
#[sqlx::test(migrations = "./migrations")]
async fn con_bdp_caido_no_se_reenvia(pool: PgPool) {
    let (user_id, config) = usuario_con_bdp_roto(&pool).await;
    let venta_id = venta_con_acuse_perdido(&pool, user_id).await;

    let recuperadas =
        BdpOrderPollerService::poll_pending(&pool, user_id, &config, &ServicioModoOperacion::new())
            .await
            .expect("el polling no revienta con el receptor caído");
    assert_eq!(recuperadas, 0);

    let (resultado,): (String,) = sqlx::query_as(
        "SELECT resultado FROM bdp_audit_log \
         WHERE user_id = $1 AND target_entity_id = $2 AND operacion = 'create_order'",
    )
    .bind(user_id)
    .bind(venta_id)
    .fetch_one(&pool)
    .await
    .expect("leer auditoría");
    assert_eq!(resultado, "ambiguo", "sin respuesta de BDP no se decide");
    let venta = VentaRepository::find_by_id(&pool, venta_id, user_id)
        .await
        .expect("leer venta")
        .expect("la venta existe");
    assert!(!venta.bdp_synced);
}

/// El otro borde: BDP contesta 200 sin comanda (`ErrorMessage` vacío y sin
/// `OrderId`). No es un error de red ni un 404: es «no la encuentro».
/// Se trata igual: la auditoría sigue abierta y la venta pendiente.
#[sqlx::test(migrations = "./migrations")]
async fn si_bdp_contesta_sin_comanda_no_se_da_por_recuperada(pool: PgPool) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(|request: &wiremock::Request| match request.url.path() {
            "/Auth/Login" => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ErrorMessage": "",
                "AuthSession": { "Token": "token-bdp", "ExpiresIn_InSecconds": 3540 }
            })),
            _ => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Order": null,
                "Status": 0,
                "ErrorMessage": ""
            })),
        })
        .mount(&server)
        .await;

    let (user_id, _) = usuario_con_bdp_roto(&pool).await;
    sqlx::query("UPDATE configuracion_restaurante SET bdp_base_url = $2 WHERE user_id = $1")
        .bind(user_id)
        .bind(server.uri())
        .execute(&pool)
        .await
        .expect("apuntar al BDP disponible");
    let config = ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("releer configuración");

    let venta_id = venta_con_acuse_perdido(&pool, user_id).await;
    let recuperadas =
        BdpOrderPollerService::poll_pending(&pool, user_id, &config, &ServicioModoOperacion::new())
            .await
            .expect("el polling no revienta");

    assert_eq!(
        recuperadas, 0,
        "sin OrderId no hay nada que dar por recuperado"
    );
    let venta = VentaRepository::find_by_id(&pool, venta_id, user_id)
        .await
        .expect("leer venta")
        .expect("la venta existe");
    assert!(!venta.bdp_synced);
    assert_eq!(venta.bdp_order_id, None);
}

/// [D10] El sondeo ve la misma anulación que el aviso: si BDP dice `cancelled`,
/// la venta queda anulada también por esta vía. Sin esto, la vía de los avisos y
/// la del polling contarían cosas distintas de la misma comanda.
#[sqlx::test(migrations = "./migrations")]
async fn el_sondeo_anula_la_venta_que_bdp_da_por_cancelada(pool: PgPool) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(|request: &wiremock::Request| {
            let cuerpo: serde_json::Value =
                serde_json::from_slice(&request.body).unwrap_or_default();
            match request.url.path() {
                "/Auth/Login" => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "ErrorMessage": "",
                    "AuthSession": { "Token": "token-bdp", "ExpiresIn_InSecconds": 3540 }
                })),
                "/API/Orders/Get" => {
                    let pedido = cuerpo
                        .get("OrderIdentifier")
                        .and_then(|i| i.get("MarketplaceOrderId"))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({
                        "Order": {
                            "OrderId": 4343,
                            "MarketplaceOrderId": pedido,
                            "Status": 2,
                            "Payments": []
                        },
                        "Status": 2,
                        "ErrorMessage": ""
                    }))
                }
                _ => ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "ErrorMessage": "" })),
            }
        })
        .mount(&server)
        .await;

    let (user_id, _) = usuario_con_bdp_roto(&pool).await;
    sqlx::query("UPDATE configuracion_restaurante SET bdp_base_url = $2 WHERE user_id = $1")
        .bind(user_id)
        .bind(server.uri())
        .execute(&pool)
        .await
        .expect("apuntar al BDP disponible");
    let config = ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("releer configuración");

    let venta_id = venta_con_acuse_perdido(&pool, user_id).await;
    let recuperadas =
        BdpOrderPollerService::poll_pending(&pool, user_id, &config, &ServicioModoOperacion::new())
            .await
            .expect("el polling no revienta");

    /* Se recupera y se cuenta: la auditoría cierra como éxito. */
    assert_eq!(recuperadas, 1);
    let venta = VentaRepository::find_by_id(&pool, venta_id, user_id)
        .await
        .expect("leer venta")
        .expect("la venta existe");
    assert_eq!(venta.bdp_order_status.as_deref(), Some("cancelled"));
    assert!(venta.anulada, "cancelada en BDP es anulada en la web");
    assert!(venta.anulada_at.is_some(), "la anulación deja fecha");
    assert!(
        venta.anulacion_motivo.is_some(),
        "la anulación deja motivo legible"
    );
}
