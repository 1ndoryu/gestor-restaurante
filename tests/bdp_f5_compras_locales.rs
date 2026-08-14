/* [128A-1/F5] Tests de integración DB de las correcciones de compras locales:
 * F5-1 serie local forzada al prefijo reservado (M18) + sync que nunca pisa
 * locales, F5-2 numeración secuencial por (user_id, serie) con reintento,
 * F5-3 total vs desglose de líneas, F5-4 fallback de desglose logueado y
 * F5-5 fecha inválida y 23505 mapeados. No contactan con BDP real. */

use axum::extract::{Path, State};
use axum::Json;
use glory_backend::config::AppConfig;
use glory_backend::errors::AppError;
use glory_backend::handlers::{
    actualizar_purchase_note_local, conciliar_purchase_note, crear_purchase_note_local,
};
use glory_backend::middleware::AuthUser;
use glory_backend::models::{
    ActualizarBdpPurchaseNoteRequest, BdpPurchaseNoteEstado, BdpPurchaseNoteLineaLocal,
    BdpPurchaseNoteReconcileRequest, CrearBdpPurchaseNoteRequest, NotificacionEvent, UserRole,
};
use glory_backend::repositories::{BdpPurchaseNoteRepository, ConfiguracionRepository};
use glory_backend::services::bdp_weblink_catalog::BdpPurchaseNoteData;
use glory_backend::AppState;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::str::FromStr;
use tokio::sync::broadcast;
use uuid::Uuid;

/* ── Helpers ─────────────────────────────────────────────────────────── */

async fn create_test_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let email = format!("f5-test-{id}@example.com");
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
        modo_operacion: glory_backend::services::ServicioModoOperacion::default(),
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

fn request_local(
    serie: Option<&str>,
    numero: Option<&str>,
    fecha: Option<&str>,
    total: Option<Decimal>,
    lineas: Option<Vec<BdpPurchaseNoteLineaLocal>>,
) -> CrearBdpPurchaseNoteRequest {
    CrearBdpPurchaseNoteRequest {
        serie: serie.map(str::to_string),
        numero: numero.map(str::to_string),
        fecha: fecha.map(str::to_string),
        codigo_proveedor: None,
        nombre_proveedor: Some("Proveedor F5".to_string()),
        total,
        lineas,
    }
}

fn linea(descripcion: &str, cantidad: &str, precio: &str, iva: &str) -> BdpPurchaseNoteLineaLocal {
    BdpPurchaseNoteLineaLocal {
        descripcion: descripcion.to_string(),
        cantidad: Decimal::from_str(cantidad).unwrap(),
        precio_unitario: Decimal::from_str(precio).unwrap(),
        iva_pct: Decimal::from_str(iva).unwrap(),
    }
}

fn sample_bdp_data(serie: &str, numero: &str, total: Decimal) -> BdpPurchaseNoteData {
    BdpPurchaseNoteData {
        serie_albaran: Some(serie.to_string()),
        num_albaran: Some(numero.to_string()),
        fecha_albaran: Some("2026-08-01".to_string()),
        cod_proveedor: Some(serde_json::json!("PROV-F5")),
        nom_proveedor: Some("Proveedor BDP F5".to_string()),
        total_albaran: Some(total),
        extra: serde_json::json!({}),
    }
}

/* ── F5-1: serie local reservada (M18) ──────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn serie_local_fuera_del_prefijo_reservado_rechazada(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config");

    /* Repo: serie "BDP-01" (prefijo no reservado) → error Protocol. */
    let req = request_local(
        Some("BDP-01"),
        Some("1"),
        Some("2026-08-01"),
        Some(Decimal::from_str("10.00").unwrap()),
        None,
    );
    let err = BdpPurchaseNoteRepository::crear_local(&pool, user_id, &req)
        .await
        .expect_err("serie no reservada debe rechazarse");
    assert!(
        matches!(err, sqlx::Error::Protocol(ref msg) if msg.contains("serie_local_prefijo_invalido")),
        "error inesperado: {err:?}"
    );

    /* Handler: el Protocol se mapea a Validation (422) con mensaje legible. */
    let state = make_app_state(pool.clone());
    let auth = make_auth(user_id);
    let result = crear_purchase_note_local(State(state), auth, Json(req)).await;
    assert!(
        matches!(result, Err(AppError::Validation(ref msg)) if msg.contains("prefijo reservado")),
        "el handler debe devolver validación: {result:?}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn serie_por_defecto_es_la_reservada(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config");

    let note = BdpPurchaseNoteRepository::crear_local(
        &pool,
        user_id,
        &request_local(None, None, Some("2026-08-01"), None, None),
    )
    .await
    .expect("crear con serie por defecto");
    assert_eq!(note.serie, "L");
    assert_eq!(note.origen, "local");
}

