// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [21AB-2] Prueba que pidió Guillermo el 20/11: que `Units: 0` no llegue nunca
 * a BDP por ninguna de las tres vías que la pueden producir —creación,
 * modificación de una comanda ya creada y ajustes de stock—, y que el
 * simulador no vea un solo payload con 0 unidades.
 *
 * El motivo (doc 20/11): BDP acepta la línea de 0 uds y después
 * `Orders/Payment/Add` con `Invoice: true` falla con «Intento de dividir por
 * cero»: la comanda queda cobrada y sin documento.
 *
 * Puerto propio (18768): 18766 y 18767 ya están ocupados por otros tests.
 */

use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use chrono::Utc;
use glory_backend::models::{ConfiguracionRestaurante, Venta, VentaLinea};
use glory_backend::services::bdp_push::{DOMINIO_STOCK, ESTADO_DESCARTADO};
use glory_backend::services::{
    fijar_permiso, BdpPushFlushService, BdpPushService, BdpSyncService, ConfiguracionService,
};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

const PUERTO: u16 = 18768;
const URL: &str = "http://127.0.0.1:18768";
const CLAVE: &str = "clave-lineas-cero-18768";
const RUTA_ORDEN: &str = "/API/Orders/Create";
const RUTA_REGULARIZAR: &str = "/API/Warehouse/Regularizations";

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

async fn historial() -> Vec<Value> {
    let cuerpo: Value = reqwest::Client::new()
        .get(format!("{URL}/__simulator/history"))
        .header("X-Simulator-Key", CLAVE)
        .send()
        .await
        .expect("admin del simulador")
        .json()
        .await
        .unwrap_or(Value::Null);
    cuerpo["History"].as_array().cloned().unwrap_or_default()
}

/// Rutas de escritura que el simulador registró (no el login ni las lecturas).
async fn escrituras() -> Vec<String> {
    historial()
        .await
        .iter()
        .filter(|e| e["Write"].as_bool().unwrap_or(false))
        .filter_map(|e| e["Path"].as_str().map(str::to_string))
        .collect()
}

/* El criterio literal del punto 2: ningún payload que llegue a BDP puede llevar
 * una línea de 0 unidades. Se mira el cuerpo real, no el código. */
async fn lineas_con_cero_uds() -> Vec<Value> {
    historial()
        .await
        .iter()
        .filter_map(|entrada| {
            let items = entrada["Payload"]["Order"]["Items"].as_array()?;
            let ceros: Vec<Value> = items
                .iter()
                .filter(|item| item["Units"].as_f64() == Some(0.0))
                .cloned()
                .collect();
            (!ceros.is_empty()).then(|| json!({"Path": entrada["Path"], "Lineas": ceros}))
        })
        .collect()
}

/* ── Cuenta y ventas en la base de pruebas ────────────────────────── */

/// Cuenta con BDP apuntando al simulador, en modo automático y con permiso de
/// las categorías que tocan estas pruebas (`comandas` y `stock`).
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
            bdp_default_article_code: Some("1001".to_string()),
            bdp_default_article_name: Some("Coca-Cola".to_string()),
            bdp_tender_map: Some(json!({"efectivo": "1", "tarjeta": "2"})),
            bdp_order_type_map: Some(json!({"comedor": "0", "barra": "0"})),
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
    fijar_permiso(pool, user, "comandas", true).await.unwrap();
    fijar_permiso(pool, user, "stock", true).await.unwrap();
    let config = ConfiguracionService::obtener(pool, user).await.unwrap();
    (user, config)
}

/// Venta local con sus líneas. Se inserta directo para poder colar una línea de
/// 0 uds sin pasar por `VentaService` (que es la frontera que se prueba aparte).
async fn venta_con_lineas(pool: &PgPool, user: Uuid, cantidades: &[&str]) -> Venta {
    let id = Uuid::new_v4();
    sqlx::query(
        r"INSERT INTO ventas (id, user_id, fecha, descripcion, iva_porcentaje, turno, canal,
           metodo_pago, importe_base, importe_iva, bdp_synced)
          VALUES ($1, $2, CURRENT_DATE, 'Test cero uds', 10, 'mediodia', 'comedor',
                  'efectivo', 1.05, 0, false)",
    )
    .bind(id)
    .bind(user)
    .execute(pool)
    .await
    .unwrap();
    for cantidad in cantidades {
        sqlx::query(
            r"INSERT INTO venta_lineas (venta_id, articulo_codigo, descripcion, cantidad,
               precio_unitario, iva_pct, descuento)
              VALUES ($1, '1001', 'COCA-COLA', $2::numeric, 1.05, 10, 0)",
        )
        .bind(id)
        .bind(cantidad)
        .execute(pool)
        .await
        .unwrap();
    }
    fila_venta(pool, id).await
}

