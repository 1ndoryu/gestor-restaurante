/* [128A-1/F8] Tests de integración de permisos operativos por acción (D8, M17).
 *
 * Enforcement en backend: con el default 'admin', un trabajador recibe 403 al
 * ajustar stock, anular ventas, editar catálogo o gestionar albaranes; al
 * ampliar el permiso en Configuración, el trabajador puede ejecutarlas.
 * La UI solo refleja el permiso; estos tests prueban el backend (M17).
 */

use axum::extract::{Path, State};
use axum::Json;
use glory_backend::config::AppConfig;
use glory_backend::errors::AppError;
use glory_backend::handlers::{
    actualizar_article_map, ajustar_stock, anular_venta, crear_article_map,
    crear_purchase_note_local, eliminar_article_map,
};
use glory_backend::middleware::AuthUser;
use glory_backend::models::{
    ActualizarBdpArticleMapRequest, ActualizarConfiguracionRequest, AjustarBdpArticleStockRequest,
    AnularVentaRequest, CrearBdpArticleMapRequest, CrearBdpPurchaseNoteRequest, NotificacionEvent,
    UserRole,
};
use glory_backend::repositories::ConfiguracionRepository;
use glory_backend::services::{ConfiguracionService, ServicioModoOperacion};
use glory_backend::AppState;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::str::FromStr;
use tokio::sync::broadcast;
use uuid::Uuid;

/* ── Helpers ─────────────────────────────────────────────────────────── */

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
    }
}

fn make_auth(user_id: Uuid, role: UserRole) -> AuthUser {
    AuthUser {
        user_id,
        role,
        effective_role: role,
        impersonator: None,
        trabajador_id: None,
    }
}

async fn config_con_permiso(
    pool: &PgPool,
    user_id: Uuid,
    campo: &str,
    valor: &str,
) -> Result<(), AppError> {
    let req = match campo {
        "permisos_catalogo_edicion" => ActualizarConfiguracionRequest {
            permisos_catalogo_edicion: Some(valor.to_string()),
            ..Default::default()
        },
        "permisos_stock_ajuste" => ActualizarConfiguracionRequest {
            permisos_stock_ajuste: Some(valor.to_string()),
            ..Default::default()
        },
        "permisos_albaranes_gestion" => ActualizarConfiguracionRequest {
            permisos_albaranes_gestion: Some(valor.to_string()),
            ..Default::default()
        },
        "permisos_anulacion_ventas" => ActualizarConfiguracionRequest {
            permisos_anulacion_ventas: Some(valor.to_string()),
            ..Default::default()
        },
        _ => unreachable!("campo no soportado: {campo}"),
    };
    ConfiguracionService::actualizar(pool, user_id, &req).await?;
    Ok(())
}

fn assert_forbidden<T>(result: &Result<T, AppError>) {
    match result {
        Err(AppError::Forbidden(_)) => {}
        Err(other) => panic!("se esperaba AppError::Forbidden, se obtuvo {other:?}"),
        Ok(_) => panic!("se esperaba AppError::Forbidden, se obtuvo Ok"),
    }
}

