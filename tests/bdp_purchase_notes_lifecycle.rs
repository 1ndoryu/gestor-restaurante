/* [247A-12] Tests de integración del ciclo de vida de albaranes de compra BDP.
 * No contactan con BDP real; validan el flujo local: pendiente → borrador → conciliado.
 * Usan #[sqlx::test] para crear una BD temporal por test con migraciones aplicadas. */

use axum::extract::{Path, State};
use axum::Json;
use chrono::NaiveDate;
use glory_backend::config::AppConfig;
use glory_backend::errors::AppError;
use glory_backend::handlers::{conciliar_purchase_note, marcar_borrador_purchase_note};
use glory_backend::middleware::AuthUser;
use glory_backend::models::{
    BdpPurchaseNoteDraftRequest, BdpPurchaseNoteEstado, BdpPurchaseNoteListParams,
    BdpPurchaseNoteReconcileRequest, NotificacionEvent, UserRole,
};
use glory_backend::repositories::gasto::NuevoGasto;
use glory_backend::repositories::{
    BdpPurchaseNoteRepository, ConfiguracionRepository, GastoRepository,
};
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

/// Crea configuración por defecto y activa los feature flags de compras BDP.
async fn create_config_with_purchase_note_flags(pool: &PgPool, user_id: Uuid) {
    ConfiguracionRepository::obtener_o_crear(pool, user_id)
        .await
        .expect("obtener_o_crear configuración");

    sqlx::query(
        "UPDATE configuracion_restaurante \
         SET ff_bdp_purchase_notes_read = TRUE, \
             ff_bdp_purchase_notes_draft = TRUE, \
             ff_bdp_purchase_notes_receive = TRUE \
         WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(pool)
    .await
    .expect("activar flags de compras BDP");
}

fn sample_purchase_note_data(
    serie: &str,
    numero: &str,
    total: Option<Decimal>,
) -> BdpPurchaseNoteData {
    BdpPurchaseNoteData {
        serie_albaran: Some(serie.to_string()),
        num_albaran: Some(numero.to_string()),
        fecha_albaran: Some("2024-07-20".to_string()),
        cod_proveedor: Some(serde_json::json!("PROV-001")),
        nom_proveedor: Some("Proveedor Test".to_string()),
        total_albaran: total,
        extra: serde_json::json!({}),
    }
}

async fn create_test_gasto(
    pool: &PgPool,
    user_id: Uuid,
    importe: Decimal,
) -> glory_backend::models::Gasto {
    let nuevo = NuevoGasto {
        user_id,
        fecha: NaiveDate::from_ymd_opt(2024, 7, 20).unwrap(),
        proveedor: "Proveedor Test",
        categoria_id: None,
        tipo_documento: "albaran",
        metodo_pago: "",
        numero_documento: "A-100",
        recurrente: false,
        importe_base: importe,
        importe_iva: Decimal::ZERO,
    };
    GastoRepository::create(pool, &nuevo)
        .await
        .expect("crear gasto de prueba")
}

fn default_filters() -> BdpPurchaseNoteListParams {
    BdpPurchaseNoteListParams {
        proveedor: None,
        fecha_desde: None,
        fecha_hasta: None,
    }
}

/* ── Tests ───────────────────────────────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn ciclo_pendiente_borrador_conciliado(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_config_with_purchase_note_flags(&pool, user_id).await;

    BdpPurchaseNoteRepository::upsert_from_bdp(
        &pool,
        user_id,
        &sample_purchase_note_data("A", "100", Decimal::from_str("125.50").ok()),
    )
    .await
    .expect("upsert inicial");

    let notes = BdpPurchaseNoteRepository::listar(&pool, user_id, &default_filters())
        .await
        .expect("listar albaranes");
    assert_eq!(notes.len(), 1);
    let note = &notes[0];
    assert!(matches!(note.estado, BdpPurchaseNoteEstado::Pendiente));
    assert!(note.gasto_id.is_none());

    let ok = BdpPurchaseNoteRepository::marcar_borrador(&pool, note.id, user_id)
        .await
        .expect("marcar borrador");
    assert!(ok);

    let note_after_draft = BdpPurchaseNoteRepository::find_by_id(&pool, note.id, user_id)
        .await
        .expect("find después de borrador")
        .expect("albarán existe");
    assert!(matches!(
        note_after_draft.estado,
        BdpPurchaseNoteEstado::Borrador
    ));

    let gasto = create_test_gasto(&pool, user_id, Decimal::from_str("125.50").unwrap()).await;
    let ok = BdpPurchaseNoteRepository::vincular_gasto(&pool, note.id, user_id, gasto.id)
        .await
        .expect("vincular gasto");
    assert!(ok);

    let note_after_reconcile = BdpPurchaseNoteRepository::find_by_id(&pool, note.id, user_id)
        .await
        .expect("find después de conciliar")
        .expect("albarán existe");
    assert!(matches!(
        note_after_reconcile.estado,
        BdpPurchaseNoteEstado::Conciliado
    ));
    assert_eq!(note_after_reconcile.gasto_id, Some(gasto.id));
}

#[sqlx::test(migrations = "./migrations")]
async fn reimportar_no_pierde_estado_ni_gasto_vinculado(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_config_with_purchase_note_flags(&pool, user_id).await;

    BdpPurchaseNoteRepository::upsert_from_bdp(
        &pool,
        user_id,
        &sample_purchase_note_data("A", "200", Decimal::from_str("80.00").ok()),
    )
    .await
    .unwrap();

    let notes = BdpPurchaseNoteRepository::listar(&pool, user_id, &default_filters())
        .await
        .unwrap();
    let note_id = notes[0].id;
    BdpPurchaseNoteRepository::marcar_borrador(&pool, note_id, user_id)
        .await
        .unwrap();
    let gasto = create_test_gasto(&pool, user_id, Decimal::from_str("80.00").unwrap()).await;
    BdpPurchaseNoteRepository::vincular_gasto(&pool, note_id, user_id, gasto.id)
        .await
        .unwrap();

    /* Re-importar con un total distinto — el upsert NO debe tocar estado/gasto_id */
    BdpPurchaseNoteRepository::upsert_from_bdp(
        &pool,
        user_id,
        &sample_purchase_note_data("A", "200", Decimal::from_str("90.00").ok()),
    )
    .await
    .unwrap();

    let note = BdpPurchaseNoteRepository::find_by_id(&pool, note_id, user_id)
        .await
        .unwrap()
        .expect("albarán sigue existiendo");
    assert!(matches!(note.estado, BdpPurchaseNoteEstado::Conciliado));
    assert_eq!(note.gasto_id, Some(gasto.id));
    assert_eq!(note.total, Decimal::from_str("90.00").ok());
}

