/* [128A-1/F7] Tests de integración de menús/packs locales (D2, §4.10).
 * CRUD local 100% operativo sin BDP. No dependen de feature flags (M12).
 * Usan #[sqlx::test] con migraciones aplicadas y no contactan con BDP real. */

use axum::extract::{Path, Query, State};
use axum::Json;
use glory_backend::config::AppConfig;
use glory_backend::errors::AppError;
use glory_backend::handlers::{
    actualizar_menu_local, crear_menu_local, eliminar_menu_local, listar_menus_locales,
    obtener_menu_local,
};
use glory_backend::middleware::AuthUser;
use glory_backend::models::{
    ActualizarBdpMenuLocalRequest, BdpMenuLocalLineaRequest, BdpMenuLocalListParams,
    BdpMenuLocalTipo, CrearBdpMenuLocalRequest, NotificacionEvent, UserRole,
};
use glory_backend::repositories::{BdpMenuLocalRepository, ConfiguracionRepository};
use glory_backend::services::ServicioModoOperacion;
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

fn linea(descripcion: &str, cantidad: &str, precio: &str) -> BdpMenuLocalLineaRequest {
    BdpMenuLocalLineaRequest {
        articulo_codigo: Some("ART-001".to_string()),
        descripcion: descripcion.to_string(),
        cantidad: Some(Decimal::from_str(cantidad).unwrap()),
        precio_unitario: Some(Decimal::from_str(precio).unwrap()),
    }
}

fn crear_request(
    nombre: &str,
    tipo: &str,
    lineas: Vec<BdpMenuLocalLineaRequest>,
) -> CrearBdpMenuLocalRequest {
    CrearBdpMenuLocalRequest {
        tipo: tipo.to_string(),
        nombre: nombre.to_string(),
        descripcion: Some("Descripción de prueba".to_string()),
        precio: None,
        activo: None,
        lineas,
    }
}

fn default_filters() -> BdpMenuLocalListParams {
    BdpMenuLocalListParams {
        tipo: None,
        activo: None,
        busqueda: None,
    }
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

fn make_auth(user_id: Uuid) -> AuthUser {
    AuthUser {
        user_id,
        role: UserRole::Admin,
        effective_role: UserRole::Admin,
        impersonator: None,
        trabajador_id: None,
    }
}

/* ── Tests de repositorio ────────────────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn crear_menu_con_lineas_guarda_detalle_y_precio_calculado(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    let req = crear_request(
        "Menú del día",
        "menu",
        vec![
            linea("Coca-Cola", "2", "1.50"),
            linea("Hamburguesa", "1", "5.00"),
        ],
    );
    let menu = BdpMenuLocalRepository::crear(&pool, user_id, &req)
        .await
        .expect("crear menú local");

    assert_eq!(menu.nombre, "Menú del día");
    assert_eq!(menu.tipo, BdpMenuLocalTipo::Menu);
    assert!(menu.activo, "por defecto el menú está activo");
    assert_eq!(menu.precio, Decimal::from_str("8.00").unwrap());
    assert_eq!(menu.lineas.len(), 2);
    assert_eq!(menu.lineas[0].descripcion, "Coca-Cola");
    assert_eq!(menu.lineas[0].articulo_codigo.as_deref(), Some("ART-001"));
}

#[sqlx::test(migrations = "./migrations")]
async fn listar_filtra_por_tipo_activo_y_busqueda(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    BdpMenuLocalRepository::crear(
        &pool,
        user_id,
        &crear_request("Menú infantil", "menu", vec![linea("Nuggets", "1", "4.00")]),
    )
    .await
    .unwrap();
    let pack = BdpMenuLocalRepository::crear(
        &pool,
        user_id,
        &crear_request(
            "Pack cumpleaños",
            "pack",
            vec![linea("Tarta", "1", "12.00")],
        ),
    )
    .await
    .unwrap();
    BdpMenuLocalRepository::crear(
        &pool,
        user_id,
        &crear_request("Menú desactivado", "menu", vec![linea("Sopa", "1", "3.00")]),
    )
    .await
    .unwrap();
    /* Desactivar el último menú. */
    let actualizar = ActualizarBdpMenuLocalRequest {
        tipo: None,
        nombre: None,
        descripcion: None,
        precio: None,
        activo: Some(false),
        lineas: None,
    };
    BdpMenuLocalRepository::actualizar(&pool, pack.id, user_id, &actualizar)
        .await
        .unwrap();

    /* Solo packs. */
    let filtro_pack = BdpMenuLocalListParams {
        tipo: Some("pack".to_string()),
        activo: None,
        busqueda: None,
    };
    let packs = BdpMenuLocalRepository::listar(&pool, user_id, &filtro_pack)
        .await
        .unwrap();
    assert_eq!(packs.len(), 1);
    assert_eq!(packs[0].nombre, "Pack cumpleaños");

    /* Solo activos: pack cumpleaños + menú infantil. */
    let filtro_activos = BdpMenuLocalListParams {
        tipo: None,
        activo: Some(true),
        busqueda: None,
    };
    let activos = BdpMenuLocalRepository::listar(&pool, user_id, &filtro_activos)
        .await
        .unwrap();
    assert_eq!(activos.len(), 2);

    /* Búsqueda por nombre. */
    let filtro_busqueda = BdpMenuLocalListParams {
        tipo: None,
        activo: None,
        busqueda: Some("cumple".to_string()),
    };
    let buscados = BdpMenuLocalRepository::listar(&pool, user_id, &filtro_busqueda)
        .await
        .unwrap();
    assert_eq!(buscados.len(), 1);
    assert_eq!(buscados[0].nombre, "Pack cumpleaños");
}

