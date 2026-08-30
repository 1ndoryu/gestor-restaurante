/* [208A-2/C5] Test de la normalización H5: no se puede persistir un estado
 * contradictorio (modo_operacion=standalone con bdp_sync_enabled=true).
 * Al guardar esa combinación, el PATCH fuerza bdp_sync_enabled=false: el modo
 * independiente no sincroniza con BDP. */

use axum::extract::State;
use axum::Json;
use glory_backend::config::AppConfig;
use glory_backend::handlers::actualizar_configuracion;
use glory_backend::middleware::AuthUser;
use glory_backend::models::{
    ActualizarConfiguracionRequest, NotificacionEvent, UserRole,
};
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

#[sqlx::test(migrations = "./migrations")]
async fn standalone_con_sync_activo_se_normaliza_a_false(pool: PgPool) {
    let user_id = crear_usuario_y_config(&pool).await;
    let state = make_app_state(pool.clone());
    let auth = AuthUser {
        user_id,
        role: UserRole::Admin,
        effective_role: UserRole::Admin,
        impersonator: None,
        trabajador_id: None,
    };

    /* Guardado contradictorio: standalone + sync=true. */
    let req = ActualizarConfiguracionRequest {
        modo_operacion: Some("standalone".to_string()),
        bdp_sync_enabled: Some(true),
        ..Default::default()
    };
    let config = actualizar_configuracion(State(state), auth, Json(req))
        .await
        .expect("el PATCH no falla")
        .0;
    assert_eq!(config.modo_operacion, "standalone");
    assert!(!config.bdp_sync_enabled, "standalone no puede quedar con sync activo");

    /* Persistido en BD, no solo en la respuesta. */
    let (modo, sync): (String, bool) = sqlx::query_as(
        "SELECT modo_operacion, bdp_sync_enabled FROM configuracion_restaurante WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("leer config guardada");
    assert_eq!(modo, "standalone");
    assert!(!sync, "la BD no guarda el estado contradictorio");
}

#[sqlx::test(migrations = "./migrations")]
async fn standalone_sin_sync_se_conserva(pool: PgPool) {
    let user_id = crear_usuario_y_config(&pool).await;
    let state = make_app_state(pool.clone());
    let auth = AuthUser {
        user_id,
        role: UserRole::Admin,
        effective_role: UserRole::Admin,
        impersonator: None,
        trabajador_id: None,
    };
    let req = ActualizarConfiguracionRequest {
        modo_operacion: Some("standalone".to_string()),
        bdp_sync_enabled: Some(false),
        ..Default::default()
    };
    let config = actualizar_configuracion(State(state), auth, Json(req))
        .await
        .expect("el PATCH no falla")
        .0;
    assert_eq!(config.modo_operacion, "standalone");
    assert!(!config.bdp_sync_enabled);
}

#[sqlx::test(migrations = "./migrations")]
async fn modo_bdp_con_sync_activo_no_se_toca(pool: PgPool) {
    let user_id = crear_usuario_y_config(&pool).await;
    let state = make_app_state(pool.clone());
    let auth = AuthUser {
        user_id,
        role: UserRole::Admin,
        effective_role: UserRole::Admin,
        impersonator: None,
        trabajador_id: None,
    };
    let req = ActualizarConfiguracionRequest {
        modo_operacion: Some("bdp".to_string()),
        bdp_sync_enabled: Some(true),
        ..Default::default()
    };
    let config = actualizar_configuracion(State(state), auth, Json(req))
        .await
        .expect("el PATCH no falla")
        .0;
    assert_eq!(config.modo_operacion, "bdp");
    assert!(config.bdp_sync_enabled, "en modo bdp el sync se conserva");
}
