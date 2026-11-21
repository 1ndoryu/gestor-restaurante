// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [21AB-1] Pruebas de pagos que pidió Guillermo el 20/11/2026.
 *
 * Tres escenarios, con el simulador como testigo (cuenta cada petición y dice
 * cuáles escriben):
 *  1. Dos solicitudes simultáneas del mismo pago → una sola `AddOrderPayment`.
 *  2. BDP cobra y la web pierde la respuesta → al reintentar se detecta por
 *     `GetOrder` y no se reenvía.
 *  3. Tope de tres pagos por comanda → el cuarto no sale.
 *
 * Puerto propio (18767): `bdp_service_integration` y
 * `bdp_seguridad_escritura_simulador` ya comparten el 18766.
 * Los tres tests comparten simulador, así que van en serie con `SERIE`.
 */

use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use chrono::Utc;
use glory_backend::models::{ConfiguracionRestaurante, Venta};
use glory_backend::services::{fijar_permiso, BdpSyncService, ConfiguracionService};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

const PUERTO: u16 = 18767;
const URL: &str = "http://127.0.0.1:18767";
const CLAVE: &str = "clave-pagos-idempotencia-18767";
const RUTA_PAGO: &str = "/API/Orders/Payment/Add";

static SERIE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
static SIMULADOR: Mutex<Option<Child>> = Mutex::new(None);
static LISTO: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

async fn asegurar_simulador() -> bool {
    if let Some(&listo) = LISTO.get() {
        return listo;
    }
    let cliente = reqwest::Client::new();
    let vivo = cliente
        .post(format!("{URL}/Service/Health"))
        .json(&json!({}))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .is_ok_and(|r| r.status().is_success());
    if vivo {
        let _ = LISTO.set(true);
        return true;
    }

    let servidor = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tools/bdp-weblink-simulator/server.py");
    if !servidor.exists() {
        let _ = LISTO.set(false);
        return false;
    }
    let Ok(hijo) = Command::new("python")
        .arg(&servidor)
        .arg("--port")
        .arg(PUERTO.to_string())
        .arg("--admin-key")
        .arg(CLAVE)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        let _ = LISTO.set(false);
        return false;
    };
    *SIMULADOR.lock().unwrap() = Some(hijo);

    for _ in 0..20 {
        tokio::time::sleep(Duration::from_millis(150)).await;
        if cliente
            .post(format!("{URL}/Service/Health"))
            .json(&json!({}))
            .timeout(Duration::from_secs(1))
            .send()
            .await
            .is_ok_and(|r| r.status().is_success())
        {
            let _ = LISTO.set(true);
            return true;
        }
    }
    let _ = LISTO.set(false);
    false
}

/// Simulador listo, estado limpio y turno en exclusiva mientras dure el test.
macro_rules! simulador_en_serie {
    () => {
        if !asegurar_simulador().await {
            eprintln!("SKIP: Python o simulador no disponible");
            return;
        }
        let _serie = SERIE.lock().await;
        admin_post("/__simulator/reset", json!({})).await;
    };
}

async fn admin_post(ruta: &str, cuerpo: Value) -> Value {
    reqwest::Client::new()
        .post(format!("{URL}{ruta}"))
        .header("X-Simulator-Key", CLAVE)
        .json(&cuerpo)
        .send()
        .await
        .expect("admin del simulador")
        .json()
        .await
        .unwrap_or(Value::Null)
}

async fn admin_estado(ruta: &str) -> Value {
    reqwest::Client::new()
        .get(format!("{URL}{ruta}"))
        .header("X-Simulator-Key", CLAVE)
        .send()
        .await
        .expect("admin del simulador")
        .json()
        .await
        .unwrap_or(Value::Null)
}