#[sqlx::test(migrations = "./migrations")]
async fn actualizar_reemplaza_lineas_y_aplica_coalesce(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    let menu = BdpMenuLocalRepository::crear(
        &pool,
        user_id,
        &crear_request("Menú original", "menu", vec![linea("Agua", "1", "1.00")]),
    )
    .await
    .unwrap();

    /* Solo cambia nombre y líneas: el resto (COALESCE) se conserva. */
    let req = ActualizarBdpMenuLocalRequest {
        tipo: Some("pack".to_string()),
        nombre: Some("Pack renovado".to_string()),
        descripcion: None,
        precio: None,
        activo: None,
        lineas: Some(vec![
            linea("Cerveza", "2", "2.50"),
            linea("Patatas", "1", "1.75"),
        ]),
    };
    let ok = BdpMenuLocalRepository::actualizar(&pool, menu.id, user_id, &req)
        .await
        .unwrap();
    assert!(ok);

    let updated = BdpMenuLocalRepository::find_by_id(&pool, menu.id, user_id)
        .await
        .unwrap()
        .expect("menú actualizado existe");
    assert_eq!(updated.nombre, "Pack renovado");
    assert_eq!(updated.tipo, BdpMenuLocalTipo::Pack);
    assert_eq!(
        updated.descripcion.as_deref(),
        Some("Descripción de prueba")
    );
    assert_eq!(updated.lineas.len(), 2);
    assert_eq!(updated.lineas[0].descripcion, "Cerveza");
    /* 2 × 2.50 + 1 × 1.75 = 6.75: el precio se recalcula desde las líneas. */
    assert_eq!(updated.precio, Decimal::from_str("6.75").unwrap());
}

