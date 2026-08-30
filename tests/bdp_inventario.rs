/* [198A-1/D6] Tests de integración del endpoint de inventario (POST /api/bdp/inventario).
 *
 * Cubren el contrato de `registrar_inventario`:
 *   - resuelve solo artículos con código BDP numérico (los locales puros se omiten);
 *   - encola una única fila `stock/inventario` con el lote resuelto;
 *   - responde `enviados` / `omitidos_sin_bdp` correctamente;
 *   - rechaza el lote vacío con error de validación.
 * No hay envío HTTP real: el worker no envía nada en standalone (independencia). */

use axum::extract::State;
use axum::Json;
use glory_backend::config::AppConfig;
use glory_backend::errors::AppError;
use glory_backend::handlers::registrar_inventario;
use glory_backend::middleware::AuthUser;
use glory_backend::models::{
    InventarioLineaRequest, NotificacionEvent, RegistrarInventarioRequest, UserRole,
};
use glory_backend::repositories::ConfiguracionRepository;
use glory_backend::services::ServicioModoOperacion;
use glory_backend::AppState;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::str::FromStr;
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

async fn insertar_mapeo(pool: &PgPool, user_id: Uuid, glory: &str, bdp: &str) {
    sqlx::query(
        "INSERT INTO bdp_article_map (user_id, articulo_glory_codigo, articulo_bdp_codigo) \
         VALUES ($1, $2, $3)",
    )
    .bind(user_id)
    .bind(glory)
    .bind(bdp)
    .execute(pool)
    .await
    .expect("insertar mapeo");
}

#[sqlx::test(migrations = "./migrations")]
async fn inventario_encola_lote_y_omite_locales_puros(pool: PgPool) {
    let user_id = crear_usuario_y_config(&pool).await;
    insertar_mapeo(&pool, user_id, "ART-1", "90000123").await; // numérico -> se envía
    insertar_mapeo(&pool, user_id, "ART-2", "").await; // local puro -> se omite
    let state = make_app_state(pool.clone());
    let auth = AuthUser {
        user_id,
        role: UserRole::Admin,
        effective_role: UserRole::Admin,
        impersonator: None,
        trabajador_id: None,
    };

    let req = RegistrarInventarioRequest {
        articulos: vec![
            InventarioLineaRequest {
                articulo_glory_codigo: "ART-1".to_string(),
                unidades_contadas: Decimal::from_str("5").unwrap(),
            },
            InventarioLineaRequest {
                articulo_glory_codigo: "ART-2".to_string(),
                unidades_contadas: Decimal::from_str("7").unwrap(),
            },
        ],
    };

    let result = registrar_inventario(State(state), auth, Json(req)).await;
    let body = result.expect("inventario debe encolar sin error");
    assert_eq!(body.0["enviados"], 1);
    assert_eq!(body.0["omitidos_sin_bdp"], 1);

    /* Una sola fila activa stock/inventario con el lote resuelto. */
    let fila: (String, String, serde_json::Value) = sqlx::query_as(
        "SELECT dominio, operacion, payload_json FROM bdp_push_pendientes WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("fila encolada");

    assert_eq!(fila.0, "stock");
    assert_eq!(fila.1, "inventario");
    let articulos = fila.2["ArticlesList"].as_array().unwrap();
    assert_eq!(articulos.len(), 1);
    assert_eq!(articulos[0]["Article"], 90000123);
    assert_eq!(articulos[0]["Units"], "5");
}

#[sqlx::test(migrations = "./migrations")]
async fn inventario_sin_codigos_bdp_devuelve_validacion(pool: PgPool) {
    let user_id = crear_usuario_y_config(&pool).await;
    insertar_mapeo(&pool, user_id, "ART-2", "").await; // solo local puro
    let state = make_app_state(pool);
    let auth = AuthUser {
        user_id,
        role: UserRole::Admin,
        effective_role: UserRole::Admin,
        impersonator: None,
        trabajador_id: None,
    };
    let req = RegistrarInventarioRequest {
        articulos: vec![InventarioLineaRequest {
            articulo_glory_codigo: "ART-2".to_string(),
            unidades_contadas: Decimal::from_str("7").unwrap(),
        }],
    };
    let result = registrar_inventario(State(state), auth, Json(req)).await;
    match result {
        Err(AppError::Validation(msg)) => {
            assert!(msg.contains("código BDP numérico"), "mensaje: {msg}");
        }
        other => panic!("se esperaba AppError::Validation, se obtuvo {other:?}"),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn inventario_lote_vacio_devuelve_validacion(pool: PgPool) {
    let user_id = crear_usuario_y_config(&pool).await;
    let state = make_app_state(pool);
    let auth = AuthUser {
        user_id,
        role: UserRole::Admin,
        effective_role: UserRole::Admin,
        impersonator: None,
        trabajador_id: None,
    };
    let req = RegistrarInventarioRequest { articulos: vec![] };
    let result = registrar_inventario(State(state), auth, Json(req)).await;
    match result {
        Err(AppError::Validation(msg)) => {
            assert!(msg.contains("al menos un artículo"), "mensaje: {msg}");
        }
        other => panic!("se esperaba AppError::Validation, se obtuvo {other:?}"),
    }
}
