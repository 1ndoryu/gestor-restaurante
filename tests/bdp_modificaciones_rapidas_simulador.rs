// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [21AB-5c] Modificaciones rápidas de la misma comanda, con el simulador como TPV.
 *
 * La regresión del 21/11: dos ediciones seguidas leían las mismas líneas de
 * partida y mandaban el mismo delta dos veces (el producto se duplicaba en el
 * TPV), y el throttle rechazaba al instante, así que el `GetOrder` de la
 * segunda caía por sus peticiones hermanas. Guillermo pide serializar, esperar
 * de forma controlada y conciliar con `GetOrder`.
 *
 * Tres escenarios:
 *  1. Tres modificaciones seguidas dejan el estado final correcto en web y TPV.
 *  2. Ninguna se pierde: las tres escriben y el total del TPV es el último.
 *  3. La reconciliación tras un corte no repite un delta ya aplicado.
 *
 * Puerto propio (18769): los demás simuladores usan 18767 y 18768.
 */

use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use glory_backend::models::{
    ActualizarVentaRequest, CanalVenta, ConfiguracionRestaurante, CrearVentaLineaRequest,
    MetodoPago, Turno,
};
use glory_backend::services::{ConfiguracionService, VentaService};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

const PUERTO: u16 = 18769;
const URL: &str = "http://127.0.0.1:18769";
const CLAVE: &str = "clave-modificaciones-18769";

static SERIE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
static SIMULADOR: Mutex<Option<Child>> = Mutex::new(None);
static LISTO: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