#[sqlx::test(migrations = "./migrations")]
async fn guardas_de_transicion_rechazan_cambios_invalidos(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_config_with_purchase_note_flags(&pool, user_id).await;

    BdpPurchaseNoteRepository::upsert_from_bdp(
        &pool,
        user_id,
        &sample_purchase_note_data("A", "300", Decimal::from_str("50.00").ok()),
    )
    .await
    .unwrap();

    let note_id = BdpPurchaseNoteRepository::listar(&pool, user_id, &default_filters())
        .await
        .unwrap()[0]
        .id;

    /* No se puede conciliar un albarán pendiente */
    let fake_gasto = Uuid::new_v4();
    let ok = BdpPurchaseNoteRepository::vincular_gasto(&pool, note_id, user_id, fake_gasto)
        .await
        .unwrap();
    assert!(
        !ok,
        "vincular_gasto debe fallar si el estado no es borrador"
    );

    /* Primer borrador ok */
    assert!(
        BdpPurchaseNoteRepository::marcar_borrador(&pool, note_id, user_id)
            .await
            .unwrap()
    );

    /* Segundo borrador sobre el mismo albarán debe fallar */
    let ok = BdpPurchaseNoteRepository::marcar_borrador(&pool, note_id, user_id)
        .await
        .unwrap();
    assert!(!ok, "no se puede marcar borrador dos veces");

    let gasto = create_test_gasto(&pool, user_id, Decimal::from_str("50.00").unwrap()).await;
    assert!(
        BdpPurchaseNoteRepository::vincular_gasto(&pool, note_id, user_id, gasto.id)
            .await
            .unwrap()
    );

    /* Ya conciliado: no se puede volver a borrador ni conciliar */
    let ok = BdpPurchaseNoteRepository::marcar_borrador(&pool, note_id, user_id)
        .await
        .unwrap();
    assert!(!ok, "no se puede volver a borrador tras conciliar");
    let ok = BdpPurchaseNoteRepository::vincular_gasto(&pool, note_id, user_id, gasto.id)
        .await
        .unwrap();
    assert!(!ok, "no se puede conciliar dos veces");
}

