/* [208A-2/C4] Tests de integración de la cola de sincronización (decisión D5):
 *   GET /api/bdp/push/pendientes  — listar filas (solo Admin).
 *   POST /api/bdp/push/:id/reintentar — reintento individual.
 *
 * Cubren:
 *   - listar devuelve las filas del usuario y 403 para no-admin;
 *   - reintentar una fila en modo efectivo standalone NO envía nada
 *     (omitidos_standalone=1, invariante de independencia);
 *   - reintentar una fila inexistente -> NotFound;
 *   - reintentar una fila ya sincronizada/descartada -> Validation.
 * No hay envío HTTP real: en standalone el reintento es no-op. */

use axum::extract::{Path, State};
use axum::Json;
use glory_backend::config::AppConfig;
use glory_backend::errors::AppError;
use glory_backend::handlers::{listar_pendientes_push, reintentar_fila};
use glory_backend::middleware::AuthUser;
use glory_backend::models::{NotificacionEvent, UserRole};
use glory_backend::repositories::ConfiguracionRepository;
use glory_backend::services::ServicioModoOperacion;
use glory_backend::AppState;
use sqlx::PgPool;
use tokio::sync::broadcast;
use uuid::Uuid;

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
    }
}

async fn crear_usuario_y_config(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let email = format!("test-{id}@example.com");
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(&email)
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("crear usuario de prueba");
    ConfiguracionRepository::obtener_o_crear(pool, id)
        .await
        .expect("config por defecto");
    id
}

async fn insertar_fila(pool: &PgPool, user_id: Uuid, estado: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO bdp_push_pendientes (id, user_id, dominio, entidad_id, operacion, payload_json, estado) \
         VALUES ($1, $2, 'articulo', 'ART-1', 'crear', '{}', $3)",
    )
    .bind(id)
    .bind(user_id)
    .bind(estado)
    .execute(pool)
    .await
    .expect("insertar fila de push");
    id
}

fn auth(user_id: Uuid, role: UserRole) -> AuthUser {
    AuthUser {
        user_id,
        role,
        effective_role: role,
        impersonator: None,
        trabajador_id: None,
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn listar_cola_devuelve_filas_solo_admin(pool: PgPool) {
    let user_id = crear_usuario_y_config(&pool).await;
    let fila_id = insertar_fila(&pool, user_id, "pendiente").await;
    let state = make_app_state(pool);

    /* Trabajador: 403. */
    let err = listar_pendientes_push(State(state.clone()), auth(user_id, UserRole::Trabajador))
        .await
        .expect_err("trabajador no puede listar la cola");
    assert!(matches!(err, AppError::Forbidden(_)), "error: {err:?}");

    /* Admin: devuelve la fila. */
    let filas = listar_pendientes_push(State(state), auth(user_id, UserRole::Admin))
        .await
        .expect("admin lista la cola")
        .0;
    assert_eq!(filas.len(), 1);
    assert_eq!(filas[0].id, fila_id);
    assert_eq!(filas[0].estado, "pendiente");
    assert_eq!(filas[0].dominio, "articulo");
}

#[sqlx::test(migrations = "./migrations")]
async fn reintentar_en_standalone_no_envia_nada(pool: PgPool) {
    let user_id = crear_usuario_y_config(&pool).await;
    let fila_id = insertar_fila(&pool, user_id, "pendiente").await;
    let state = make_app_state(pool);
    let auth = auth(user_id, UserRole::Admin);

    /* Config por defecto sin credenciales BDP -> modo efectivo standalone. */
    let resumen = reintentar_fila(State(state), auth, Path(fila_id))
        .await
        .expect("el reintento en standalone no falla")
        .0;
    assert_eq!(resumen.omitidos_standalone, 1, "standalone nunca envía");
    assert_eq!(resumen.procesados, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn reintentar_fila_inexistente_devuelve_not_found(pool: PgPool) {
    let user_id = crear_usuario_y_config(&pool).await;
    let state = make_app_state(pool);
    let auth = auth(user_id, UserRole::Admin);
    let err = reintentar_fila(State(state), auth, Path(Uuid::new_v4()))
        .await
        .expect_err("fila inexistente");
    assert!(matches!(err, AppError::NotFound(_)), "error: {err:?}");
}

#[sqlx::test(migrations = "./migrations")]
async fn reintentar_fila_ya_sincronizada_devuelve_validacion(pool: PgPool) {
    let user_id = crear_usuario_y_config(&pool).await;
    let fila_id = insertar_fila(&pool, user_id, "sincronizado").await;
    let state = make_app_state(pool);
    let auth = auth(user_id, UserRole::Admin);
    let err = reintentar_fila(State(state), auth, Path(fila_id))
        .await
        .expect_err("fila terminal no se reintenta");
    match err {
        AppError::Validation(msg) => {
            assert!(msg.contains("sincronizada o descartada"), "mensaje: {msg}");
        }
        other => panic!("se esperaba AppError::Validation, se obtuvo {other:?}"),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn reintentar_requiere_admin(pool: PgPool) {
    let user_id = crear_usuario_y_config(&pool).await;
    let fila_id = insertar_fila(&pool, user_id, "pendiente").await;
    let state = make_app_state(pool);
    let err = reintentar_fila(State(state), auth(user_id, UserRole::Trabajador), Path(fila_id))
        .await
        .expect_err("trabajador no reintenta");
    assert!(matches!(err, AppError::Forbidden(_)), "error: {err:?}");
}
