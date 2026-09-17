// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [039A-1/H-P1-02] Test de regresión: fail-closed por modo efectivo en los
 * handlers que se conectan directamente al BDP (login/export).
 *
 * En `standalone` (config por defecto) NINGÚN flujo que requiera BDP puede
 * llegar a intentar red: el guard `exigir_modo_bdp` (config+cache M2/M3, nunca
 * hace red) rechaza con 422 antes de construir el cliente. Estos tests corren
 * sin BDP real y sin red externa; si el guard desapareciera, el intento de
 * login contra una URL inalcanzable devolvería Internal (transport), no el
 * bloqueo esperado, y el test fallaría.
 */

use axum::extract::State;
use axum::Json;
use glory_backend::config::AppConfig;
use glory_backend::errors::AppError;
use glory_backend::handlers::{
    explorar_bdp, importar_catalogo, importar_clientes_bdp, snapshot_completo, snapshot_parcial,
    sync_catalog, BdpCustomerImportRequest, SnapshotParcialRequest,
};
use glory_backend::middleware::AuthUser;
use glory_backend::models::{ActualizarConfiguracionRequest, NotificacionEvent, UserRole};
use glory_backend::repositories::ConfiguracionRepository;
use glory_backend::services::{BdpBackupService, ConfiguracionService, ServicioModoOperacion};
use glory_backend::AppState;
use sqlx::PgPool;
use tokio::sync::broadcast;
use uuid::Uuid;

/* ── Helpers (mismo patrón que tests/bdp_f8_permisos.rs) ────────────── */

async fn create_test_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let email = format!("test-{id}@example.com");
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(&email)
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("crear usuario de prueba");
    id
}

fn make_app_state(pool: PgPool) -> AppState {
    let (notif_tx, _): (broadcast::Sender<NotificacionEvent>, _) = broadcast::channel(16);
    AppState {
        pool,
        jwt_secret: "test-secret".to_string(),
        config: AppConfig {
            database_url: "postgres://localhost".to_string(),
            jwt_secret: "test-secret".to_string(),
            host: "127.0.0.1".to_string(),
            port: 3000,
            smtp: None,
            app_url: "http://localhost".to_string(),
            error_report_email: None,
        },
        notif_tx,
        modo_operacion: ServicioModoOperacion::default(),
        resumen_cache: Default::default(),
        listados_cache: Default::default(),
    }
}

fn make_auth(user_id: Uuid) -> AuthUser {
    AuthUser {
        user_id,
        role: UserRole::Admin,
        effective_role: UserRole::Admin,
        impersonator: None,
        trabajador_id: None,
    }
}

fn assert_modo_independiente<T>(result: &Result<T, AppError>) {
    match result {
        Err(AppError::Validation(msg)) if msg.contains("modo independiente") => {}
        Err(other) => {
            panic!("se esperaba bloqueo por modo independiente (422), se obtuvo {other:?}")
        }
        Ok(_) => panic!("se esperaba bloqueo por modo independiente, se obtuvo Ok"),
    }
}

/* ── Standalone (config por defecto): cero intentos de red ──────────── */

#[sqlx::test(migrations = "./migrations")]
async fn sync_catalog_bloqueado_en_standalone(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config por defecto");
    let state = make_app_state(pool);
    let auth = make_auth(user_id);

    let result = sync_catalog(State(state), auth).await;
    assert_modo_independiente(&result);
}

#[sqlx::test(migrations = "./migrations")]
async fn importar_catalogo_bloqueado_en_standalone(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config por defecto");
    let state = make_app_state(pool);
    let auth = make_auth(user_id);

    let result = importar_catalogo(State(state), auth).await;
    assert_modo_independiente(&result);
}

#[sqlx::test(migrations = "./migrations")]
async fn importar_clientes_bdp_bloqueado_en_standalone(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config por defecto");
    let state = make_app_state(pool);
    let auth = make_auth(user_id);
    let req = BdpCustomerImportRequest {
        aplicar: false,
        confirmacion: None,
    };

    let result = importar_clientes_bdp(State(state), auth, Json(req)).await;
    assert_modo_independiente(&result);
}

/* [039A-1/H-P1-03] Explorar BDP y snapshots "completo"/"parcial" descargan
 * de BDP (red real): en `standalone` deben fallar-cerrados con credenciales
 * configuradas, sin llegar a construir el cliente. */

#[sqlx::test(migrations = "./migrations")]
async fn explorar_bdp_bloqueado_en_standalone_con_credenciales(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let update = ActualizarConfiguracionRequest {
        bdp_base_url: Some("http://127.0.0.1:1".to_string()),
        bdp_login: Some("test".to_string()),
        bdp_password: Some("test".to_string()),
        bdp_integrator_code: Some("1".to_string()),
        ..Default::default()
    };
    ConfiguracionService::actualizar(&pool, user_id, &update)
        .await
        .expect("configurar credenciales");

    let state = make_app_state(pool);
    let auth = make_auth(user_id);
    let result = explorar_bdp(State(state), auth).await;
    assert_modo_independiente(&result);
}

