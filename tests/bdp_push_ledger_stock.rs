// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
/* [D26g] Respuesta perdida en stock. La regla se comprueba de punta a punta:
 * antes del HTTP se guarda S, V y S+V; si la respuesta no llega, se relee
 * `GetStock` y solo se reintenta cuando la lectura sigue en S.
 *
 * El proveedor falso hace de BDP y guarda el stock en memoria: la escritura de
 * regularización sí lo modifica, así que una relectura miente si el reenvío es
 * a ciegas. Es la prueba del defecto, no un doble de conveniencia. */
use glory_backend::models::ActualizarConfiguracionRequest;
use glory_backend::services::bdp_push::{
    BdpPushService, DecisionRevisionStock, DOMINIO_STOCK, ESTADO_PENDIENTE, ESTADO_SINCRONIZADO,
};
use glory_backend::services::{listar_revisiones_stock, resolver_revision_stock};
use glory_backend::services::{BdpBackupService, BdpPushFlushService, ConfiguracionService};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

/// BDP de mentira: stock por artículo y un único permiso para cortar la
/// respuesta de la próxima escritura (`cortar`), que es el fallo que se prueba.
/* Los candados van en `Arc` para clonarlo: una copia respondiendo en el mock y
 * la original en el test mirando el mismo stock. */
#[derive(Clone, Default)]
struct BdpDeMentira {
    stock: Arc<Mutex<std::collections::HashMap<i64, f64>>>,
    cortar: Arc<Mutex<bool>>,
    /* BDP que se cae sin escribir: la respuesta se pierde y el stock no cambia. */
    no_aplicar: Arc<Mutex<bool>>,
    /* Movimiento ajeno (una venta de caja) que entra con el nuestro. */
    sumar_extra: Arc<Mutex<f64>>,
}

impl BdpDeMentira {
    fn con_stock(stock: f64) -> Self {
        let yo = Self::default();
        yo.stock.lock().unwrap().insert(1001, stock);
        yo
    }

    fn stock_de(&self) -> f64 {
        *self.stock.lock().unwrap().get(&1001).unwrap_or(&0.0)
    }

    fn cortar_la_proxima(&self) {
        *self.cortar.lock().unwrap() = true;
    }

    fn caerse_sin_escribir(&self) {
        *self.cortar.lock().unwrap() = true;
        *self.no_aplicar.lock().unwrap() = true;
    }

    fn entra_una_venta(&self, unidades: f64) {
        *self.cortar.lock().unwrap() = true;
        *self.sumar_extra.lock().unwrap() = unidades;
    }
}

impl Respond for BdpDeMentira {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let cuerpo: Value = serde_json::from_slice(&request.body).unwrap_or(Value::Null);
        match request.url.path() {
            "/Auth/Login" => ResponseTemplate::new(200).set_body_json(json!({
                "ErrorMessage": "",
                "AuthSession": { "Token": "token-bdp", "ExpiresIn_InSecconds": 3540 }
            })),
            "/API/Warehouse/GetListStock" => {
                let almacen = cuerpo.get("Store").and_then(Value::as_i64).unwrap_or(1);
                let lectura = self.stock_de();
                ResponseTemplate::new(200).set_body_json(json!({
                    "Stock": [{ "Article": 1001, "Altern": 0, "Units": lectura,
                                "ErrorMessage": "", "Store": almacen }],
                    "ErrorMessage": ""
                }))
            }
            "/API/Warehouse/GetStock" => {
                let articulo = cuerpo.get("Article").and_then(Value::as_i64).unwrap_or(0);
                let lectura = if articulo == 1001 {
                    self.stock_de()
                } else {
                    0.0
                };
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "Stock": lectura, "ErrorMessage": "" }))
            }
            "/API/Warehouse/Regularizations" => {
                let unidades = match cuerpo.get("Units") {
                    Some(Value::String(texto)) => texto.parse::<f64>().unwrap_or(0.0),
                    Some(otro) => otro.as_f64().unwrap_or(0.0),
                    None => 0.0,
                };
                let sin_escribir = std::mem::take(&mut *self.no_aplicar.lock().unwrap());
                let ajeno = std::mem::take(&mut *self.sumar_extra.lock().unwrap());
                if !sin_escribir {
                    *self.stock.lock().unwrap().entry(1001).or_insert(0.0) += unidades;
                }
                *self.stock.lock().unwrap().entry(1001).or_insert(0.0) += ajeno;
                let cortar = std::mem::take(&mut *self.cortar.lock().unwrap());
                if cortar {
                    /* Sin cuerpo y cerrando la conexión: el cliente recibe un
                     * error de lectura con estado 200, no un rechazo. */
                    let mut respuesta = ResponseTemplate::new(200);
                    respuesta = respuesta.set_body_raw(Vec::new(), "application/json");
                    return respuesta;
                }
                ResponseTemplate::new(200).set_body_json(json!({ "ErrorMessage": "" }))
            }
            _ => ResponseTemplate::new(404)
                .set_body_json(json!({ "ErrorMessage": "ruta no simulada" })),
        }
    }
}

