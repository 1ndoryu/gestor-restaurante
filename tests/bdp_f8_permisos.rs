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
    actualizar_article_map, actualizar_purchase_note_local, ajustar_stock, anular_venta,
    conciliar_purchase_note, crear_article_map, crear_purchase_note_local, eliminar_article_map,
    eliminar_purchase_note_local, eliminar_venta, factura_local, marcar_borrador_purchase_note,
    pago_parcial_local, FacturaLocalRequest, PagoLocalRequest,
};
use glory_backend::middleware::AuthUser;
use glory_backend::models::{
    ActualizarBdpArticleMapRequest, ActualizarBdpPurchaseNoteRequest,
    ActualizarConfiguracionRequest, AjustarBdpArticleStockRequest, AnularVentaRequest,
    BdpPurchaseNoteDraftRequest, BdpPurchaseNoteReconcileRequest, CrearBdpArticleMapRequest,
    CrearBdpPurchaseNoteRequest, NotificacionEvent, UserRole,
};
use glory_backend::repositories::ConfiguracionRepository;
use glory_backend::services::{
    verificar_permiso, AccionPermiso, ConfiguracionService, ServicioModoOperacion,
};
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
        "permisos_pagos_locales" => ActualizarConfiguracionRequest {
            permisos_pagos_locales: Some(valor.to_string()),
            ..Default::default()
        },
        "permisos_facturacion_local" => ActualizarConfiguracionRequest {
            permisos_facturacion_local: Some(valor.to_string()),
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
        permisos_pagos_locales: Some("admin_trabajador".to_string()),
        permisos_facturacion_local: Some("todos".to_string()),
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
    assert_eq!(config.permisos_pagos_locales, "admin_trabajador");
    assert_eq!(config.permisos_facturacion_local, "todos");
}

/* ── Correcciones de la 2a revisión (F8-1..F8-4) ────────────────────── */

/* F8-1: pagos parciales locales y factura local son operaciones monetarias
 * (F6): con el default 'admin' un Trabajador recibe 403 aunque el modo
 * efectivo sea standalone. */
#[sqlx::test(migrations = "./migrations")]
async fn trabajador_recibe_403_pago_parcial_local_con_default_admin(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config por defecto");
    let state = make_app_state(pool);
    let auth = make_auth(user_id, UserRole::Trabajador);
    let req = PagoLocalRequest {
        amount: Decimal::from_str("10.00").unwrap(),
        tender_id: 1,
        confirmacion: "PAGO LOCAL cualquiera 10.00".to_string(),
        idempotency_key: None,
    };
    let result = pago_parcial_local(State(state), auth, Path(Uuid::new_v4()), Json(req)).await;
    assert_forbidden(&result);
}

#[sqlx::test(migrations = "./migrations")]
async fn trabajador_recibe_403_factura_local_con_default_admin(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config por defecto");
    let state = make_app_state(pool);
    let auth = make_auth(user_id, UserRole::Trabajador);
    let req = FacturaLocalRequest {
        confirmacion: "FACTURA LOCAL cualquiera".to_string(),
        idempotency_key: None,
    };
    let result = factura_local(State(state), auth, Path(Uuid::new_v4()), Json(req)).await;
    assert_forbidden(&result);
}

/* F8-2: el DELETE de ventas es escritura sensible (histórico fiscal local):
 * reusa el permiso de anulación; con default 'admin' un Trabajador recibe 403
 * antes de tocar el servicio. */
#[sqlx::test(migrations = "./migrations")]
async fn trabajador_recibe_403_eliminar_venta_con_default_admin(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config por defecto");
    let state = make_app_state(pool);
    let auth = make_auth(user_id, UserRole::Trabajador);
    let result = eliminar_venta(State(state), auth, Path(Uuid::new_v4())).await;
    assert_forbidden(&result);
}

/* F8-2: el guard pasa para Admin (la venta no existe → NotFound, no 403). */
#[sqlx::test(migrations = "./migrations")]
async fn admin_no_recibe_403_al_eliminar_venta_inexistente(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config por defecto");
    let state = make_app_state(pool);
    let auth = make_auth(user_id, UserRole::Admin);
    let result = eliminar_venta(State(state), auth, Path(Uuid::new_v4())).await;
    match result {
        Err(AppError::NotFound(_)) => {}
        Err(AppError::Forbidden(_)) => panic!("admin no debería recibir 403 por permiso"),
        Err(other) => panic!("se esperaba NotFound, se obtuvo {other:?}"),
        Ok(_) => panic!("la venta no existe, eliminar no debería devolver Ok"),
    }
}

/* F8-3: los 4 handlers de albaranes que faltaban en la cobertura 403 (el
 * guard ya existía con AlbaranesGestion; se fija el hueco por endpoint). */