/// Rutas de escritura que el simulador registró (no el login ni las lecturas).
async fn escrituras() -> Vec<String> {
    let cuerpo = admin_estado("/__simulator/history").await;
    cuerpo["History"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter(|e| e["Write"].as_bool().unwrap_or(false))
        .filter_map(|e| e["Path"].as_str().map(str::to_string))
        .collect()
}

async fn escrituras_de_pago() -> usize {
    escrituras()
        .await
        .iter()
        .filter(|p| *p == RUTA_PAGO)
        .count()
}

/// Pagos que la comanda tiene apuntados dentro del simulador.
async fn pagos_en_comanda(order_id: i64) -> Vec<Value> {
    let estado = admin_estado("/__simulator/state").await;
    estado["Orders"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .find(|o| o["OrderId"].as_i64() == Some(order_id))
        .and_then(|o| o["Payments"].as_array().cloned())
        .unwrap_or_default()
}

/* ── Cuenta y venta en la base de pruebas ─────────────────────────── */

/// Cuenta con BDP apuntando al simulador, en modo automático y con permiso de
/// categoría `pagos`: el pago no necesita armado manual, pero sí su permiso.
async fn cuenta(pool: &PgPool) -> (Uuid, ConfiguracionRestaurante) {
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
        &glory_backend::models::ActualizarConfiguracionRequest {
            modo_operacion: Some("bdp".to_string()),
            bdp_base_url: Some(URL.to_string()),
            bdp_sync_enabled: Some(true),
            bdp_auto_backup_before_write: Some(true),
            bdp_login: Some("local".to_string()),
            bdp_password: Some("secreto".to_string()),
            bdp_integrator_code: Some("SIM".to_string()),
            bdp_pos_id: Some(1),
            bdp_employee_id: Some(1),
            bdp_items_profile_id: Some(1),
            ff_bdp_partial_payments: Some(true),
            ..Default::default()
        },
    )
    .await
    .expect("config de la cuenta");
    sqlx::query(
        "UPDATE configuracion_restaurante SET bdp_sync_mode = 'automatic' WHERE user_id = $1",
    )
    .bind(user)
    .execute(pool)
    .await
    .unwrap();
    fijar_permiso(pool, user, "pagos", true).await.unwrap();
    let config = ConfiguracionService::obtener(pool, user).await.unwrap();
    (user, config)
}

/// Crea la comanda en el simulador y la venta local con ese total.
async fn comanda_y_venta(pool: &PgPool, user: Uuid, total: Decimal) -> Venta {
    let order_id = comanda_en_simulador(total).await;
    let id = Uuid::new_v4();
    sqlx::query(
        r"INSERT INTO ventas (id, user_id, fecha, descripcion, iva_porcentaje, turno, canal,
           metodo_pago, importe_base, importe_iva, bdp_synced, bdp_order_id)
          VALUES ($1, $2, CURRENT_DATE, 'Test pagos', 10, 'mediodia', 'comedor',
                  'efectivo', $3, 0, true, $4)",
    )
    .bind(id)
    .bind(user)
    .bind(total)
    .bind(order_id)
    .execute(pool)
    .await
    .unwrap();
    Venta {
        id,
        user_id: user,
        fecha: Utc::now().date_naive(),
        comensales: Some(2),
        descripcion: "Test pagos".to_string(),
        iva_porcentaje: Decimal::new(10, 0),
        turno: "mediodia".into(),
        canal: "comedor".into(),
        metodo_pago: "efectivo".into(),
        importe_base: total,
        importe_iva: Decimal::ZERO,
        reserva_id: None,
        cliente_id: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        haddock_synced: false,
        haddock_synced_at: None,
        haddock_sync_error: None,
        bdp_synced: true,
        bdp_synced_at: Some(Utc::now()),
        bdp_sync_error: None,
        bdp_order_id: Some(order_id),
        bdp_order_status: None,
        bdp_invoiced: false,
        anulada: false,
        anulada_at: None,
        anulacion_motivo: None,
        anulacion_usuario: None,
        facturada_local: false,
        factura_numero: None,
        factura_fecha: None,
        propina: Decimal::ZERO,
        mesa_id: None,
        num_mesa: None,
        num_salon: None,
    }
}