#[sqlx::test(migrations = "./migrations")]
async fn aislamiento_por_usuario(pool: PgPool) {
    let user_a = create_test_user(&pool).await;
    let user_b = create_test_user(&pool).await;
    create_config_with_purchase_note_flags(&pool, user_a).await;
    create_config_with_purchase_note_flags(&pool, user_b).await;

    BdpPurchaseNoteRepository::upsert_from_bdp(
        &pool,
        user_a,
        &sample_purchase_note_data("A", "400", Decimal::from_str("30.00").ok()),
    )
    .await
    .unwrap();

    let note_a = BdpPurchaseNoteRepository::listar(&pool, user_a, &default_filters())
        .await
        .unwrap()[0]
        .id;

    /* Usuario B no puede ver ni modificar el albarán de A */
    assert!(BdpPurchaseNoteRepository::find_by_id(&pool, note_a, user_b)
        .await
        .unwrap()
        .is_none());
    assert!(
        !BdpPurchaseNoteRepository::marcar_borrador(&pool, note_a, user_b)
            .await
            .unwrap()
    );

    /* A sí puede trabajar con su albarán */
    assert!(
        BdpPurchaseNoteRepository::marcar_borrador(&pool, note_a, user_a)
            .await
            .unwrap()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn conciliacion_dentro_de_transaccion_es_atomica(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_config_with_purchase_note_flags(&pool, user_id).await;

    BdpPurchaseNoteRepository::upsert_from_bdp(
        &pool,
        user_id,
        &sample_purchase_note_data("A", "500", Decimal::from_str("99.99").ok()),
    )
    .await
    .unwrap();
    let note_id = BdpPurchaseNoteRepository::listar(&pool, user_id, &default_filters())
        .await
        .unwrap()[0]
        .id;
    BdpPurchaseNoteRepository::marcar_borrador(&pool, note_id, user_id)
        .await
        .unwrap();

    /* Ejecutar creación de gasto + vinculación en una sola transacción */
    let mut tx = pool.begin().await.expect("begin transaction");

    let nuevo = NuevoGasto {
        user_id,
        fecha: NaiveDate::from_ymd_opt(2024, 7, 20).unwrap(),
        proveedor: "Proveedor TX",
        categoria_id: None,
        tipo_documento: "albaran",
        metodo_pago: "",
        numero_documento: "A-500",
        recurrente: false,
        importe_base: Decimal::from_str("99.99").unwrap(),
        importe_iva: Decimal::ZERO,
    };
    let gasto = GastoRepository::create(&mut *tx, &nuevo)
        .await
        .expect("crear gasto en tx");
    BdpPurchaseNoteRepository::vincular_gasto(&mut *tx, note_id, user_id, gasto.id)
        .await
        .expect("vincular en tx");

    tx.commit().await.expect("commit");

    let note = BdpPurchaseNoteRepository::find_by_id(&pool, note_id, user_id)
        .await
        .unwrap()
        .expect("albarán conciliado");
    assert!(matches!(note.estado, BdpPurchaseNoteEstado::Conciliado));
    assert_eq!(note.gasto_id, Some(gasto.id));
}

/* ── Helpers de handlers ─────────────────────────────────────────────── */

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

/* ── Tests de handlers ───────────────────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn handler_borrador_rechaza_flag_desactivado(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    /* Configuración por defecto: todos los flags de compras BDP desactivados */
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración por defecto");

    BdpPurchaseNoteRepository::upsert_from_bdp(
        &pool,
        user_id,
        &sample_purchase_note_data("A", "600", Decimal::from_str("10.00").ok()),
    )
    .await
    .unwrap();
    let note_id = BdpPurchaseNoteRepository::listar(&pool, user_id, &default_filters())
        .await
        .unwrap()[0]
        .id;

    let state = make_app_state(pool);
    let auth = make_auth(user_id);
    let result = marcar_borrador_purchase_note(
        State(state),
        auth,
        Path(note_id),
        Json(BdpPurchaseNoteDraftRequest {}),
    )
    .await;

    assert!(
        result.is_err(),
        "debe fallar cuando el feature flag está desactivado"
    );
    match result.unwrap_err() {
        AppError::Validation(msg) => assert!(msg.contains("borradores de compra BDP")),
        other => panic!("se esperaba AppError::Validation, se obtuvo {other:?}"),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn handler_conciliar_rechaza_flag_desactivado(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración por defecto");

    BdpPurchaseNoteRepository::upsert_from_bdp(
        &pool,
        user_id,
        &sample_purchase_note_data("A", "700", Decimal::from_str("20.00").ok()),
    )
    .await
    .unwrap();
    let note_id = BdpPurchaseNoteRepository::listar(&pool, user_id, &default_filters())
        .await
        .unwrap()[0]
        .id;

    let state = make_app_state(pool);
    let auth = make_auth(user_id);
    let req = BdpPurchaseNoteReconcileRequest {
        gasto_existente_id: None,
        categoria_id: None,
    };
    let result = conciliar_purchase_note(State(state), auth, Path(note_id), Json(req)).await;

    assert!(
        result.is_err(),
        "debe fallar cuando el feature flag está desactivado"
    );
    match result.unwrap_err() {
        AppError::Validation(msg) => assert!(msg.contains("conciliación de compras BDP")),
        other => panic!("se esperaba AppError::Validation, se obtuvo {other:?}"),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn handler_conciliar_crea_gasto_nuevo_cuando_no_se_indica_existente(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_config_with_purchase_note_flags(&pool, user_id).await;

    BdpPurchaseNoteRepository::upsert_from_bdp(
        &pool,
        user_id,
        &sample_purchase_note_data("A", "800", Decimal::from_str("35.00").ok()),
    )
    .await
    .unwrap();
    let note_id = BdpPurchaseNoteRepository::listar(&pool, user_id, &default_filters())
        .await
        .unwrap()[0]
        .id;

    /* El handler requiere estado borrador para conciliar */
    BdpPurchaseNoteRepository::marcar_borrador(&pool, note_id, user_id)
        .await
        .unwrap();

    let state = make_app_state(pool.clone());
    let auth = make_auth(user_id);
    let req = BdpPurchaseNoteReconcileRequest {
        gasto_existente_id: None,
        categoria_id: None,
    };
    let result = conciliar_purchase_note(State(state), auth, Path(note_id), Json(req)).await;

    assert!(
        result.is_ok(),
        "conciliar debe crear un gasto nuevo y vincularlo"
    );
    let Json(result) = result.unwrap();

    let note = BdpPurchaseNoteRepository::find_by_id(&pool, result.albaran_id, user_id)
        .await
        .unwrap()
        .expect("albarán conciliado");
    assert!(matches!(note.estado, BdpPurchaseNoteEstado::Conciliado));
    assert_eq!(note.gasto_id, Some(result.gasto_id));

    let gasto = GastoRepository::find_by_id(&pool, result.gasto_id, user_id)
        .await
        .unwrap()
        .expect("gasto creado");
    assert_eq!(gasto.importe_base, Decimal::from_str("35.00").unwrap());
    assert_eq!(gasto.proveedor, "Proveedor Test");
}

#[sqlx::test(migrations = "./migrations")]
async fn handler_borrador_happy_path_marca_albaran(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_config_with_purchase_note_flags(&pool, user_id).await;

    BdpPurchaseNoteRepository::upsert_from_bdp(
        &pool,
        user_id,
        &sample_purchase_note_data("A", "900", Decimal::from_str("15.00").ok()),
    )
    .await
    .unwrap();
    let note_id = BdpPurchaseNoteRepository::listar(&pool, user_id, &default_filters())
        .await
        .unwrap()[0]
        .id;

    let state = make_app_state(pool.clone());
    let auth = make_auth(user_id);
    let result = marcar_borrador_purchase_note(
        State(state),
        auth,
        Path(note_id),
        Json(BdpPurchaseNoteDraftRequest {}),
    )
    .await;

    assert!(
        result.is_ok(),
        "el handler de borrador debe funcionar con flag activado"
    );
    let Json(note) = result.unwrap();
    assert!(matches!(note.estado, BdpPurchaseNoteEstado::Borrador));
}

#[sqlx::test(migrations = "./migrations")]
async fn handler_conciliar_vincula_gasto_existente(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_config_with_purchase_note_flags(&pool, user_id).await;

    BdpPurchaseNoteRepository::upsert_from_bdp(
        &pool,
        user_id,
        &sample_purchase_note_data("A", "1000", Decimal::from_str("75.00").ok()),
    )
    .await
    .unwrap();
    let note_id = BdpPurchaseNoteRepository::listar(&pool, user_id, &default_filters())
        .await
        .unwrap()[0]
        .id;
    BdpPurchaseNoteRepository::marcar_borrador(&pool, note_id, user_id)
        .await
        .unwrap();

    let gasto_existente =
        create_test_gasto(&pool, user_id, Decimal::from_str("75.00").unwrap()).await;

    let state = make_app_state(pool.clone());
    let auth = make_auth(user_id);
    let req = BdpPurchaseNoteReconcileRequest {
        gasto_existente_id: Some(gasto_existente.id),
        categoria_id: None,
    };
    let result = conciliar_purchase_note(State(state), auth, Path(note_id), Json(req)).await;

    assert!(result.is_ok(), "conciliar debe vincular un gasto existente");
    let Json(result) = result.unwrap();
    assert_eq!(result.gasto_id, gasto_existente.id);
    assert_eq!(result.accion, "vinculado");

    let note = BdpPurchaseNoteRepository::find_by_id(&pool, note_id, user_id)
        .await
        .unwrap()
        .expect("albarán conciliado");
    assert!(matches!(note.estado, BdpPurchaseNoteEstado::Conciliado));
    assert_eq!(note.gasto_id, Some(gasto_existente.id));
}