async fn asegurar_simulador() -> bool {
    if let Some(&listo) = LISTO.get() {
        return listo;
    }
    let cliente = reqwest::Client::new();
    if cliente
        .post(format!("{URL}/Service/Health"))
        .json(&json!({}))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .is_ok_and(|r| r.status().is_success())
    {
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

async fn comanda_en_simulador() -> i64 {
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
        .post(format!("{URL}/API/Orders/Create"))
        .bearer_auth(&token)
        .json(&json!({
            "EmployeeId": 1, "ItemsProfileId": 1,
            "OrderEndType": 0, "OrderOperationType": 0, "Invoice": false,
            "Order": {
                "MarketId": 9900,
                "MarketplaceOrderId": format!("GMOD{}", &Uuid::new_v4().simple().to_string()[..10]),
                "PosId": 1, "Total": 1.00,
                "Items": [{"Id": 1001, "Name": "Coca-Cola", "Units": 1, "Price": 1.00, "Total": 1.00}]
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

async fn orden_en_simulador(order_id: i64) -> Value {
    admin_estado("/__simulator/state")
        .await
        .get("Orders")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .find(|o| o["OrderId"].as_i64() == Some(order_id))
        .unwrap_or(Value::Null)
}

/* ── Cuenta y venta ─────────────────────────────────────────────── */

async fn cuenta(pool: &PgPool) -> (Uuid, ConfiguracionRestaurante) {
    let user = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(user)
        .bind(format!("test-mod-{user}@example.com"))
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
    let config = ConfiguracionService::obtener(pool, user).await.unwrap();
    (user, config)
}

/// Venta sincronizada con una línea, atada a la comanda del simulador.
async fn venta_sincronizada(pool: &PgPool, user: Uuid) -> (Uuid, i64) {
    let order_id = comanda_en_simulador().await;
    let id = Uuid::new_v4();
    sqlx::query(
        r"INSERT INTO ventas (id, user_id, fecha, descripcion, iva_porcentaje, turno, canal,
           metodo_pago, importe_base, importe_iva, bdp_synced, bdp_order_id)
          VALUES ($1, $2, CURRENT_DATE, 'Test modificaciones', 10, 'mediodia', 'comedor',
                  'efectivo', 1.00, 0, true, $3)",
    )
    .bind(id)
    .bind(user)
    .bind(order_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        r"INSERT INTO venta_lineas (id, venta_id, articulo_codigo, descripcion, cantidad,
           precio_unitario, iva_pct, descuento, invitacion, comentario)
          VALUES ($1, $2, '1001', 'Coca-Cola', 1, 1.00, 10, 0, false, '')",
    )
    .bind(Uuid::new_v4())
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
    (id, order_id)
}

fn linea(codigo: &str, descripcion: &str, cantidad: i64, precio: &str) -> CrearVentaLineaRequest {
    CrearVentaLineaRequest {
        articulo_codigo: Some(codigo.into()),
        descripcion: descripcion.into(),
        cantidad: Some(Decimal::from(cantidad)),
        precio_unitario: precio.parse().unwrap(),
        iva_pct: Some(Decimal::from(10)),
        descuento: Some(Decimal::ZERO),
        invitacion: Some(false),
        comentario: None,
    }
}

fn cambio(lineas: Vec<CrearVentaLineaRequest>) -> ActualizarVentaRequest {
    ActualizarVentaRequest {
        fecha: None,
        comensales: None,
        descripcion: None,
        iva_porcentaje: None,
        turno: Some(Turno::Mediodia),
        canal: Some(CanalVenta::Comedor),
        metodo_pago: Some(MetodoPago::Efectivo),
        importe_base: None,
        importe_iva: None,
        lineas: Some(lineas),
    }
}

/// Unidades de un artículo en la comanda del simulador (deltas incluidos).
fn unidades_de(orden: &Value, articulo: &str) -> f64 {
    orden["Items"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter(|i| i["Name"].as_str() == Some(articulo))
                .filter_map(|i| i["Units"].as_f64())
                .sum()
        })
        .unwrap_or(0.0)
}

/* ===== 1. Tres modificaciones encadenadas: ninguna se pierde ===== */

#[sqlx::test(migrations = "./migrations")]
async fn tres_modificaciones_seguidas_dejan_el_estado_final(pool: PgPool) {
    simulador_en_serie!();
    let (user, _config) = cuenta(&pool).await;
    let (venta_id, order_id) = venta_sincronizada(&pool, user).await;

    /* Tres cambios encadenados, cada uno sobre el estado anterior. */
    for cantidad in 2..=4 {
        VentaService::update(
            &pool,
            venta_id,
            user,
            cambio(vec![
                linea("1001", "Coca-Cola", cantidad, "1.00"),
                linea("1002", "Patatas", cantidad, "2.00"),
            ]),
        )
        .await
        .unwrap_or_else(|e| panic!("la modificación {cantidad} no puede perderse: {e:?}"));
    }

    /* Estado final en la web. */
    let lineas =
        glory_backend::repositories::VentaLineaRepository::listar_por_venta(&pool, venta_id)
            .await
            .expect("líneas locales");
    assert_eq!(lineas.len(), 2, "la web conserva sus dos líneas");
    assert!(
        lineas.iter().all(|l| l.cantidad == Decimal::from(4)),
        "la última modificación manda: {:?}",
        lineas.iter().map(|l| l.cantidad).collect::<Vec<_>>()
    );

    /* Estado final en el TPV: los deltas suman exactamente 4 de cada uno. */
    let orden = orden_en_simulador(order_id).await;
    assert!(
        (unidades_de(&orden, "Coca-Cola") - 4.0).abs() < 0.001,
        "Coca-Cola: {}",
        unidades_de(&orden, "Coca-Cola")
    );
    assert!(
        (unidades_de(&orden, "Patatas") - 4.0).abs() < 0.001,
        "Patatas: {}",
        unidades_de(&orden, "Patatas")
    );
}

/* ===== 2. Dos modificaciones simultáneas idénticas no duplican ===== */

#[sqlx::test(migrations = "./migrations")]
async fn dos_modificaciones_simultaneas_no_duplican_el_delta(pool: PgPool) {
    simulador_en_serie!();
    let (user, _config) = cuenta(&pool).await;
    let (venta_id, order_id) = venta_sincronizada(&pool, user).await;

    /* Las dos piden lo mismo a la vez. Sin serialización, cada una calcula su
     * delta contra el estado viejo (1 ud) y ambas mandan +2: el TPV acaba con
     * 5. Con la serialización, la segunda ve 3 y su delta es cero. */
    let peticion = || cambio(vec![linea("1001", "Coca-Cola", 3, "1.00")]);
    let (a, b) = tokio::join!(
        VentaService::update(&pool, venta_id, user, peticion()),
        VentaService::update(&pool, venta_id, user, peticion()),
    );
    a.expect("la primera modificación no puede fallar");
    b.expect("la segunda modificación no puede fallar");

    let orden = orden_en_simulador(order_id).await;
    assert!(
        (unidades_de(&orden, "Coca-Cola") - 3.0).abs() < 0.001,
        "el delta no puede aplicarse dos veces: {} unidades en el TPV",
        unidades_de(&orden, "Coca-Cola")
    );

    let lineas =
        glory_backend::repositories::VentaLineaRepository::listar_por_venta(&pool, venta_id)
            .await
            .expect("líneas locales");
    assert_eq!(lineas.len(), 1, "una sola línea en la web");
    assert_eq!(lineas[0].cantidad, Decimal::from(3));
}

/* ===== 3. Un corte tras escribir no repite el delta ya aplicado ===== */

#[sqlx::test(migrations = "./migrations")]
async fn corte_tras_aplicar_no_duplica_la_modificacion(pool: PgPool) {
    simulador_en_serie!();
    let (user, _config) = cuenta(&pool).await;
    let (venta_id, order_id) = venta_sincronizada(&pool, user).await;

    /* El simulador aplica la escritura y corta la respuesta: el ambiguo de
     * siempre, ahora reconciliado con `GetOrder`. */
    admin_post(
        "/__simulator/fault",
        json!({"path": "/API/Orders/Create", "mode": "apply_then_disconnect", "times": 1}),
    )
    .await;

    VentaService::update(
        &pool,
        venta_id,
        user,
        cambio(vec![linea("1001", "Coca-Cola", 3, "1.00")]),
    )
    .await
    .expect("la reconciliación reconoce que el cambio se aplicó");

    /* La comanda tiene el cambio una sola vez: 1 original + 2 de ajuste. */
    let orden = orden_en_simulador(order_id).await;
    assert!(
        (unidades_de(&orden, "Coca-Cola") - 3.0).abs() < 0.001,
        "el delta no puede aplicarse dos veces: {}",
        unidades_de(&orden, "Coca-Cola")
    );

    /* Y el ambiguo quedó cerrado: si siguiera abierto, la siguiente escritura
     * de la venta se bloquearía. */
    let abiertas: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bdp_audit_log \
         WHERE user_id = $1 AND datos_enviados ->> 'venta_id' = $2 \
           AND resultado IN ('pendiente', 'ambiguo')",
    )
    .bind(user)
    .bind(venta_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("consulta de auditorías");
    assert_eq!(abiertas, 0, "la reconciliación debe cerrar la modificación");
}

/* ===== 4. Un rechazo explícito de BDP no bloquea la venta ===== */

/* [21AB-5d] Lo observado en el TPV demo: una modificación de una comanda de
 * barra (Type 0) devuelve `[300033]` y BDP no cambia nada. La auditoría se
 * anotaba `ambiguo` —se reconciliaba como si el corte hubiera podido
 * aplicarse— y a partir de ahí `ensure_no_unresolved` bloqueaba toda escritura
 * de esa venta, para siempre. Un `Remote` (HTTP 200 con `ErrorMessage`) es un
 * rechazo de negocio: no escribió nada y tiene que quedar en `error`. */
#[sqlx::test(migrations = "./migrations")]
async fn un_rechazo_de_negocio_no_deja_la_venta_bloqueada(pool: PgPool) {
    simulador_en_serie!();
    let (user, _config) = cuenta(&pool).await;
    let (venta_id, _order_id) = venta_sincronizada(&pool, user).await;

    admin_post(
        "/__simulator/fault",
        json!({"Path": "/API/Orders/Create", "remote_error": "[300033]-EL IMPORTE TOTAL INDICADO DE LA COMANDA NO COINCIDE CON EL TEÓRICO"}),
    )
    .await;

    let error = VentaService::update(
        &pool,
        venta_id,
        user,
        cambio(vec![linea("1001", "Coca-Cola", 3, "1.00")]),
    )
    .await
    .expect_err("el rechazo de BDP revierte la edición local");
    assert!(
        error.to_string().contains("300033"),
        "el mensaje debe llevar el error del TPV: {error}"
    );

    let abiertas: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bdp_audit_log \
         WHERE user_id = $1 AND datos_enviados ->> 'venta_id' = $2 \
           AND resultado IN ('pendiente', 'ambiguo')",
    )
    .bind(user)
    .bind(venta_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("consulta de auditorías");
    assert_eq!(
        abiertas, 0,
        "un rechazo explícito no puede dejar auditoría abierta"
    );

    /* Sin el fallo inyectado, la siguiente edición pasa: la venta no quedó
     * bloqueada por el rechazo anterior. */
    VentaService::update(
        &pool,
        venta_id,
        user,
        cambio(vec![linea("1001", "Coca-Cola", 3, "1.00")]),
    )
    .await
    .expect("la venta sigue siendo editable tras un rechazo de negocio");
}

/* ===== 5. Un 200 sin aplicar no se da por hecho ===== */

/* [21AB-5d] El TPV demo contestó `OrderId` + `ErrorMessage` vacío a una
 * modificación de barra y la comanda quedó igual (total 1,05, una línea). Si la
 * web se lo cree, muestra un cambio que el TPV no tiene y el siguiente delta se
 * calcula contra un estado falso. La aplicación relee el total y revierte. */
#[sqlx::test(migrations = "./migrations")]
async fn un_ok_sin_aplicar_revierte_la_edicion(pool: PgPool) {
    simulador_en_serie!();
    let (user, _config) = cuenta(&pool).await;
    let (venta_id, order_id) = venta_sincronizada(&pool, user).await;

    admin_post(
        "/__simulator/fault",
        json!({"Path": "/API/Orders/Create", "ok_without_apply": true}),
    )
    .await;

    let error = VentaService::update(
        &pool,
        venta_id,
        user,
        cambio(vec![linea("1001", "Coca-Cola", 3, "1.00")]),
    )
    .await
    .expect_err("BDP no aplicó el cambio: la web no puede darlo por hecho");
    assert!(
        error.to_string().contains("no aplicó el cambio"),
        "mensaje esperado de verificación: {error}"
    );

    /* La comanda sigue como estaba y la web también. */
    let orden = orden_en_simulador(order_id).await;
    assert!(
        (unidades_de(&orden, "Coca-Cola") - 1.0).abs() < 0.001,
        "BDP no escribió nada: {}",
        unidades_de(&orden, "Coca-Cola")
    );
    let lineas =
        glory_backend::repositories::VentaLineaRepository::listar_por_venta(&pool, venta_id)
            .await
            .expect("líneas de la venta");
    assert_eq!(lineas.len(), 1);
    assert_eq!(
        lineas[0].cantidad,
        Decimal::ONE,
        "la web conserva su estado"
    );

    /* Y no quedó auditoría abierta que bloquee la siguiente edición. */
    let abiertas: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bdp_audit_log \
         WHERE user_id = $1 AND datos_enviados ->> 'venta_id' = $2 \
           AND resultado IN ('pendiente', 'ambiguo')",
    )
    .bind(user)
    .bind(venta_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("consulta de auditorías");
    assert_eq!(
        abiertas, 0,
        "un 200 sin aplicar no deja la venta en el aire"
    );
}