#[sqlx::test(migrations = "./migrations")]
async fn upsert_bdp_nunca_pisa_albaran_local(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config");

    let req = request_local(
        Some("L"),
        Some("7"),
        Some("2026-08-01"),
        Some(Decimal::from_str("100.00").unwrap()),
        None,
    );
    let local = BdpPurchaseNoteRepository::crear_local(&pool, user_id, &req)
        .await
        .expect("crear local");

    /* Sync BDP con la MISMA clave natural (L, 7) y total distinto. */
    let insertado = BdpPurchaseNoteRepository::upsert_from_bdp(
        &pool,
        user_id,
        &sample_bdp_data("L", "7", Decimal::from_str("500.00").unwrap()),
    )
    .await
    .expect("upsert bdp");
    assert!(!insertado, "el sync no debe tocar filas locales");

    let tras_sync = BdpPurchaseNoteRepository::find_by_id(&pool, local.id, user_id)
        .await
        .expect("buscar")
        .expect("la fila local sigue existiendo");
    assert_eq!(tras_sync.total, Decimal::from_str("100.00").ok());
    assert_eq!(tras_sync.origen, "local");
    assert!(matches!(tras_sync.estado, BdpPurchaseNoteEstado::Pendiente));
}

/* ── F5-2: numeración secuencial por (user_id, serie) ───────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn numeracion_secuencial_por_serie_y_usuario(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config");

    /* Serie por defecto L: números 1 y 2 (secuencial por serie, no global). */
    let n1 = BdpPurchaseNoteRepository::crear_local(
        &pool,
        user_id,
        &request_local(None, None, Some("2026-08-01"), None, None),
    )
    .await
    .expect("primera nota local");
    assert_eq!(n1.numero, "1");

    let n2 = BdpPurchaseNoteRepository::crear_local(
        &pool,
        user_id,
        &request_local(None, None, Some("2026-08-01"), None, None),
    )
    .await
    .expect("segunda nota local");
    assert_eq!(n2.numero, "2");

    /* Otra serie local arranca su propio secuencial en 1. */
    let otra = BdpPurchaseNoteRepository::crear_local(
        &pool,
        user_id,
        &request_local(Some("L-FRUTAS"), None, Some("2026-08-01"), None, None),
    )
    .await
    .expect("nota en otra serie");
    assert_eq!(otra.serie, "L-FRUTAS");
    assert_eq!(otra.numero, "1");

    /* Número explícito se respeta (no se recalcula). */
    let expl = BdpPurchaseNoteRepository::crear_local(
        &pool,
        user_id,
        &request_local(Some("L"), Some("99"), Some("2026-08-01"), None, None),
    )
    .await
    .expect("nota con número explícito");
    assert_eq!(expl.numero, "99");
}

/* ── F5-3: total explícito vs desglose de líneas ────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn total_discrepante_con_lineas_rechazado(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config");

    /* Líneas suman 22.00 (base 20 + IVA 2); total explícito 30.00 → error. */
    let req = request_local(
        None,
        None,
        Some("2026-08-01"),
        Some(Decimal::from_str("30.00").unwrap()),
        Some(vec![linea("Tomate", "2", "10.00", "10")]),
    );

    let err = BdpPurchaseNoteRepository::crear_local(&pool, user_id, &req)
        .await
        .expect_err("total discrepante debe fallar");
    assert!(
        matches!(err, sqlx::Error::Protocol(ref msg) if msg.contains("no coincide")),
        "error inesperado: {err:?}"
    );

    /* Handler: 422 con mensaje accionable (F5-3). */
    let state = make_app_state(pool.clone());
    let auth = make_auth(user_id);
    let result = crear_purchase_note_local(State(state), auth, Json(req)).await;
    assert!(
        matches!(result, Err(AppError::Validation(ref msg)) if msg.contains("no coincide")),
        "el handler debe devolver validación: {result:?}"
    );
}

