/* [208A-2/C3] Tests de integración del conteo de inventario persistido
 * (POST /api/bdp/inventario/conteos, decisiones D3/D4).
 *
 * Cubren el contrato de `crear_conteo_inventario`:
 *   - persiste el conteo (cabecera + líneas con esperado/contado/diferencia);
 *   - aplica la diferencia al stock local con motivo 'conteo' (D4);
 *   - es idempotente por clave: reenviar la misma idempotency_key devuelve el
 *     conteo ya guardado sin volver a aplicar (aplicadas=0);
 *   - rechaza con Validation si una línea dejaría el stock negativo y revierte
 *     el conteo completo (rollback atómico);
 *   - encola el lote a BDP solo para líneas con código BDP numérico.
 * No hay envío HTTP real: el worker no envía nada en standalone (independencia). */

use axum::extract::State;
use axum::Json;
use glory_backend::config::AppConfig;
use glory_backend::errors::AppError;
use glory_backend::handlers::crear_conteo_inventario;
use glory_backend::middleware::AuthUser;
use glory_backend::models::{
    CrearConteoInventarioRequest, InventarioLineaRequest, NotificacionEvent, UserRole,
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

async fn insertar_stock(pool: &PgPool, user_id: Uuid, glory: &str, stock: &str) {
    sqlx::query(
        "INSERT INTO bdp_article_stock \
            (id, user_id, articulo_glory_codigo, warehouse_id, warehouse_name, stock, ajustado_local) \
         VALUES ($1, $2, $3, '0', 'General', $4, true)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(glory)
    .bind(Decimal::from_str(stock).expect("stock numérico válido"))
    .execute(pool)
    .await
    .expect("insertar stock local");
}

fn auth_admin(user_id: Uuid) -> AuthUser {
    AuthUser {
        user_id,
        role: UserRole::Admin,
        effective_role: UserRole::Admin,
        impersonator: None,
        trabajador_id: None,
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn conteo_persiste_aplica_y_encola(pool: PgPool) {
    let user_id = crear_usuario_y_config(&pool).await;
    insertar_mapeo(&pool, user_id, "ART-1", "90000123").await; // con código BDP -> se encola
    insertar_mapeo(&pool, user_id, "ART-2", "").await; // local puro -> se omite del envío
    insertar_stock(&pool, user_id, "ART-1", "10").await; // esperado 10
    let state = make_app_state(pool.clone());
    let auth = auth_admin(user_id);

    let req = CrearConteoInventarioRequest {
        observaciones: Some("recuento semanal".to_string()),
        idempotency_key: Some("conteo-sesion-1".to_string()),
        articulos: vec![
            InventarioLineaRequest {
                articulo_glory_codigo: "ART-1".to_string(),
                unidades_contadas: Decimal::from_str("7").unwrap(), // -3
            },
            InventarioLineaRequest {
                articulo_glory_codigo: "ART-2".to_string(),
                unidades_contadas: Decimal::from_str("5").unwrap(), // esperado 0 -> +5
            },
        ],
    };

    let result = crear_conteo_inventario(State(state), auth, Json(req)).await;
    let body = result.expect("conteo debe guardarse sin error").0;
    assert!(!body.reutilizado);
    assert_eq!(body.aplicadas, 2, "ambas líneas con diferencia se aplican al stock");
    assert_eq!(body.encolados, 1, "solo ART-1 tiene código BDP");
    assert_eq!(body.omitidos_sin_bdp, 1);
    assert_eq!(body.lineas.len(), 2);
    assert_eq!(body.conteo.estado, "aplicado");

    /* Stock local aplicado: ART-1 10-3=7, ART-2 0+5=5 (base creada con esperado). */
    let stock: Vec<(String, Decimal)> = sqlx::query_as(
        "SELECT articulo_glory_codigo, stock FROM bdp_article_stock \
         WHERE user_id = $1 ORDER BY articulo_glory_codigo",
    )
    .bind(user_id)
    .fetch_all(&pool)
    .await
    .expect("leer stock local");
    assert_eq!(stock.len(), 2);
    assert!(stock.contains(&("ART-1".to_string(), Decimal::from_str("7").unwrap())), "stock: {stock:?}");
    assert!(stock.contains(&("ART-2".to_string(), Decimal::from_str("5").unwrap())), "stock: {stock:?}");

    /* Auditoría con motivo 'conteo' (origen local). */
    let audit: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bdp_audit_log \
         WHERE user_id = $1 AND operacion = 'stock_ajuste' AND origen_operacion = 'local' \
           AND datos_enviados->>'motivo' = 'conteo'",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("contar auditoría");
    assert_eq!(audit, 2, "una auditoría por línea aplicada");

    /* Cola: una única fila stock/inventario con solo ART-1. */
    let (dominio, operacion, payload): (String, String, serde_json::Value) = sqlx::query_as(
        "SELECT dominio, operacion, payload_json FROM bdp_push_pendientes WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("fila encolada");
    assert_eq!(dominio, "stock");
    assert_eq!(operacion, "inventario");
    let articulos = payload["ArticlesList"].as_array().unwrap();
    assert_eq!(articulos.len(), 1);
    assert_eq!(articulos[0]["Article"], 90000123);
    assert_eq!(articulos[0]["Units"], "7");
}

#[sqlx::test(migrations = "./migrations")]
async fn conteo_misma_clave_no_aplica_dos_veces(pool: PgPool) {
    let user_id = crear_usuario_y_config(&pool).await;
    insertar_mapeo(&pool, user_id, "ART-1", "").await;
    insertar_stock(&pool, user_id, "ART-1", "10").await;
    let state = make_app_state(pool.clone());

    let req = CrearConteoInventarioRequest {
        observaciones: None,
        idempotency_key: Some("misma-sesion".to_string()),
        articulos: vec![InventarioLineaRequest {
            articulo_glory_codigo: "ART-1".to_string(),
            unidades_contadas: Decimal::from_str("4").unwrap(), // -6
        }],
    };

    let auth2 = AuthUser {
        user_id,
        role: UserRole::Admin,
        effective_role: UserRole::Admin,
        impersonator: None,
        trabajador_id: None,
    };
    let first = crear_conteo_inventario(State(state.clone()), auth_admin(user_id), Json(req.clone()))
        .await
        .expect("primer guardado")
        .0;
    assert_eq!(first.aplicadas, 1);

    let second = crear_conteo_inventario(State(state), auth2, Json(req))
        .await
        .expect("segundo guardado con la misma clave")
        .0;
    assert!(second.reutilizado, "la misma clave no vuelve a aplicar");
    assert_eq!(second.aplicadas, 0);

    let stock: Decimal = sqlx::query_scalar(
        "SELECT stock FROM bdp_article_stock WHERE user_id = $1 AND articulo_glory_codigo = 'ART-1'",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("leer stock");
    assert_eq!(stock, Decimal::from_str("4").unwrap(), "el stock se aplica una sola vez");
    let conteos: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bdp_conteos_inventario WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("contar conteos");
    assert_eq!(conteos, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn conteo_stock_negativo_rechaza_y_revierte(pool: PgPool) {
    let user_id = crear_usuario_y_config(&pool).await;
    insertar_mapeo(&pool, user_id, "ART-1", "").await;
    insertar_stock(&pool, user_id, "ART-1", "10").await;
    let state = make_app_state(pool.clone());
    let auth = auth_admin(user_id);

    /* Unidades contadas negativas no son válidas y dejarían el stock negativo. */
    let req = CrearConteoInventarioRequest {
        observaciones: None,
        idempotency_key: None,
        articulos: vec![InventarioLineaRequest {
            articulo_glory_codigo: "ART-1".to_string(),
            unidades_contadas: Decimal::from_str("-5").unwrap(), // 10 + (-15) = -5
        }],
    };
    let result = crear_conteo_inventario(State(state), auth, Json(req)).await;
    match result {
        Err(AppError::Validation(msg)) => {
            assert!(msg.contains("quedaría en"), "mensaje: {msg}");
        }
        other => panic!("se esperaba AppError::Validation, se obtuvo {other:?}"),
    }

    /* Rollback atómico: ni el conteo ni las líneas ni la auditoría quedan. */
    let conteos: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bdp_conteos_inventario WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("contar conteos");
    assert_eq!(conteos, 0, "el conteo se revierte completo");
    let stock: Decimal = sqlx::query_scalar(
        "SELECT stock FROM bdp_article_stock WHERE user_id = $1 AND articulo_glory_codigo = 'ART-1'",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("leer stock");
    assert_eq!(stock, Decimal::from_str("10").unwrap(), "el stock no se toca");
}

#[sqlx::test(migrations = "./migrations")]
async fn conteo_vacio_devuelve_validacion(pool: PgPool) {
    let user_id = crear_usuario_y_config(&pool).await;
    let state = make_app_state(pool);
    let auth = auth_admin(user_id);
    let req = CrearConteoInventarioRequest {
        observaciones: None,
        idempotency_key: None,
        articulos: vec![],
    };
    let result = crear_conteo_inventario(State(state), auth, Json(req)).await;
    match result {
        Err(AppError::Validation(msg)) => {
            assert!(msg.contains("al menos un artículo"), "mensaje: {msg}");
        }
        other => panic!("se esperaba AppError::Validation, se obtuvo {other:?}"),
    }
}