#[sqlx::test(migrations = "./migrations")]
async fn snapshot_completo_bloqueado_en_standalone_con_credenciales(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let update = ActualizarConfiguracionRequest {
        bdp_base_url: Some("http://127.0.0.1:1".to_string()),
        bdp_login: Some("test".to_string()),
        bdp_password: Some("test".to_string()),
        bdp_integrator_code: Some("1".to_string()),
        ..Default::default()
    };
    ConfiguracionService::actualizar(&pool, user_id, &update)
        .await
        .expect("configurar credenciales");

    let state = make_app_state(pool);
    let auth = make_auth(user_id);
    let result = snapshot_completo(State(state), auth, Json(None)).await;
    assert_modo_independiente(&result);
}

#[sqlx::test(migrations = "./migrations")]
async fn snapshot_parcial_bloqueado_en_standalone_con_credenciales(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let update = ActualizarConfiguracionRequest {
        bdp_base_url: Some("http://127.0.0.1:1".to_string()),
        bdp_login: Some("test".to_string()),
        bdp_password: Some("test".to_string()),
        bdp_integrator_code: Some("1".to_string()),
        ..Default::default()
    };
    ConfiguracionService::actualizar(&pool, user_id, &update)
        .await
        .expect("configurar credenciales");

    let state = make_app_state(pool);
    let auth = make_auth(user_id);
    let req = SnapshotParcialRequest {
        tipos: vec!["articulos".to_string()],
        notas: None,
    };
    let result = snapshot_parcial(State(state), auth, Json(req)).await;
    assert_modo_independiente(&result);
}

/* ── Positivo: en modo Bdp efectivo el guard NO bloquea (sin red externa) ──
 * Con URL inalcanzable en loopback, el flujo pasa el guard y falla en el
 * login con transporte (AppError::Internal), nunca con el mensaje del guard. */

#[sqlx::test(migrations = "./migrations")]
async fn sync_catalog_no_bloqueado_en_modo_bdp(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config por defecto");
    /* modo_operacion='bdp' explícito → modo efectivo Bdp aunque la URL sea
     * inalcanzable (el modo deriva de config, no de conectividad). */
    let update = ActualizarConfiguracionRequest {
        modo_operacion: Some("bdp".to_string()),
        bdp_base_url: Some("http://127.0.0.1:1".to_string()),
        bdp_login: Some("test".to_string()),
        bdp_password: Some("test".to_string()),
        bdp_integrator_code: Some("1".to_string()),
        ..Default::default()
    };
    ConfiguracionService::actualizar(&pool, user_id, &update)
        .await
        .expect("actualizar config");

    let state = make_app_state(pool);
    let auth = make_auth(user_id);
    let result = sync_catalog(State(state), auth).await;

    match result {
        Err(AppError::Internal(msg)) => {
            assert!(
                msg.contains("login BDP"),
                "se esperaba error de transporte en el login, se obtuvo: {msg}"
            );
        }
        Err(other) => panic!(
            "en modo Bdp el guard no debe bloquear; se esperaba Internal de login, se obtuvo {other:?}"
        ),
        Ok(_) => panic!("URL inalcanzable: sync_catalog no podía devolver Ok"),
    }
}

/* [039A-1/H-P1-04] Snapshot local "Glory" con tipo `ventas` (0 llamadas
 * BDP, permitido en standalone): la exportación SQL referenciaba columnas
 * inexistentes en el esquema real (`total`, `estado`) y devolvía 500.
 * Regresión: debe exportar filas reales con `total` derivado de
 * `importe_base + importe_iva` contra el esquema vigente de migraciones. */
#[sqlx::test(migrations = "./migrations")]
async fn snapshot_glory_ventas_exporta_con_esquema_real(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config por defecto");

    sqlx::query(
        "INSERT INTO ventas (user_id, fecha, turno, canal, metodo_pago, importe_base, \
         importe_iva, descripcion) VALUES ($1, CURRENT_DATE, 'manana', 'comedor', \
         'efectivo', 10.00, 2.10, 'test H-P1-04')",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("insertar venta de prueba");

    let snapshot = BdpBackupService::snapshot_glory(
        &pool,
        user_id,
        &["ventas".to_string()],
        Some("test H-P1-04".to_string()),
    )
    .await
    .expect("el snapshot local de ventas debe exportar sin columnas fantasma");

    let ventas = snapshot
        .datos
        .get("ventas")
        .and_then(|v| v.as_array())
        .expect("datos.ventas debe ser un array");
    assert_eq!(ventas.len(), 1, "debe exportar la venta insertada");
    let total = ventas[0]
        .get("total")
        .and_then(|t| t.as_f64())
        .expect("total derivado presente");
    assert_eq!(total, 12.10, "total = importe_base + importe_iva");
    let fantasma = ventas[0].get("estado").is_some();
    assert!(!fantasma, "no debe exportar la columna fantasma `estado`");
}