#[sqlx::test(migrations = "./migrations")]
async fn eliminar_borra_menu_y_sus_lineas(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    let menu = BdpMenuLocalRepository::crear(
        &pool,
        user_id,
        &crear_request("Menú a borrar", "menu", vec![linea("Agua", "1", "1.00")]),
    )
    .await
    .unwrap();

    let ok = BdpMenuLocalRepository::eliminar(&pool, menu.id, user_id)
        .await
        .unwrap();
    assert!(ok);

    assert!(
        BdpMenuLocalRepository::find_by_id(&pool, menu.id, user_id)
            .await
            .unwrap()
            .is_none(),
        "el menú debe desaparecer"
    );
    let lineas: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM bdp_menu_local_lineas WHERE menu_id = $1")
            .bind(menu.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(lineas, 0, "las líneas se borran por CASCADE");
}

#[sqlx::test(migrations = "./migrations")]
async fn aislamiento_por_usuario(pool: PgPool) {
    let user_a = create_test_user(&pool).await;
    let user_b = create_test_user(&pool).await;

    let menu_a = BdpMenuLocalRepository::crear(
        &pool,
        user_a,
        &crear_request("Solo de A", "menu", vec![linea("Agua", "1", "1.00")]),
    )
    .await
    .unwrap();

    /* El usuario B no ve ni modifica el menú de A. */
    assert!(BdpMenuLocalRepository::find_by_id(&pool, menu_a.id, user_b)
        .await
        .unwrap()
        .is_none());
    let listado_b = BdpMenuLocalRepository::listar(&pool, user_b, &default_filters())
        .await
        .unwrap();
    assert!(listado_b.is_empty());
    let ok = BdpMenuLocalRepository::eliminar(&pool, menu_a.id, user_b)
        .await
        .unwrap();
    assert!(!ok, "B no puede eliminar el menú de A");
}

#[sqlx::test(migrations = "./migrations")]
async fn doble_nombre_mismo_tipo_viola_unique(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    BdpMenuLocalRepository::crear(
        &pool,
        user_id,
        &crear_request("Menú repetido", "menu", vec![linea("Agua", "1", "1.00")]),
    )
    .await
    .unwrap();

    let resultado = BdpMenuLocalRepository::crear(
        &pool,
        user_id,
        &crear_request("Menú repetido", "menu", vec![linea("Pan", "1", "0.50")]),
    )
    .await;
    assert!(
        resultado.is_err(),
        "el UNIQUE(user_id, tipo, nombre) debe rechazar el duplicado"
    );
    let err = resultado.unwrap_err();
    let es_unico = err
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code)
        .is_some_and(|c| c == "23505");
    assert!(
        es_unico,
        "se esperaba unique_violation 23505, se obtuvo {err:?}"
    );
}

/* ── Tests de handlers ───────────────────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn handler_crear_menu_local_funciona_sin_flags_en_standalone(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    /* Configuración por defecto (modo 'auto' sin BDP configurado) = modo
     * efectivo standalone: el CRUD local NO consulta feature flags (M12). */
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración por defecto");

    let state = make_app_state(pool.clone());
    let auth = make_auth(user_id);
    let req = crear_request("Menú por handler", "menu", vec![linea("Café", "2", "1.20")]);
    let result = crear_menu_local(State(state), auth, Json(req)).await;

    assert!(
        result.is_ok(),
        "en modo standalone el CRUD local no depende de flags"
    );
    let Json(menu) = result.unwrap();
    assert_eq!(menu.nombre, "Menú por handler");
    assert_eq!(menu.lineas.len(), 1);
    assert_eq!(menu.precio, Decimal::from_str("2.40").unwrap());
}

