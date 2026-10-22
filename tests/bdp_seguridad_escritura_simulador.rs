// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
/* [10AA-4] Matriz de seguridad de escritura contra el simulador BDP.
 *
 * La pregunta que responde, caso por caso: con esta configuración de la cuenta,
 * ¿sale algo hacia el TPV y en qué queda el permiso guardado? El simulador
 * registra cada petición con su `Write` (true/false) y su estado (clientes,
 * comandas) es comparable, así que sirve de testigo: si algo se escribe, aquí
 * se ve; si no, no hay ni login.
 *
 * Puerto propio (18766) para no pelearse con `bdp_simulator_integration`.
 * Si no hay Python, los tres se saltan. */

use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use glory_backend::models::ActualizarConfiguracionRequest;
use glory_backend::services::bdp_push::{BdpPushService, DOMINIO_ARTICULO, OPERACION_CREAR};
use glory_backend::services::{BdpBackupService, BdpPushFlushService, ConfiguracionService};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

const PUERTO: u16 = 18766;
const URL: &str = "http://127.0.0.1:18766";
const CLAVE: &str = "clave-matriz-seguridad-18766";

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
    let hijo = Command::new("python")
        .arg(&servidor)
        .arg("--port")
        .arg(PUERTO.to_string())
        .arg("--admin-key")
        .arg(CLAVE)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let Ok(hijo) = hijo else {
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

macro_rules! simulador_o_salto {
    () => {
        if !asegurar_simulador().await {
            eprintln!("SKIP: Python o simulador no disponible");
            return;
        }
        admin_post("/__simulator/reset").await;
    };
}

async fn admin_post(ruta: &str) -> Value {
    reqwest::Client::new()
        .post(format!("{URL}{ruta}"))
        .header("X-Simulator-Key", CLAVE)
        .json(&json!({}))
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

/// Peticiones registradas por el simulador (login incluido) y cuántas escriben.
async fn historial() -> (usize, Vec<String>) {
    let cuerpo = admin_estado("/__simulator/history").await;
    let entradas = cuerpo["History"].as_array().cloned().unwrap_or_default();
    let escrituras: Vec<String> = entradas
        .iter()
        .filter(|e| e["Write"].as_bool().unwrap_or(false))
        .filter_map(|e| e["Path"].as_str().map(str::to_string))
        .collect();
    (entradas.len(), escrituras)
}

async fn cuenta(pool: &PgPool, modo: &str, modalidad: &str) -> Uuid {
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
            bdp_base_url: Some(URL.to_string()),
            bdp_sync_enabled: Some(true),
            bdp_auto_backup_before_write: Some(true),
            bdp_login: Some("local".to_string()),
            bdp_password: Some("secreto".to_string()),
            bdp_integrator_code: Some("SIM".to_string()),
            bdp_pos_id: Some(1),
            bdp_employee_id: Some(1),
            bdp_items_profile_id: Some(1),
            push_modalidad: Some(modalidad.to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("config de la cuenta");
    sqlx::query("UPDATE configuracion_restaurante SET bdp_sync_mode = $2 WHERE user_id = $1")
        .bind(user)
        .bind(modo)
        .execute(pool)
        .await
        .unwrap();
    user
}

async fn encolar_articulo(pool: &PgPool, user: Uuid, codigo: i64) {
    BdpPushService::encolar(
        pool,
        user,
        DOMINIO_ARTICULO,
        &codigo.to_string(),
        OPERACION_CREAR,
        &json!({
            "AutomaticCode": false,
            "ArticleData": { "ArtCode": codigo, "ArtDescription": "Plato simulado" },
            "AllProfiles": true
        }),
    )
    .await
    .unwrap();
}

async fn snapshot_vigente(pool: &PgPool, user: Uuid) {
    let config = ConfiguracionService::obtener(pool, user).await.unwrap();
    let target = BdpBackupService::canonical_target(&config).unwrap();
    let huella = BdpBackupService::connection_fingerprint(&config).unwrap();
    sqlx::query(
        "INSERT INTO bdp_snapshots \
         (user_id, tipo, direccion, trigger_tipo, datos, target_base_url, connection_fingerprint) \
         VALUES ($1, 'completo', 'bdp', 'manual', '{}'::jsonb, $2, $3)",
    )
    .bind(user)
    .bind(&target)
    .bind(&huella)
    .execute(pool)
    .await
    .unwrap();
}

async fn modo_guardado(pool: &PgPool, user: Uuid) -> String {
    sqlx::query_scalar("SELECT bdp_sync_mode FROM configuracion_restaurante WHERE user_id = $1")
        .bind(user)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn estado_tpv() -> Value {
    admin_estado("/__simulator/state").await
}

/* ===== 1. Solo lectura: el barrido no escribe ni cambia la configuración =====
 * Se ejercitan las dos vías que corren solas (el barrido de fondo y el reintento
 * por fila, que también arma) sin permiso. La acción explícita de una persona
 * sí escribe y vuelve a solo lectura: eso es el kill switch, y lo cubre el
 * bloque 2 junto con el modo `unidirectional`. */

#[sqlx::test(migrations = "./migrations")]
async fn solo_lectura_no_deja_que_nada_escriba_solo(pool: PgPool) {
    simulador_o_salto!();
    let antes = estado_tpv().await;

    let user = cuenta(&pool, "read_only", "automatico").await;
    snapshot_vigente(&pool, user).await;
    encolar_articulo(&pool, user, 90_000_101).await;

    BdpPushFlushService::sincronizar_cuentas_automaticas(&pool).await;
    let sin_permiso = BdpPushFlushService::flush(&pool, user, false)
        .await
        .unwrap();

    let (peticiones, escrituras) = historial().await;
    assert_eq!(peticiones, 0, "no debe ni autenticarse contra el TPV");
    assert!(escrituras.is_empty());
    assert_eq!(estado_tpv().await, antes, "el estado del TPV no cambia");
    assert_eq!(modo_guardado(&pool, user).await, "read_only");
    assert_eq!(sin_permiso.sincronizados, 0);
    assert_eq!(
        BdpPushService::listar_pendientes(&pool, user)
            .await
            .unwrap()
            .len(),
        1,
        "la fila sigue esperando"
    );
}

/* ===== 2. Unidireccional: el barrido no escribe; el botón sí, y vuelve a solo lectura ===== */

#[sqlx::test(migrations = "./migrations")]
async fn unidireccional_solo_escribe_por_una_accion_explicita(pool: PgPool) {
    simulador_o_salto!();

    let user = cuenta(&pool, "unidirectional", "automatico").await;
    snapshot_vigente(&pool, user).await;
    encolar_articulo(&pool, user, 90_000_102).await;

    BdpPushFlushService::sincronizar_cuentas_automaticas(&pool).await;
    let (peticiones, _) = historial().await;
    assert_eq!(
        peticiones, 0,
        "el barrido de fondo no debe escribir sin permiso permanente"
    );

    let forzado = BdpPushFlushService::flush(&pool, user, true).await.unwrap();
    assert_eq!(forzado.sincronizados, 1, "resumen: {forzado:?}");

    let (_, escrituras) = historial().await;
    assert_eq!(
        escrituras,
        vec!["/API/Articles/CreateAndUpdateProfiles".to_string()],
        "una acción explícita escribe exactamente una vez"
    );
    assert_eq!(
        modo_guardado(&pool, user).await,
        "read_only",
        "tras consumir el armado, la cuenta vuelve sola a solo lectura"
    );
}

/* ===== 3. Automático: el barrido escribe y el permiso no se degrada ===== */

#[sqlx::test(migrations = "./migrations")]
async fn automatico_escribe_en_el_barrido_y_conserva_el_modo(pool: PgPool) {
    simulador_o_salto!();

    let user = cuenta(&pool, "automatic", "automatico").await;
    /* Dos filas de la misma entidad: la segunda demuestra que no queda un
     * armado huérfano bloqueando el siguiente envío. */
    encolar_articulo(&pool, user, 90_000_103).await;

    let enviadas = BdpPushFlushService::sincronizar_cuentas_automaticas(&pool).await;
    assert_eq!(enviadas, 1);

    encolar_articulo(&pool, user, 90_000_103).await;
    let enviadas = BdpPushFlushService::sincronizar_cuentas_automaticas(&pool).await;
    assert_eq!(
        enviadas, 1,
        "el segundo envío de la misma entidad no se bloquea"
    );

    let (_, escrituras) = historial().await;
    assert_eq!(
        escrituras,
        vec![
            "/API/Articles/CreateAndUpdateProfiles".to_string(),
            "/API/Articles/CreateAndUpdateProfiles".to_string()
        ]
    );
    assert_eq!(
        modo_guardado(&pool, user).await,
        "automatic",
        "el modo concedido no cambia solo"
    );
}

/* ===== 4. El destino: un TPV que no está autorizado no recibe nada =====
 * La allowlist es la última barrera; aquí se comprueba que sigue activa para
 * un destino que no es bucle local. */

#[sqlx::test(migrations = "./migrations")]
async fn un_destino_fuera_de_la_allowlist_no_recibe_escrituras(pool: PgPool) {
    let user = cuenta(&pool, "automatic", "automatico").await;
    /* Host no bucle y no declarado en BDP_WRITE_ALLOWED_ORIGINS. */
    sqlx::query(
        "UPDATE configuracion_restaurante SET bdp_base_url = 'http://192.0.2.10:8068' WHERE user_id = $1",
    )
    .bind(user)
    .execute(&pool)
    .await
    .unwrap();
    encolar_articulo(&pool, user, 90_000_104).await;

    BdpPushFlushService::sincronizar_cuentas_automaticas(&pool).await;

    let pendientes = BdpPushService::listar_pendientes(&pool, user)
        .await
        .unwrap();
    assert_eq!(pendientes.len(), 1, "la fila queda pendiente, no se pierde");
    /* El motivo se guarda en la propia fila; se lee aparte porque el listado
     * público no lo trae. */
    let ultimo_error: Option<String> =
        sqlx::query_scalar("SELECT ultimo_error FROM bdp_push_pendientes WHERE user_id = $1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        ultimo_error
            .as_deref()
            .is_some_and(|e| e.contains("BDP_WRITE_ALLOWED_ORIGINS")),
        "el motivo debe señalar la allowlist de destinos; fue: {ultimo_error:?}"
    );
}
