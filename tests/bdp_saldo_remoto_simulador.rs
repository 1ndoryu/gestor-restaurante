// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [21AB-5a] Saldo de la comanda desde `GetOrder`, con el simulador como TPV.
 *
 * La regresión del 21/11: la web calculaba `importe_base + importe_iva` y el
 * TPV cobraba otro total (descuento), así que tras cobrarlo todo la pantalla
 * seguía mostrando 0,45 pendientes. Guillermo pide `Order.Total` menos los
 * `Payments` de BDP.
 *
 * Cuatro escenarios:
 *  1. El saldo sale del TPV aunque la web calcule otro total.
 *  2. Tres pagos dejan pendiente cero; el cuarto no se escribe.
 *  3. Sin BDP el saldo se declara local (no se disfraza de saldo del TPV).
 *  4. El pago local no admite más que el pendiente del TPV.
 *
 * Puerto propio (18770): el 18768 lo usa `bdp_lineas_cero_simulador` con otra
 * clave de admin, y compartir puerto hacía que el reset de este archivo diera
 * 403 en silencio y los tests corrieran sobre el estado del otro.
 * Los tests comparten simulador, así que van en serie con `SERIE`.
 */

use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use chrono::Utc;
use glory_backend::models::{ConfiguracionRestaurante, Venta};
use glory_backend::repositories::{BdpPagoRepository, VentaRepository};
use glory_backend::services::{
    fijar_permiso, BdpSyncService, ConfiguracionService, OrigenSaldo, VentaService,
};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

const PUERTO: u16 = 18770;
const URL: &str = "http://127.0.0.1:18770";
const CLAVE: &str = "clave-saldo-remoto-18770";
const RUTA_PAGO: &str = "/API/Orders/Payment/Add";

static SERIE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
static SIMULADOR: Mutex<Option<Child>> = Mutex::new(None);
static LISTO: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/* En la VPS 89 solo existe `python3`: buscar `python` a secas hacía que el
 * simulador no arrancara y todos estos tests se saltaran en verde. */
fn interprete_python() -> &'static str {
    ["python3", "python"]
        .into_iter()
        .find(|bin| {
            Command::new(bin)
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|s| s.success())
        })
        .unwrap_or_else(|| {
            panic!("sin Python (ni python3 ni python) no se puede probar el simulador BDP")
        })
}

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
    let Ok(hijo) = Command::new(interprete_python())
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
        assert!(
            asegurar_simulador().await,
            "el simulador BDP no arrancó: el test no puede saltarse en verde"
        );
        let _serie = SERIE.lock().await;
        admin_reset().await;
    };
}

/// Reset del simulador con la clave de admin de este archivo.
///
/// Falla si el admin lo rechaza: un 403 silencioso dejaba los tests corriendo
/// sobre el estado de otro proceso (puerto compartido, 24/11/2026).
async fn admin_reset() {
    let resp = reqwest::Client::new()
        .post(format!("{URL}/__simulator/reset"))
        .header("X-Simulator-Key", CLAVE)
        .json(&json!({}))
        .send()
        .await
        .expect("admin del simulador");
    assert!(
        resp.status().is_success(),
        "el simulador rechazó el reset ({}): ¿otro proceso con otra clave ocupa el puerto {}?",
        resp.status(),
        PUERTO
    );
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

async fn escrituras_de_pago() -> usize {
    admin_estado("/__simulator/history")
        .await
        .get("History")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter(|e| e["Write"].as_bool().unwrap_or(false))
        .filter(|e| e["Path"].as_str() == Some(RUTA_PAGO))
        .count()
}

async fn pagos_en_comanda(order_id: i64) -> Vec<Value> {
    admin_estado("/__simulator/state")
        .await
        .get("Orders")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .find(|o| o["OrderId"].as_i64() == Some(order_id))
        .and_then(|o| o["Payments"].as_array().cloned())
        .unwrap_or_default()
}

/* ── Cuenta y venta en la base de pruebas ─────────────────────────── */

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
    /* `add_payment` se enciende con su propia categoría: sin ella el guard
     * bloquea el cobro antes de llegar al simulador. */
    fijar_permiso(pool, user, "pagos", true).await.unwrap();
    let config = ConfiguracionService::obtener(pool, user).await.unwrap();
    (user, config)
}