/* ── Default 'admin': trabajador recibe 403 ─────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn trabajador_recibe_403_ajustar_stock_con_default_admin(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config por defecto");
    let state = make_app_state(pool);
    let auth = make_auth(user_id, UserRole::Trabajador);
    let req = AjustarBdpArticleStockRequest {
        articulo_glory_codigo: "ART-001".to_string(),
        delta: Decimal::from_str("-2").unwrap(),
        motivo: "merma".to_string(),
        warehouse_id: None,
        idempotency_key: None,
    };
    let result = ajustar_stock(State(state), auth, Json(req)).await;
    assert_forbidden(&result);
}

#[sqlx::test(migrations = "./migrations")]
async fn trabajador_recibe_403_crear_article_map_con_default_admin(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config por defecto");
    let state = make_app_state(pool);
    let auth = make_auth(user_id, UserRole::Trabajador);
    let req = CrearBdpArticleMapRequest {
        articulo_glory_codigo: "ART-001".to_string(),
        articulo_bdp_codigo: Some("1001".to_string()),
        articulo_bdp_nombre: Some("Artículo de prueba".to_string()),
        descripcion: None,
        precio_tarifa1: None,
        iva_pct: None,
        departamento: None,
        familia: None,
        subfamilia: None,
        activo: None,
        barcode: None,
    };
    let result = crear_article_map(State(state), auth, Json(req)).await;
    assert_forbidden(&result);
}

#[sqlx::test(migrations = "./migrations")]
async fn trabajador_recibe_403_actualizar_y_eliminar_article_map_con_default_admin(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config por defecto");
    let state = make_app_state(pool);
    let auth_actualizar = make_auth(user_id, UserRole::Trabajador);
    let auth_eliminar = make_auth(user_id, UserRole::Trabajador);
    let id = Uuid::new_v4();

    let actualizar_req = ActualizarBdpArticleMapRequest {
        articulo_bdp_codigo: None,
        articulo_bdp_nombre: None,
        descripcion: None,
        precio_tarifa1: None,
        iva_pct: None,
        departamento: None,
        familia: None,
        subfamilia: None,
        activo: None,
        barcode: None,
    };
    let result = actualizar_article_map(
        State(state.clone()),
        auth_actualizar,
        Path(id),
        Json(actualizar_req),
    )
    .await;
    assert_forbidden(&result);

    let result = eliminar_article_map(State(state), auth_eliminar, Path(id)).await;
    assert_forbidden(&result);
}

#[sqlx::test(migrations = "./migrations")]
async fn trabajador_recibe_403_crear_purchase_note_con_default_admin(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config por defecto");
    let state = make_app_state(pool);
    let auth = make_auth(user_id, UserRole::Trabajador);
    let req = CrearBdpPurchaseNoteRequest {
        serie: None,
        numero: None,
        fecha: None,
        codigo_proveedor: None,
        nombre_proveedor: Some("Proveedor A".to_string()),
        total: Some(Decimal::from_str("100.00").unwrap()),
        lineas: None,
    };
    let result = crear_purchase_note_local(State(state), auth, Json(req)).await;
    assert_forbidden(&result);
}

#[sqlx::test(migrations = "./migrations")]
async fn trabajador_recibe_403_anular_venta_con_default_admin(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config por defecto");
    let state = make_app_state(pool);
    let auth = make_auth(user_id, UserRole::Trabajador);
    let req = AnularVentaRequest {
        motivo: Some("Prueba de permiso".to_string()),
        idempotency_key: None,
        anulacion_usuario: None,
    };
    let result = anular_venta(State(state), auth, Path(Uuid::new_v4()), Json(req)).await;
    assert_forbidden(&result);
}

/* ── Admin sin 403 ───────────────────────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn admin_puede_ajustar_stock_sin_403(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config por defecto");
    let state = make_app_state(pool);
    let auth = make_auth(user_id, UserRole::Admin);
    let req = AjustarBdpArticleStockRequest {
        articulo_glory_codigo: "ART-001".to_string(),
        delta: Decimal::from_str("5").unwrap(),
        motivo: "entrada".to_string(),
        warehouse_id: None,
        idempotency_key: None,
    };
    let result = ajustar_stock(State(state), auth, Json(req)).await;
    assert!(
        result.is_ok(),
        "admin con default puede ajustar stock: {result:?}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn admin_no_recibe_403_al_anular_venta_inexistente(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config por defecto");
    let state = make_app_state(pool);
    let auth = make_auth(user_id, UserRole::Admin);
    let req = AnularVentaRequest {
        motivo: Some("Prueba".to_string()),
        idempotency_key: None,
        anulacion_usuario: None,
    };
    let result = anular_venta(State(state), auth, Path(Uuid::new_v4()), Json(req)).await;
    /* El guard pasa (no es Forbidden); al no existir la venta, la anulación
     * falla con NotFound — eso demuestra que el permiso no bloquea al admin. */
    match result {
        Err(AppError::NotFound(_)) => {}
        Err(AppError::Forbidden(_)) => panic!("admin no debería recibir 403 por permiso"),
        Err(other) => panic!("se esperaba NotFound, se obtuvo {other:?}"),
        Ok(_) => panic!("la venta no existe, anular no debería devolver Ok"),
    }
}