async fn fila_venta(pool: &PgPool, venta_id: Uuid) -> Venta {
    sqlx::query_as::<_, Venta>(
        "SELECT id, user_id, fecha, comensales, descripcion, iva_porcentaje, turno, canal,
                metodo_pago, importe_base, importe_iva, reserva_id, cliente_id, created_at,
                updated_at, haddock_synced, haddock_synced_at, haddock_sync_error, bdp_synced,
                bdp_synced_at, bdp_sync_error, bdp_order_id, bdp_order_status, bdp_invoiced,
                anulada, anulada_at, anulacion_motivo, anulacion_usuario, facturada_local,
                factura_numero, factura_fecha, propina, mesa_id, num_mesa, num_salon
           FROM ventas WHERE id = $1",
    )
    .bind(venta_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

fn linea(descripcion: &str, cantidad: &str) -> VentaLinea {
    VentaLinea {
        id: Uuid::new_v4(),
        venta_id: Uuid::nil(),
        articulo_codigo: "1001".into(),
        descripcion: descripcion.into(),
        cantidad: cantidad.parse().unwrap(),
        precio_unitario: Decimal::new(105, 2),
        iva_pct: Decimal::from(10),
        descuento: Decimal::ZERO,
        invitacion: false,
        comentario: String::new(),
        created_at: Utc::now(),
    }
}

async fn login() -> String {
    let resp: Value = reqwest::Client::new()
        .post(format!("{URL}/Auth/Login"))
        .json(&json!({"Login":"local","Password":"secreto","TiempoSession":59,"CodigoIntegrador":"SIM"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    resp["AuthSession"]["Token"].as_str().unwrap().to_string()
}

/// Comanda en el simulador con ese `Total`; devuelve su `OrderId`.
async fn comanda_en_simulador(total: f64) -> i64 {
    let token = login().await;
    let resp: Value = reqwest::Client::new()
        .post(format!("{URL}{RUTA_ORDEN}"))
        .bearer_auth(&token)
        .json(&json!({
            "EmployeeId": 1, "ItemsProfileId": 1,
            "OrderEndType": 0, "OrderOperationType": 0, "Invoice": false,
            "Order": {
                "MarketId": 9900,
                "MarketplaceOrderId": format!("GCERO{}", &Uuid::new_v4().simple().to_string()[..10]),
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

/* ===== 1. Creación ===== */

#[sqlx::test(migrations = "./migrations")]
async fn la_creacion_con_cero_uds_no_llega_a_bdp(pool: PgPool) {
    simulador_en_serie!();
    let (user, config) = cuenta(&pool).await;

    /* Se cuela la línea de 0 uds directamente en la BD: la validación de la UI
     * y la del alta no son la frontera que aquí se prueba. */
    let venta = venta_con_lineas(&pool, user, &["1", "0"]).await;
    BdpSyncService::sync_venta(&pool, &venta, &config, false, None).await;

    assert!(
        !escrituras().await.contains(&RUTA_ORDEN.to_string()),
        "una comanda con 0 uds no puede salir a BDP: {:?}",
        escrituras().await
    );
    let error: Option<String> =
        sqlx::query_scalar("SELECT bdp_sync_error FROM ventas WHERE id = $1")
            .bind(venta.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        error
            .as_ref()
            .is_some_and(|e| e.contains("cantidad debe ser mayor que cero")),
        "el rechazo tiene que decir qué línea falla: {error:?}"
    );
    assert!(lineas_con_cero_uds().await.is_empty());
}

/// El control del mismo camino: la misma venta con 1 ud sí sale y el payload
/// llega con su línea. Sin este positivo, el «no llegó» de arriba no prueba nada.
#[sqlx::test(migrations = "./migrations")]
async fn el_control_de_una_ud_si_crea_la_comanda(pool: PgPool) {
    simulador_en_serie!();
    let (user, config) = cuenta(&pool).await;
    let venta = venta_con_lineas(&pool, user, &["1"]).await;

    BdpSyncService::sync_venta(&pool, &venta, &config, false, None).await;

    let escrituras = escrituras().await;
    assert!(
        escrituras.contains(&RUTA_ORDEN.to_string()),
        "la comanda válida tiene que salir: {escrituras:?}"
    );
    assert!(lineas_con_cero_uds().await.is_empty());
    let order_id: Option<i64> = sqlx::query_scalar("SELECT bdp_order_id FROM ventas WHERE id = $1")
        .bind(venta.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(order_id.is_some_and(|id| id > 0), "debe quedar su OrderId");
}

/* ===== 2. Modificación de una comanda ya creada ===== */

#[sqlx::test(migrations = "./migrations")]
async fn la_modificacion_no_envia_cero_uds(pool: PgPool) {
    simulador_en_serie!();
    let (user, config) = cuenta(&pool).await;

    /* La comanda existe en BDP por 2.10 (2 × 1.05): es el total que la web
     * conoce de `antes`, y `modificar_comanda` lo relee antes de escribir. */
    let order_id = comanda_en_simulador(2.10).await;
    let venta_local = venta_con_lineas(&pool, user, &["2"]).await;
    sqlx::query("UPDATE ventas SET bdp_synced = true, bdp_order_id = $2 WHERE id = $1")
        .bind(venta_local.id)
        .bind(order_id)
        .execute(&pool)
        .await
        .unwrap();
    let venta = fila_venta(&pool, venta_local.id).await;

    let antes = vec![linea("COCA-COLA", "2")];
    /* Se añade una línea nueva de 0 uds: no puede convertirse en un ajuste. */
    let despues = vec![linea("COCA-COLA", "3"), linea("SCHWEPPES LIMÓN", "0")];

    BdpSyncService::modificar_comanda(&pool, &venta, &antes, &despues, &config)
        .await
        .expect("la modificación válida debe aceptarse");

    assert!(
        lineas_con_cero_uds().await.is_empty(),
        "ningún payload de modificación puede llevar 0 uds: {:?}",
        lineas_con_cero_uds().await
    );
    /* El ajuste sí salió: solo la línea que cambia, con su +1. */
    let ajustes: Vec<Value> = historial()
        .await
        .iter()
        .filter(|e| e["Path"] == RUTA_ORDEN && e["Payload"]["Order"]["OrderId"].is_number())
        .flat_map(|e| {
            e["Payload"]["Order"]["Items"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .collect();
    assert_eq!(ajustes.len(), 1, "solo la línea que cambia: {ajustes:?}");
    assert_eq!(ajustes[0]["Units"].as_f64(), Some(1.0));
}

/* ===== 3. Ajustes de stock con neto cero ===== */

#[sqlx::test(migrations = "./migrations")]
async fn el_ajuste_de_stock_neto_cero_no_se_envia(pool: PgPool) {
    simulador_en_serie!();
    let (user, _config) = cuenta(&pool).await;

    let ajuste = |unidades: &str| {
        json!({
            "Article": 1001, "sD1": "", "sD2": "", "sD3": "",
            "Units": unidades, "CodReg": 1, "Store": 1, "DateReg": "2026-11-20"
        })
    };
    BdpPushService::acumular_regularizacion(&pool, user, "1001:1", Decimal::from(2), &ajuste("2"))
        .await
        .unwrap();
    BdpPushService::acumular_regularizacion(
        &pool,
        user,
        "1001:1",
        Decimal::from(-2),
        &ajuste("-2"),
    )
    .await
    .unwrap();

    /* El neto es 0: la fila no se envía, queda descartada. */
    let estado: String = sqlx::query_scalar(
        "SELECT estado FROM bdp_push_pendientes \
         WHERE user_id = $1 AND dominio = $2 AND entidad_id = '1001:1'",
    )
    .bind(user)
    .bind(DOMINIO_STOCK)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(estado, ESTADO_DESCARTADO);

    let _ = BdpPushFlushService::flush(&pool, user, true).await;
    assert!(
        !escrituras().await.contains(&RUTA_REGULARIZAR.to_string()),
        "un neto de 0 uds no se envía a BDP: {:?}",
        escrituras().await
    );
}