#[sqlx::test(migrations = "./migrations")]
async fn handler_crear_rechaza_nombre_vacio(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let state = make_app_state(pool);
    let auth = make_auth(user_id);
    let req = crear_request("   ", "menu", vec![linea("Café", "1", "1.20")]);
    let result = crear_menu_local(State(state), auth, Json(req)).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Validation(msg) => assert!(msg.contains("nombre")),
        other => panic!("se esperaba AppError::Validation, se obtuvo {other:?}"),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn handler_crear_rechaza_sin_lineas(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let state = make_app_state(pool);
    let auth = make_auth(user_id);
    let req = crear_request("Menú vacío", "menu", vec![]);
    let result = crear_menu_local(State(state), auth, Json(req)).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Validation(msg) => assert!(msg.contains("línea")),
        other => panic!("se esperaba AppError::Validation, se obtuvo {other:?}"),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn handler_crear_tipo_invalido_rechaza(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let state = make_app_state(pool);
    let auth = make_auth(user_id);
    let req = crear_request("Raro", "combo", vec![linea("Café", "1", "1.20")]);
    let result = crear_menu_local(State(state), auth, Json(req)).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Validation(msg) => assert!(msg.contains("tipo")),
        other => panic!("se esperaba AppError::Validation, se obtuvo {other:?}"),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn handler_detalle_devuelve_404_si_no_existe(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let state = make_app_state(pool);
    let auth = make_auth(user_id);
    let result = obtener_menu_local(State(state), auth, Path(Uuid::new_v4())).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::NotFound(msg) => assert!(msg.contains("no encontrado")),
        other => panic!("se esperaba AppError::NotFound, se obtuvo {other:?}"),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn handler_actualizar_devuelve_404_si_no_existe(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let state = make_app_state(pool);
    let auth = make_auth(user_id);
    let req = ActualizarBdpMenuLocalRequest {
        tipo: None,
        nombre: Some("Nuevo nombre".to_string()),
        descripcion: None,
        precio: None,
        activo: None,
        lineas: None,
    };
    let result = actualizar_menu_local(State(state), auth, Path(Uuid::new_v4()), Json(req)).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::NotFound(msg) => assert!(msg.contains("no encontrado")),
        other => panic!("se esperaba AppError::NotFound, se obtuvo {other:?}"),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn handler_duplicado_devuelve_conflicto(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let state = make_app_state(pool.clone());
    let auth = make_auth(user_id);

    let req = crear_request("Menú único", "menu", vec![linea("Agua", "1", "1.00")]);
    assert!(crear_menu_local(
        State(make_app_state(pool.clone())),
        make_auth(user_id),
        Json(req.clone())
    )
    .await
    .is_ok());

    let result = crear_menu_local(State(state), auth, Json(req)).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::Conflict(msg) => assert!(msg.contains("ese nombre")),
        other => panic!("se esperaba AppError::Conflict, se obtuvo {other:?}"),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn handler_listar_devuelve_menus_del_usuario(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    BdpMenuLocalRepository::crear(
        &pool,
        user_id,
        &crear_request("Listado", "menu", vec![linea("Agua", "1", "1.00")]),
    )
    .await
    .unwrap();

    let state = make_app_state(pool);
    let auth = make_auth(user_id);
    let result = listar_menus_locales(State(state), auth, Query(default_filters())).await;
    assert!(result.is_ok());
    let Json(menus) = result.unwrap();
    assert_eq!(menus.len(), 1);
    assert_eq!(menus[0].nombre, "Listado");
}

#[sqlx::test(migrations = "./migrations")]
async fn handler_eliminar_borra_y_devuelve_404_en_segunda_llamada(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let menu = BdpMenuLocalRepository::crear(
        &pool,
        user_id,
        &crear_request("A eliminar", "pack", vec![linea("Tarta", "1", "10.00")]),
    )
    .await
    .unwrap();

    let state = make_app_state(pool.clone());
    let auth = make_auth(user_id);
    let result = eliminar_menu_local(State(state), auth, Path(menu.id)).await;
    assert!(result.is_ok());

    let state2 = make_app_state(pool);
    let result2 = eliminar_menu_local(State(state2), make_auth(user_id), Path(menu.id)).await;
    match result2.unwrap_err() {
        AppError::NotFound(msg) => assert!(msg.contains("no encontrado")),
        other => panic!("se esperaba AppError::NotFound, se obtuvo {other:?}"),
    }
}