/* ── Permisos ampliados: trabajador puede ────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn permisos_stock_ajuste_todos_permite_trabajador(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    config_con_permiso(&pool, user_id, "permisos_stock_ajuste", "todos")
        .await
        .expect("ampliar permiso");
    let state = make_app_state(pool);
    let auth = make_auth(user_id, UserRole::Trabajador);
    let req = AjustarBdpArticleStockRequest {
        articulo_glory_codigo: "ART-002".to_string(),
        delta: Decimal::from_str("3").unwrap(),
        motivo: "entrada".to_string(),
        warehouse_id: None,
        idempotency_key: None,
    };
    let result = ajustar_stock(State(state), auth, Json(req)).await;
    assert!(
        result.is_ok(),
        "con 'todos' el trabajador puede ajustar stock: {result:?}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn permisos_anulacion_ventas_todos_permite_trabajador(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    config_con_permiso(&pool, user_id, "permisos_anulacion_ventas", "todos")
        .await
        .expect("ampliar permiso");
    let state = make_app_state(pool);
    let auth = make_auth(user_id, UserRole::Trabajador);
    let req = AnularVentaRequest {
        motivo: Some("Prueba".to_string()),
        idempotency_key: None,
        anulacion_usuario: None,
    };
    let result = anular_venta(State(state), auth, Path(Uuid::new_v4()), Json(req)).await;
    match result {
        Err(AppError::NotFound(_)) => {}
        Err(AppError::Forbidden(_)) => panic!("con 'todos' el trabajador puede anular"),
        Err(other) => panic!("se esperaba NotFound, se obtuvo {other:?}"),
        Ok(_) => panic!("la venta no existe, anular no debería devolver Ok"),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn permisos_catalogo_edicion_admin_trabajador_permite_trabajador(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    config_con_permiso(
        &pool,
        user_id,
        "permisos_catalogo_edicion",
        "admin_trabajador",
    )
    .await
    .expect("ampliar permiso");
    let state = make_app_state(pool);
    let auth = make_auth(user_id, UserRole::Trabajador);
    let req = CrearBdpArticleMapRequest {
        articulo_glory_codigo: "ART-003".to_string(),
        articulo_bdp_codigo: None,
        articulo_bdp_nombre: None,
        descripcion: Some("Artículo local".to_string()),
        precio_tarifa1: None,
        iva_pct: None,
        departamento: None,
        familia: None,
        subfamilia: None,
        activo: Some(true),
        barcode: None,
    };
    let result = crear_article_map(State(state), auth, Json(req)).await;
    assert!(
        result.is_ok(),
        "con 'admin_trabajador' el trabajador puede crear mapeos: {result:?}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn permisos_albaranes_gestion_todos_permite_trabajador(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    config_con_permiso(&pool, user_id, "permisos_albaranes_gestion", "todos")
        .await
        .expect("ampliar permiso");
    let state = make_app_state(pool);
    let auth = make_auth(user_id, UserRole::Trabajador);
    let req = CrearBdpPurchaseNoteRequest {
        serie: None,
        numero: None,
        fecha: None,
        codigo_proveedor: None,
        nombre_proveedor: Some("Proveedor B".to_string()),
        total: Some(Decimal::from_str("50.00").unwrap()),
        lineas: None,
    };
    let result = crear_purchase_note_local(State(state), auth, Json(req)).await;
    assert!(
        result.is_ok(),
        "con 'todos' el trabajador puede crear albaranes: {result:?}"
    );
}

/* ── Configuración: validación y persistencia ────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn patch_config_con_permiso_invalido_devuelve_validation(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let req = ActualizarConfiguracionRequest {
        permisos_stock_ajuste: Some("superuser".to_string()),
        ..Default::default()
    };
    let result = ConfiguracionService::actualizar(&pool, user_id, &req).await;
    match result {
        Err(AppError::Validation(msg)) => assert!(msg.contains("permisos_stock_ajuste")),
        other => panic!("se esperaba AppError::Validation, se obtuvo {other:?}"),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn patch_config_persiste_permisos(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let req = ActualizarConfiguracionRequest {
        permisos_catalogo_edicion: Some("todos".to_string()),
        permisos_stock_ajuste: Some("admin_trabajador".to_string()),
        permisos_albaranes_gestion: Some("todos".to_string()),
        permisos_anulacion_ventas: Some("admin".to_string()),
        ..Default::default()
    };
    ConfiguracionService::actualizar(&pool, user_id, &req)
        .await
        .expect("guardar permisos");

    let config = ConfiguracionService::obtener(&pool, user_id)
        .await
        .expect("obtener config");
    assert_eq!(config.permisos_catalogo_edicion, "todos");
    assert_eq!(config.permisos_stock_ajuste, "admin_trabajador");
    assert_eq!(config.permisos_albaranes_gestion, "todos");
    assert_eq!(config.permisos_anulacion_ventas, "admin");
}