/* ── F5-5: fecha inválida y duplicado 23505 → 409 ───────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn fecha_invalida_rechazada_en_crear_y_actualizar(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config");

    let state = make_app_state(pool.clone());

    /* Crear con fecha malformada → 422, no silenciosa. */
    let req_bad = request_local(
        None,
        None,
        Some("2026-13-40"),
        Some(Decimal::from_str("10.00").unwrap()),
        None,
    );
    let result = crear_purchase_note_local(State(state), make_auth(user_id), Json(req_bad)).await;
    assert!(
        matches!(result, Err(AppError::Validation(ref msg)) if msg.contains("YYYY-MM-DD")),
        "fecha inválida debe rechazarse: {result:?}"
    );

    /* Actualizar con fecha malformada → 422. */
    let note = BdpPurchaseNoteRepository::crear_local(
        &pool,
        user_id,
        &request_local(None, None, Some("2026-08-01"), None, None),
    )
    .await
    .expect("crear local válido");
    let update = ActualizarBdpPurchaseNoteRequest {
        numero: None,
        fecha: Some("01/08/2026".to_string()),
        codigo_proveedor: None,
        nombre_proveedor: None,
        total: None,
        lineas: None,
    };
    let result = actualizar_purchase_note_local(
        State(make_app_state(pool.clone())),
        make_auth(user_id),
        Path(note.id),
        Json(update),
    )
    .await;
    assert!(
        matches!(result, Err(AppError::Validation(ref msg)) if msg.contains("YYYY-MM-DD")),
        "fecha inválida en update debe rechazarse: {result:?}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn numero_duplicado_mapeado_a_conflicto_409(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config");

    let req = request_local(
        Some("L"),
        Some("7"),
        Some("2026-08-01"),
        Some(Decimal::from_str("10.00").unwrap()),
        None,
    );
    let first = crear_purchase_note_local(
        State(make_app_state(pool.clone())),
        make_auth(user_id),
        Json(req.clone()),
    )
    .await;
    assert!(first.is_ok(), "primer alta ok: {first:?}");

    /* Misma serie y número → 23505 → 409 legible (F5-5), no 500 genérico. */
    let dup = crear_purchase_note_local(
        State(make_app_state(pool.clone())),
        make_auth(user_id),
        Json(req),
    )
    .await;
    assert!(
        matches!(dup, Err(AppError::Conflict(ref msg)) if msg.contains("duplicado")),
        "duplicado debe mapearse a 409: {dup:?}"
    );
}

/* ── F5-4: conciliación sin desglose → fallback logueado (total, IVA 0) ─ */

#[sqlx::test(migrations = "./migrations")]
async fn conciliacion_sin_desglose_usa_total_con_iva_cero(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config");

    /* Albarán local SIN líneas (solo total): datos_bdp = {} → no hay desglose. */
    let note = BdpPurchaseNoteRepository::crear_local(
        &pool,
        user_id,
        &request_local(
            Some("L"),
            Some("1"),
            Some("2026-08-01"),
            Some(Decimal::from_str("50.00").unwrap()),
            None,
        ),
    )
    .await
    .expect("crear local sin líneas");
    assert!(
        BdpPurchaseNoteRepository::marcar_borrador(&pool, note.id, user_id)
            .await
            .expect("marcar borrador")
    );

    let result = conciliar_purchase_note(
        State(make_app_state(pool.clone())),
        make_auth(user_id),
        Path(note.id),
        Json(BdpPurchaseNoteReconcileRequest {
            gasto_existente_id: None,
            categoria_id: None,
        }),
    )
    .await;
    assert!(result.is_ok(), "conciliación sin desglose ok: {result:?}");

    /* El gasto registra base=total e IVA=0 (camino del fallback). */
    let (base, iva): (Decimal, Decimal) = sqlx::query_as(
        "SELECT importe_base, importe_iva FROM gastos \
         WHERE user_id = $1 AND numero_documento = $2",
    )
    .bind(user_id)
    .bind("L-1")
    .fetch_one(&pool)
    .await
    .expect("gasto creado por la conciliación");
    assert_eq!(base, Decimal::from_str("50.00").unwrap());
    assert_eq!(iva, Decimal::ZERO);
}

/* ── Revisión de que el total se guarda SIEMPRE desde las líneas ─────── */

#[sqlx::test(migrations = "./migrations")]
async fn total_guardado_desde_lineas_con_total_coincidente(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config");

    /* Líneas suman 29.26 y el total explícito coincide → se guarda 29.26. */
    let note = BdpPurchaseNoteRepository::crear_local(
        &pool,
        user_id,
        &request_local(
            None,
            None,
            Some("2026-08-01"),
            Some(Decimal::from_str("29.26").unwrap()),
            Some(vec![
                linea("Tomate", "2", "10.00", "10"),
                linea("Pan", "3", "2.00", "21"),
            ]),
        ),
    )
    .await
    .expect("total coincidente aceptado");
    assert_eq!(note.total, Decimal::from_str("29.26").ok());
    assert_eq!(note.datos_bdp["importe_base"], serde_json::json!("26.00"));
}