#[sqlx::test(migrations = "./migrations")]
async fn trabajador_recibe_403_actualizar_purchase_note_con_default_admin(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config por defecto");
    let state = make_app_state(pool);
    let auth = make_auth(user_id, UserRole::Trabajador);
    let req = ActualizarBdpPurchaseNoteRequest {
        numero: None,
        fecha: None,
        codigo_proveedor: None,
        nombre_proveedor: None,
        total: None,
        lineas: None,
    };
    let result =
        actualizar_purchase_note_local(State(state), auth, Path(Uuid::new_v4()), Json(req)).await;
    assert_forbidden(&result);
}

#[sqlx::test(migrations = "./migrations")]
async fn trabajador_recibe_403_eliminar_purchase_note_con_default_admin(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config por defecto");
    let state = make_app_state(pool);
    let auth = make_auth(user_id, UserRole::Trabajador);
    let result = eliminar_purchase_note_local(State(state), auth, Path(Uuid::new_v4())).await;
    assert_forbidden(&result);
}

#[sqlx::test(migrations = "./migrations")]
async fn trabajador_recibe_403_marcar_borrador_purchase_note_con_default_admin(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config por defecto");
    let state = make_app_state(pool);
    let auth = make_auth(user_id, UserRole::Trabajador);
    let result = marcar_borrador_purchase_note(
        State(state),
        auth,
        Path(Uuid::new_v4()),
        Json(BdpPurchaseNoteDraftRequest {}),
    )
    .await;
    assert_forbidden(&result);
}

#[sqlx::test(migrations = "./migrations")]
async fn trabajador_recibe_403_conciliar_purchase_note_con_default_admin(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config por defecto");
    let state = make_app_state(pool);
    let auth = make_auth(user_id, UserRole::Trabajador);
    let req = BdpPurchaseNoteReconcileRequest {
        gasto_existente_id: None,
        categoria_id: None,
    };
    let result = conciliar_purchase_note(State(state), auth, Path(Uuid::new_v4()), Json(req)).await;
    assert_forbidden(&result);
}

/* F8-1: ampliar el permiso habilita al Trabajador (pasa el guard y llega al
 * servicio: la venta no existe → NotFound, no 403). */
#[sqlx::test(migrations = "./migrations")]
async fn permisos_pagos_locales_todos_permite_trabajador(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    config_con_permiso(&pool, user_id, "permisos_pagos_locales", "todos")
        .await
        .expect("ampliar permiso");
    let state = make_app_state(pool);
    let auth = make_auth(user_id, UserRole::Trabajador);
    let id = Uuid::new_v4();
    let req = PagoLocalRequest {
        amount: Decimal::from_str("10.00").unwrap(),
        tender_id: 1,
        confirmacion: format!("PAGO LOCAL {id} 10.00"),
        idempotency_key: None,
    };
    let result = pago_parcial_local(State(state), auth, Path(id), Json(req)).await;
    match result {
        Err(AppError::NotFound(_)) => {}
        Err(AppError::Forbidden(_)) => panic!("con 'todos' el trabajador puede pagar local"),
        Err(other) => panic!("se esperaba NotFound, se obtuvo {other:?}"),
        Ok(_) => panic!("la venta no existe, el pago no debería devolver Ok"),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn permisos_facturacion_local_todos_permite_trabajador(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    config_con_permiso(&pool, user_id, "permisos_facturacion_local", "todos")
        .await
        .expect("ampliar permiso");
    let state = make_app_state(pool);
    let auth = make_auth(user_id, UserRole::Trabajador);
    let id = Uuid::new_v4();
    let req = FacturaLocalRequest {
        confirmacion: format!("FACTURA LOCAL {id}"),
        idempotency_key: None,
    };
    let result = factura_local(State(state), auth, Path(id), Json(req)).await;
    match result {
        Err(AppError::NotFound(_)) => {}
        Err(AppError::Forbidden(_)) => panic!("con 'todos' el trabajador puede facturar local"),
        Err(other) => panic!("se esperaba NotFound, se obtuvo {other:?}"),
        Ok(_) => panic!("la venta no existe, la factura no debería devolver Ok"),
    }
}

/* F8-4: verificar_permiso es lectura pura: sin fila de configuración aplica
 * fail-closed 'admin' y NO crea la fila (obtener_o_crear era escritura en
 * cada request protegido). */
#[sqlx::test(migrations = "./migrations")]
async fn verificar_permiso_sin_config_no_crea_fila_y_falla_cerrado(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    /* Sin fila de configuración: Admin pasa (fail-closed al default 'admin'),
     * Trabajador recibe 403. */
    let admin = make_auth(user_id, UserRole::Admin);
    verificar_permiso(&pool, AccionPermiso::PagosLocales, &admin)
        .await
        .expect("admin sin fila de config pasa (default fail-closed 'admin')");

    let trabajador = make_auth(user_id, UserRole::Trabajador);
    let result = verificar_permiso(&pool, AccionPermiso::PagosLocales, &trabajador).await;
    assert_forbidden(&result);

    /* La lectura no debe haber creado la fila de configuración. */
    let filas: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM configuracion_restaurante WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .expect("contar config");
    assert_eq!(
        filas, 0,
        "verificar_permiso no debe crear config (lectura pura)"
    );
}