/// Config del usuario en modo BDP, snapshot completo vigente y cuentas de stock
/// en cero: lo mínimo para que el armado deje pasar la escritura.
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

fn ajuste(unidades: &str) -> Value {
    json!({
        "Article": 1001, "sD1": "", "sD2": "", "sD3": "",
        "Units": unidades, "CodReg": 1, "Store": 1, "DateReg": "2026-10-21"
    })
}

async fn encolar_ajuste(pool: &PgPool, user: Uuid, unidades: &str) {
    BdpPushService::acumular_regularizacion(
        pool,
        user,
        "1001:1",
        unidades.parse().unwrap(),
        &ajuste(unidades),
    )
    .await
    .unwrap();
}

async fn fila(pool: &PgPool, user: Uuid) -> (String, i32, Option<String>) {
    sqlx::query_as(
        "SELECT estado, reintentos, ultimo_error FROM bdp_push_pendientes \
         WHERE user_id = $1 AND dominio = $2 AND entidad_id = '1001:1'",
    )
    .bind(user)
    .bind(DOMINIO_STOCK)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn libro(pool: &PgPool, user: Uuid) -> Vec<(String, String, Option<String>)> {
    sqlx::query_as(
        "SELECT estado, articulo_codigo, nota FROM bdp_stock_ledger \
         WHERE user_id = $1 ORDER BY creado_en, articulo_codigo",
    )
    .bind(user)
    .fetch_all(pool)
    .await
    .unwrap()
}

/// S+V tras la respuesta perdida: el movimiento sí se aplicó. No se reintenta,
/// la fila queda sincronizada y el stock no se mueve dos veces.
#[sqlx::test(migrations = "./migrations")]
async fn respuesta_perdida_con_s_mas_v_no_se_reintenta(pool: PgPool) {
    let bdp = BdpDeMentira::con_stock(10.0);
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(bdp.clone())
        .mount(&server)
        .await;
    let user = usuario_listo(&pool, &server.uri()).await;

    bdp.cortar_la_proxima();
    encolar_ajuste(&pool, user, "2").await;
    let resumen = BdpPushFlushService::flush(&pool, user, true).await.unwrap();

    assert_eq!(
        resumen.sincronizados, 1,
        "el libro vio S+V: el envío está hecho"
    );
    assert_eq!(resumen.errores, 0);
    assert_eq!(bdp.stock_de(), 12.0, "un solo movimiento de +2, no dos");
    let (estado, reintentos, nota) = fila(&pool, user).await;
    assert_eq!(estado, ESTADO_SINCRONIZADO);
    assert_eq!(reintentos, 0, "no se gasta reintento cuando ya se aplicó");
    assert!(nota.unwrap_or_default().contains("S+V"));
    let filas = libro(&pool, user).await;
    assert_eq!(filas.len(), 1);
    assert_eq!(filas[0].0, "confirmado");
    assert_eq!(bdp.stock_de(), 12.0, "la relectura no reenvía nada");
}

/// Se ve S: no llegó. Un único reintento controlado, y el segundo fallo ya no
/// se reintenta.
#[sqlx::test(migrations = "./migrations")]
async fn ver_s_reintenta_una_sola_vez(pool: PgPool) {
    let bdp = BdpDeMentira::con_stock(10.0);
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(bdp.clone())
        .mount(&server)
        .await;
    let user = usuario_listo(&pool, &server.uri()).await;

    /* El BDP se cae sin escribir y sin responder: el stock se queda en S. */
    bdp.caerse_sin_escribir();
    encolar_ajuste(&pool, user, "2").await;
    let resumen = BdpPushFlushService::flush(&pool, user, true).await.unwrap();

    assert_eq!(resumen.errores, 1);
    assert_eq!(resumen.sincronizados, 0);
    let (estado, reintentos, nota) = fila(&pool, user).await;
    assert_eq!(estado, "error");
    assert_eq!(reintentos, 1, "un reintento contabilizado, no un bucle");
    assert!(nota.unwrap_or_default().contains("sigue en S"));
    assert_eq!(libro(&pool, user).await[0].0, "sin_movimiento");
    assert_eq!(bdp.stock_de(), 10.0);

    /* Segundo envío con la misma respuesta perdida: ya gastó su reintento. */
    bdp.caerse_sin_escribir();
    let resumen = BdpPushFlushService::flush(&pool, user, true).await.unwrap();
    assert_eq!(resumen.errores, 1);
    /* El segundo envío lee S y también pierde la respuesta en S: como ya hay un
     * cierre sin movimiento de este envío, no se reintenta otra vez. */
    let filas = libro(&pool, user).await;
    assert_eq!(
        filas.len(),
        2,
        "un libro por intento, no se pisa el anterior"
    );
    assert_eq!(filas[0].0, "sin_movimiento");
    assert_eq!(filas[1].0, "requiere_revision");
    assert!(filas[1]
        .2
        .clone()
        .unwrap_or_default()
        .contains("ya reintentó"));
    let (_, _, nota) = fila(&pool, user).await;
    assert!(nota.unwrap_or_default().contains("ya reintentó"));
}

/// Valor que no es ni S ni S+V: no se puede atribuir (una venta de caja también
/// mueve stock). Se para a revisión y no se reenvía.
#[sqlx::test(migrations = "./migrations")]
async fn otro_valor_para_a_revision_sin_reintento(pool: PgPool) {
    let bdp = BdpDeMentira::con_stock(10.0);
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(bdp.clone())
        .mount(&server)
        .await;
    let user = usuario_listo(&pool, &server.uri()).await;

    /* Una venta de caja mueve el stock a la vez: la lectura posterior no se
     * puede atribuir a este envío (10 + 2 + 30 = 42 ≠ S+V = 12). */
    bdp.entra_una_venta(30.0);
    encolar_ajuste(&pool, user, "2").await;
    let resumen = BdpPushFlushService::flush(&pool, user, true).await.unwrap();

    assert_eq!(resumen.errores, 1);
    let (estado, reintentos, nota) = fila(&pool, user).await;
    assert_eq!(
        estado, "error",
        "no se reintenta lo que no se puede atribuir"
    );
    assert_eq!(reintentos, 0, "parar no gasta reintentos automáticos");
    assert!(nota.unwrap_or_default().contains("no se reintenta"));
    assert_eq!(libro(&pool, user).await[0].0, "requiere_revision");
    assert_eq!(
        bdp.stock_de(),
        42.0,
        "no se reenvió: solo el movimiento ajeno"
    );
}

/// El bloqueo por artículo y almacén: mientras el libro tenga una fila
/// pendiente, no se abre otra del mismo par y no se envía nada.
#[sqlx::test(migrations = "./migrations")]
async fn el_bloqueo_impide_dos_movimientos_del_mismo_par(pool: PgPool) {
    let bdp = BdpDeMentira::con_stock(10.0);
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(bdp.clone())
        .mount(&server)
        .await;
    let user = usuario_listo(&pool, &server.uri()).await;

    sqlx::query(
        "INSERT INTO bdp_stock_ledger \
         (id, user_id, articulo_codigo, almacen, operacion, variacion, stock_previo, \
          stock_esperado, id_local, origen, documento_id) \
         VALUES ($1, $2, '1001', 1, 'regularizar', 1, 10, 11, 'otra-operacion', 'web', $1)",
    )
    .bind(Uuid::new_v4())
    .bind(user)
    .execute(&pool)
    .await
    .unwrap();

    encolar_ajuste(&pool, user, "2").await;
    let resumen = BdpPushFlushService::flush(&pool, user, true).await.unwrap();

    assert_eq!(resumen.errores, 1, "el par bloqueado no se envía");
    assert_eq!(bdp.stock_de(), 10.0, "no hubo movimiento");
    let (_, _, nota) = fila(&pool, user).await;
    assert!(
        nota.unwrap_or_default()
            .contains("otra operación de stock pendiente"),
        "el aviso dice qué artículo y almacén están ocupados"
    );
}

/// El dictamen humano destraba lo que el libro paró. Sin él, la auditoría se
/// quedaba `ambiguo` y ese artículo no admitía ninguna escritura más: ni un
/// reintento ni un ajuste nuevo.
#[sqlx::test(migrations = "./migrations")]
async fn la_revision_manual_destraba_el_articulo(pool: PgPool) {
    let bdp = BdpDeMentira::con_stock(10.0);
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(bdp.clone())
        .mount(&server)
        .await;
    let user = usuario_listo(&pool, &server.uri()).await;

    /* Se pierde la respuesta y una venta deja el stock en un valor que no es ni
     * S ni S+V: se para a revisión. */
    bdp.entra_una_venta(30.0);
    encolar_ajuste(&pool, user, "2").await;
    BdpPushFlushService::flush(&pool, user, true).await.unwrap();

    let revisiones = listar_revisiones_stock(&pool, user).await.unwrap();
    assert_eq!(revisiones.len(), 1);
    assert_eq!(
        revisiones[0].stock_posterior,
        Some("42".parse().unwrap()),
        "la lectura que justifica la revisión"
    );

    /* Con la operación ambigua, cualquier envío nuevo del mismo par se bloquea. */
    encolar_ajuste(&pool, user, "2").await;
    let resumen = BdpPushFlushService::flush(&pool, user, true).await.unwrap();
    assert_eq!(resumen.errores, 1, "el guard sigue bloqueando sin dictamen");

    /* La persona mira el TPV: el movimiento sí se aplicó. */
    let resolucion = resolver_revision_stock(
        &pool,
        user,
        &revisiones[0].id_local,
        DecisionRevisionStock::Aplicado,
        "comprobado en el TPV: el movimiento está",
    )
    .await
    .unwrap();
    assert!(resolucion.aplicado);
    /* La cola sumó otro +2 mientras esperaba dictamen: cerrada la parte que ya
     * consta, el resto sigue vivo en vez de perderse. */
    assert_eq!(resolucion.estado_cola, ESTADO_PENDIENTE);
    assert!(listar_revisiones_stock(&pool, user)
        .await
        .unwrap()
        .is_empty());
    let restante: (String, String) = sqlx::query_as(
        "SELECT payload_json ->> 'Units', estado FROM bdp_push_pendientes WHERE user_id = $1",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        restante.0.parse::<f64>().unwrap(),
        2.0,
        "lo aplicado se descuenta, no se duplica"
    );
    assert_eq!(restante.1, ESTADO_PENDIENTE);

    /* Y el artículo vuelve a admitir escrituras: el resto se envía. */
    let resumen = BdpPushFlushService::flush(&pool, user, true).await.unwrap();
    assert_eq!(resumen.sincronizados, 1, "el guard ya deja pasar");
    assert_eq!(bdp.stock_de(), 44.0, "42 del BDP más el resto sin aplicar");
}

/// Si el dictamen es «no se aplicó», la cola vuelve a `pendiente` con los
/// reintentos a cero: es un envío nuevo, no un reintento automático más.
#[sqlx::test(migrations = "./migrations")]
async fn la_revision_no_aplicada_reencola_sin_reintentos(pool: PgPool) {
    let bdp = BdpDeMentira::con_stock(10.0);
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(bdp.clone())
        .mount(&server)
        .await;
    let user = usuario_listo(&pool, &server.uri()).await;

    bdp.entra_una_venta(30.0);
    encolar_ajuste(&pool, user, "2").await;
    BdpPushFlushService::flush(&pool, user, true).await.unwrap();
    /* El envío puede haber gastado reintentos: el dictamen humano los perdona. */
    sqlx::query("UPDATE bdp_push_pendientes SET reintentos = 4 WHERE user_id = $1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();

    let id_local = listar_revisiones_stock(&pool, user).await.unwrap()[0]
        .id_local
        .clone();
    let resolucion = resolver_revision_stock(
        &pool,
        user,
        &id_local,
        DecisionRevisionStock::NoAplicado,
        "no esta en el TPV: hay que reenviarlo",
    )
    .await
    .unwrap();

    assert!(!resolucion.aplicado);
    assert_eq!(resolucion.estado_cola, ESTADO_PENDIENTE);
    let (estado, reintentos, _) = fila(&pool, user).await;
    assert_eq!(estado, ESTADO_PENDIENTE);
    assert_eq!(reintentos, 0, "un dictamen humano reabre la cuenta");
    let (estado_libro, nota): (String, Option<String>) = sqlx::query_as(
        "SELECT estado, nota FROM bdp_stock_ledger WHERE user_id = $1 AND id_local = $2",
    )
    .bind(user)
    .bind(&id_local)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(estado_libro, "rechazado");
    assert!(nota.unwrap_or_default().contains("no esta en el TPV"));
}