/// Venta con ese total local y esa comanda en el simulador.
async fn venta_con_comanda(
    pool: &PgPool,
    user: Uuid,
    total_local: Decimal,
    total_bdp: Decimal,
) -> Venta {
    let order_id = comanda_en_simulador(total_bdp).await;
    let id = Uuid::new_v4();
    sqlx::query(
        r"INSERT INTO ventas (id, user_id, fecha, descripcion, iva_porcentaje, turno, canal,
           metodo_pago, importe_base, importe_iva, bdp_synced, bdp_order_id)
          VALUES ($1, $2, CURRENT_DATE, 'Test saldo', 10, 'mediodia', 'comedor',
                  'efectivo', $3, 0, true, $4)",
    )
    .bind(id)
    .bind(user)
    .bind(total_local)
    .bind(order_id)
    .execute(pool)
    .await
    .unwrap();
    Venta {
        id,
        user_id: user,
        fecha: Utc::now().date_naive(),
        comensales: Some(2),
        descripcion: "Test saldo".to_string(),
        iva_porcentaje: Decimal::new(10, 0),
        turno: "mediodia".into(),
        canal: "comedor".into(),
        metodo_pago: "efectivo".into(),
        importe_base: total_local,
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

async fn token() -> String {
    let login: Value = reqwest::Client::new()
        .post(format!("{URL}/Auth/Login"))
        .json(&json!({"Login":"local","Password":"secreto","TiempoSession":59,"CodigoIntegrador":"SIM"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    login["AuthSession"]["Token"].as_str().unwrap().to_string()
}

async fn comanda_en_simulador(total: Decimal) -> i64 {
    let token = token().await;
    let total: f64 = total.to_string().parse().unwrap();
    let resp: Value = reqwest::Client::new()
        .post(format!("{URL}/API/Orders/Create"))
        .bearer_auth(&token)
        .json(&json!({
            "EmployeeId": 1, "ItemsProfileId": 1,
            "OrderEndType": 0, "OrderOperationType": 0, "Invoice": false,
            "Order": {
                "MarketId": 9900,
                "MarketplaceOrderId": format!("GSAL{}", &Uuid::new_v4().simple().to_string()[..10]),
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
    let token = token().await;
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

/* ===== 1. El saldo sale del TPV aunque la web calcule otro total ===== */

#[sqlx::test(migrations = "./migrations")]
async fn saldo_sale_del_tpv_aunque_la_web_calcule_otro_total(pool: PgPool) {
    simulador_en_serie!();
    let (user, config) = cuenta(&pool).await;
    /* La web cree 4,95 (base + IVA); el TPV tiene 4,50 por el descuento. */
    let venta = venta_con_comanda(&pool, user, Decimal::new(495, 2), Decimal::new(450, 2)).await;
    cobrar_en_simulador(venta.bdp_order_id.unwrap(), "21ab5a-descuento", 4.50).await;

    let saldo = BdpSyncService::saldo_de_venta(&pool, &venta, &config)
        .await
        .expect("el saldo debe salir de BDP");

    assert_eq!(saldo.origen, OrigenSaldo::Bdp);
    assert_eq!(
        saldo.total,
        Decimal::new(450, 2),
        "total del TPV, no base + IVA"
    );
    assert_eq!(saldo.pagado, Decimal::new(450, 2));
    assert_eq!(
        saldo.pendiente,
        Decimal::ZERO,
        "cobrado todo: no puede quedar saldo pendiente (regresión del 21/11)"
    );
}

/* ===== 2. Tres pagos dejan pendiente cero; el cuarto se corta por saldo ===== */

#[sqlx::test(migrations = "./migrations")]
async fn tres_pagos_dejan_pendiente_cero_y_el_cuarto_no_excede_el_saldo(pool: PgPool) {
    simulador_en_serie!();
    let (user, config) = cuenta(&pool).await;
    let venta = venta_con_comanda(&pool, user, Decimal::new(450, 2), Decimal::new(450, 2)).await;

    for n in 1..=3 {
        BdpSyncService::add_order_payment(
            &pool,
            &venta,
            &config,
            Decimal::new(150, 2),
            1,
            Some(&format!("21ab5a-pago-{n}")),
        )
        .await
        .unwrap_or_else(|e| panic!("el pago {n} debe registrarse: {e}"));
    }

    let saldo = BdpSyncService::saldo_de_venta(&pool, &venta, &config)
        .await
        .expect("saldo tras tres pagos");
    assert_eq!(saldo.origen, OrigenSaldo::Bdp);
    assert_eq!(saldo.pagado, Decimal::new(450, 2));
    assert_eq!(
        saldo.pendiente,
        Decimal::ZERO,
        "todo cobrado: pendiente cero"
    );

    /* El cuarto pago ya no se corta por número (BDP 27/11: el tope de tres del
     * manual está desactualizado), pero con el saldo a cero no hay nada que
     * cobrar y no llega a BDP. */
    let antes = escrituras_de_pago().await;
    let cuarto = BdpSyncService::add_order_payment(
        &pool,
        &venta,
        &config,
        Decimal::new(10, 2),
        1,
        Some("21ab5a-cuarto"),
    )
    .await;
    let mensaje = cuarto.expect_err("sin saldo, el cuarto pago debe bloquearse");
    assert!(
        mensaje.contains("excede el saldo pendiente"),
        "mensaje esperado del saldo: {mensaje}"
    );
    assert_eq!(
        escrituras_de_pago().await,
        antes,
        "el cuarto pago no puede llegar a BDP"
    );
    assert_eq!(pagos_en_comanda(venta.bdp_order_id.unwrap()).await.len(), 3);
}

/* ===== 3. Sin BDP el saldo se declara local ===== */

#[sqlx::test(migrations = "./migrations")]
async fn sin_bdp_el_saldo_se_declara_local(pool: PgPool) {
    simulador_en_serie!();
    let (user, _config) = cuenta(&pool).await;
    let venta = venta_con_comanda(&pool, user, Decimal::new(495, 2), Decimal::new(450, 2)).await;

    /* BDP inalcanzable: el login falla y el saldo no puede ser el del TPV. */
    let mut config_caida = ConfiguracionService::obtener(&pool, user).await.unwrap();
    config_caida.bdp_base_url = "http://127.0.0.1:1".to_string();

    let saldo = BdpSyncService::saldo_de_venta(&pool, &venta, &config_caida)
        .await
        .expect("el respaldo local no puede fallar");

    assert_eq!(
        saldo.origen,
        OrigenSaldo::Local,
        "sin TPV el saldo es local"
    );
    assert_eq!(saldo.total, Decimal::new(495, 2), "copia de la web");
    assert_eq!(saldo.pendiente, Decimal::new(495, 2));
}

/* ===== 4. El pago local no admite más que el pendiente del TPV ===== */

#[sqlx::test(migrations = "./migrations")]
async fn el_pago_local_no_admite_mas_que_el_pendiente_del_tpv(pool: PgPool) {
    simulador_en_serie!();
    let (user, _config) = cuenta(&pool).await;
    /* Local 4,95; TPV 4,50 con 2,00 ya cobrados: el tope real es 2,50. */
    let venta = venta_con_comanda(&pool, user, Decimal::new(495, 2), Decimal::new(450, 2)).await;
    cobrar_en_simulador(venta.bdp_order_id.unwrap(), "21ab5a-previo", 2.00).await;

    /* 2,60 excede el pendiente del TPV (2,50) aunque quepa en el local (4,95). */
    let intento = VentaService::pago_parcial_local(
        &pool,
        user,
        venta.id,
        Decimal::new(260, 2),
        1,
        Some("21ab5a-local-excede"),
    )
    .await;
    let error = intento.expect_err("no puede cobrarse más que el pendiente del TPV");
    assert!(
        error.to_string().contains("excede el saldo pendiente"),
        "mensaje esperado de saldo: {error}"
    );

    /* 2,50 sí cabe: se registra y el saldo del TPV queda en cero. */
    VentaService::pago_parcial_local(
        &pool,
        user,
        venta.id,
        Decimal::new(250, 2),
        1,
        Some("21ab5a-local-cabe"),
    )
    .await
    .expect("el pendiente del TPV sí se puede cobrar");

    let config = ConfiguracionService::obtener(&pool, user).await.unwrap();
    let saldo = BdpSyncService::saldo_de_venta(&pool, &venta, &config)
        .await
        .unwrap();
    assert_eq!(
        saldo.pendiente,
        Decimal::ZERO,
        "cobrado el resto: pendiente cero"
    );
}

/* ===== 5. Facturada: BDP borra la comanda y el saldo no puede resucitar ===== */

/* [21AB-5d] Lo observado en el TPV demo: tras `InvoiceOrder`, `GetOrder`
 * devuelve `Order: null` (`Status: 3`). El saldo caía entonces al cálculo
 * local y una venta cobrada y facturada mostraba 7,80 pendientes.
 *
 * Sin simulador a propósito: la comanda ya no existe para BDP, así que aquí
 * la prueba es que ni se pregunta. El `bdp_order_id` apunta a una comanda
 * inexistente y aun así el saldo es cero. */
#[sqlx::test(migrations = "./migrations")]
async fn facturada_no_deja_pendiente_aunque_bdp_borre_la_comanda(pool: PgPool) {
    let (user, config) = cuenta(&pool).await;
    /* Venta con comanda remota y ya facturada, sin simulador: si el saldo
     * preguntara a BDP, el login fallaría y el test lo delataría. */
    let id = Uuid::new_v4();
    sqlx::query(
        r"INSERT INTO ventas (id, user_id, fecha, descripcion, iva_porcentaje, turno, canal,
           metodo_pago, importe_base, importe_iva, bdp_synced, bdp_order_id, bdp_invoiced)
          VALUES ($1, $2, CURRENT_DATE, 'Test facturada', 10, 'mediodia', 'comedor',
                  'efectivo', 9.90, 0, true, 999999, true)",
    )
    .bind(id)
    .bind(user)
    .execute(&pool)
    .await
    .unwrap();
    for n in 1..=3 {
        let clave = format!("21ab5d-cobro-{n}");
        BdpPagoRepository::insertar(
            &pool,
            id,
            Decimal::new(70, 2),
            1,
            &clave,
            Some(999_999),
            Some(&clave),
        )
        .await
        .expect("el ledger local registra el cobro");
    }
    let venta = VentaRepository::find_by_id(&pool, id, user)
        .await
        .unwrap()
        .expect("la venta sigue existiendo");

    let saldo = BdpSyncService::saldo_de_venta(&pool, &venta, &config)
        .await
        .expect("saldo de una venta facturada");
    assert_eq!(saldo.pendiente, Decimal::ZERO, "facturada: nada pendiente");
    assert_eq!(saldo.total, Decimal::new(210, 2), "el total es lo cobrado");
}