/// Comanda en el simulador con ese `Total`; devuelve su `OrderId`.
async fn comanda_en_simulador(total: Decimal) -> i64 {
    let login: Value = reqwest::Client::new()
        .post(format!("{URL}/Auth/Login"))
        .json(&json!({"Login":"local","Password":"secreto","TiempoSession":59,"CodigoIntegrador":"SIM"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let token = login["AuthSession"]["Token"].as_str().unwrap().to_string();
    let total: f64 = total.to_string().parse().unwrap();
    let resp: Value = reqwest::Client::new()
        .post(format!("{URL}/API/Orders/Create"))
        .bearer_auth(&token)
        .json(&json!({
            "EmployeeId": 1, "ItemsProfileId": 1,
            "OrderEndType": 0, "OrderOperationType": 0, "Invoice": false,
            "Order": {
                "MarketId": 9900,
                "MarketplaceOrderId": format!("GPAG{}", &Uuid::new_v4().simple().to_string()[..10]),
                "PosId": 1, "Total": total,
                "Items": [{"Id": 1001, "Name": "Coca-Cola", "Units": 1, "Price": total, "Total": total}]
            }
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    resp["OrderId"].as_i64().expect("OrderId del simulador")
}

/// Cobra directamente en el simulador, sin pasar por los servicios.
async fn cobrar_en_simulador(order_id: i64, payment_id: &str, importe: f64) {
    let login: Value = reqwest::Client::new()
        .post(format!("{URL}/Auth/Login"))
        .json(&json!({"Login":"local","Password":"secreto","TiempoSession":59,"CodigoIntegrador":"SIM"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let token = login["AuthSession"]["Token"].as_str().unwrap().to_string();
    let resp: Value = reqwest::Client::new()
        .post(format!("{URL}{RUTA_PAGO}"))
        .bearer_auth(&token)
        .json(&json!({
            "OrderIdentifier": {"OrderId": order_id},
            "Payment": {"TenderId": 1, "Amount": importe, "PaymentId": payment_id}
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        resp["ErrorMessage"].as_str().unwrap_or(""),
        "",
        "la comanda de prueba debe admitir el cobro previo"
    );
}

async fn pagos_exitosos(pool: &PgPool, venta_id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM bdp_pagos WHERE venta_id = $1 AND resultado = 'exito'")
        .bind(venta_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn resultado_auditoria(pool: &PgPool, venta_id: Uuid) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT resultado FROM bdp_audit_log \
         WHERE operacion = 'add_payment' AND datos_enviados ->> 'venta_id' = $1 \
         ORDER BY created_at",
    )
    .bind(venta_id.to_string())
    .fetch_all(pool)
    .await
    .unwrap()
}

/* ===== 1. Dos solicitudes simultáneas del mismo pago ===== */

#[sqlx::test(migrations = "./migrations")]
async fn dos_pagos_simultaneos_solo_escriben_una_vez(pool: PgPool) {
    simulador_en_serie!();
    let (user, config) = cuenta(&pool).await;
    let venta = comanda_y_venta(&pool, user, Decimal::new(2750, 2)).await;
    let clave = "21ab1-simultaneo";

    let (uno, dos) = tokio::join!(
        BdpSyncService::add_order_payment(
            &pool,
            &venta,
            &config,
            Decimal::new(2750, 2),
            1,
            Some(clave)
        ),
        BdpSyncService::add_order_payment(
            &pool,
            &venta,
            &config,
            Decimal::new(2750, 2),
            1,
            Some(clave)
        ),
    );

    assert_eq!(
        escrituras_de_pago().await,
        1,
        "BDP solo debe recibir un AddOrderPayment (uno={uno:?}, dos={dos:?})"
    );
    assert_eq!(pagos_en_comanda(venta.bdp_order_id.unwrap()).await.len(), 1);
    assert_eq!(pagos_exitosos(&pool, venta.id).await, 1);
    /* La segunda puede fallar con «ya hay un pago en proceso» o encontrar el
     * pago ya exitoso: lo que no puede es escribir dos veces. */
    assert!(
        uno.is_ok() || dos.is_ok(),
        "una de las dos solicitudes debe completarse: {uno:?} {dos:?}"
    );
}

/* ===== 2. BDP cobra y la web pierde la respuesta ===== */

#[sqlx::test(migrations = "./migrations")]
async fn respuesta_perdida_se_recupera_sin_reenviar(pool: PgPool) {
    simulador_en_serie!();
    let (user, config) = cuenta(&pool).await;
    let venta = comanda_y_venta(&pool, user, Decimal::new(2750, 2)).await;
    let order_id = venta.bdp_order_id.unwrap();
    let clave = "21ab1-perdida";

    /* El simulador aplica el cobro y corta la respuesta: BDP cobró, la web no
     * lo sabe. Es exactamente el caso que Guillermo pide cubrir. */
    admin_post(
        "/__simulator/fault",
        json!({"Path": RUTA_PAGO, "apply_then_disconnect": true}),
    )
    .await;

    let primero = BdpSyncService::add_order_payment(
        &pool,
        &venta,
        &config,
        Decimal::new(2750, 2),
        1,
        Some(clave),
    )
    .await;
    assert!(
        primero.is_err(),
        "el corte debe hacer fallar el cobro: {primero:?}"
    );
    assert_eq!(
        pagos_en_comanda(order_id).await.len(),
        1,
        "BDP sí cobró: el pago tiene que estar en la comanda"
    );
    assert_eq!(
        resultado_auditoria(&pool, venta.id).await,
        vec!["ambiguo".to_string()],
        "sin `ambiguo` el reintento no reconcilia y reenviaría el cobro"
    );

    /* Reintento con la misma clave: debe detectarlo por GetOrder y no reenviar. */
    let segundo = BdpSyncService::add_order_payment(
        &pool,
        &venta,
        &config,
        Decimal::new(2750, 2),
        1,
        Some(clave),
    )
    .await;
    assert!(segundo.is_ok(), "el reintento debe resolverse: {segundo:?}");
    assert_eq!(
        segundo.unwrap(),
        None,
        "ya estaba pagado: nada que facturar"
    );
    assert_eq!(
        escrituras_de_pago().await,
        1,
        "no se reenvía el pago ya aplicado"
    );
    assert_eq!(pagos_en_comanda(order_id).await.len(), 1, "un solo cobro");
    assert_eq!(pagos_exitosos(&pool, venta.id).await, 1);
}

/* ===== 3. Tope de tres pagos por comanda ===== */

#[sqlx::test(migrations = "./migrations")]
async fn el_cuarto_pago_se_bloquea(pool: PgPool) {
    simulador_en_serie!();
    let (user, config) = cuenta(&pool).await;
    let venta = comanda_y_venta(&pool, user, Decimal::new(1000, 2)).await;
    let order_id = venta.bdp_order_id.unwrap();

    /* Tres pagos ya registrados en BDP, cada uno de 1 €. */
    for n in 1..=3 {
        cobrar_en_simulador(order_id, &format!("21ab1-previo-{n}"), 1.0).await;
    }
    let antes = escrituras_de_pago().await;

    let cuarto = BdpSyncService::add_order_payment(
        &pool,
        &venta,
        &config,
        Decimal::new(100, 2),
        1,
        Some("21ab1-cuarto"),
    )
    .await;

    let mensaje = cuarto.expect_err("el cuarto pago debe bloquearse");
    assert!(
        mensaje.contains("3 pagos registrados en BDP"),
        "mensaje esperado del tope: {mensaje}"
    );
    assert_eq!(
        escrituras_de_pago().await,
        antes,
        "el cuarto pago no puede llegar a BDP"
    );
    assert_eq!(pagos_en_comanda(order_id).await.len(), 3);
}
