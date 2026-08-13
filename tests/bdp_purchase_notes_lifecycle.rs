/* [247A-12] Tests de integración del ciclo de vida de albaranes de compra BDP.
 * No contactan con BDP real; validan el flujo local: pendiente → borrador → conciliado.
 * Usan #[sqlx::test] para crear una BD temporal por test con migraciones aplicadas. */

use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::NaiveDate;
use glory_backend::config::AppConfig;
use glory_backend::errors::AppError;
use glory_backend::handlers::{
    actualizar_purchase_note_local, conciliar_purchase_note, crear_purchase_note_local,
    eliminar_purchase_note_local, listar_purchase_notes, marcar_borrador_purchase_note,
};
use glory_backend::middleware::AuthUser;
use glory_backend::models::{
    ActualizarBdpPurchaseNoteRequest, BdpPurchaseNoteDraftRequest, BdpPurchaseNoteEstado,
    BdpPurchaseNoteLineaLocal, BdpPurchaseNoteListParams, BdpPurchaseNoteReconcileRequest,
    CrearBdpPurchaseNoteRequest, NotificacionEvent, UserRole,
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

/* ── Tests de handlers ───────────────────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn handler_borrador_sin_flags_funciona_en_standalone(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    /* [128A-1/F5][M12] Configuración por defecto (modo 'auto' sin BDP
     * configurado) = modo efectivo standalone: el ciclo de vida local NO
     * consulta los feature flags BDP. */
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
        result.is_ok(),
        "en modo standalone el borrador local no debe depender del flag"
    );
    let Json(note) = result.unwrap();
    assert!(matches!(note.estado, BdpPurchaseNoteEstado::Borrador));
}

#[sqlx::test(migrations = "./migrations")]
async fn handler_borrador_flag_off_bloquea_en_modo_bdp(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    /* [128A-1/F5][M12] En modo efectivo bdp los flags sí gatean. */
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración por defecto");
    sqlx::query("UPDATE configuracion_restaurante SET modo_operacion = 'bdp' WHERE user_id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();

    BdpPurchaseNoteRepository::upsert_from_bdp(
        &pool,
        user_id,
        &sample_purchase_note_data("A", "650", Decimal::from_str("10.00").ok()),
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
        "en modo bdp debe fallar cuando el feature flag está desactivado"
    );
    match result.unwrap_err() {
        AppError::Validation(msg) => assert!(msg.contains("borradores de compra BDP")),
        other => panic!("se esperaba AppError::Validation, se obtuvo {other:?}"),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn handler_conciliar_flag_off_bloquea_en_modo_bdp(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración por defecto");
    sqlx::query("UPDATE configuracion_restaurante SET modo_operacion = 'bdp' WHERE user_id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();

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
        "en modo bdp debe fallar cuando el feature flag está desactivado"
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

/* ── Tests F5: CRUD de albaranes locales (M18) ──────────────────────── */

fn crear_request_local(
    nombre_proveedor: &str,
    total: Option<Decimal>,
    lineas: Option<Vec<BdpPurchaseNoteLineaLocal>>,
) -> CrearBdpPurchaseNoteRequest {
    CrearBdpPurchaseNoteRequest {
        serie: None,
        numero: None,
        fecha: Some("2026-08-01".to_string()),
        codigo_proveedor: None,
        nombre_proveedor: Some(nombre_proveedor.to_string()),
        total,
        lineas,
    }
}

fn linea_local(
    descripcion: &str,
    cantidad: &str,
    precio: &str,
    iva: &str,
) -> BdpPurchaseNoteLineaLocal {
    BdpPurchaseNoteLineaLocal {
        descripcion: descripcion.to_string(),
        cantidad: Decimal::from_str(cantidad).unwrap(),
        precio_unitario: Decimal::from_str(precio).unwrap(),
        iva_pct: Decimal::from_str(iva).unwrap(),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn crear_local_asigna_serie_l_y_secuencial(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración por defecto");

    let req = crear_request_local(
        "Proveedor Local",
        Some(Decimal::from_str("100.00").unwrap()),
        None,
    );
    let primera = BdpPurchaseNoteRepository::crear_local(&pool, user_id, &req)
        .await
        .expect("crear primer albarán local");
    assert_eq!(primera.serie, "L");
    assert_eq!(primera.numero, "1");
    assert_eq!(primera.origen, "local");
    assert!(matches!(primera.estado, BdpPurchaseNoteEstado::Pendiente));
    assert_eq!(primera.total, Decimal::from_str("100.00").ok());

    let segunda = BdpPurchaseNoteRepository::crear_local(&pool, user_id, &req)
        .await
        .expect("crear segundo albarán local");
    assert_eq!(segunda.serie, "L");
    assert_eq!(segunda.numero, "2");

    /* El secuencial es por usuario: otro usuario empieza en 1. */
    let user_b = create_test_user(&pool).await;
    let del_otro = BdpPurchaseNoteRepository::crear_local(&pool, user_b, &req)
        .await
        .expect("crear albarán local de otro usuario");
    assert_eq!(del_otro.numero, "1");
}

#[sqlx::test(migrations = "./migrations")]
async fn crear_local_calcula_total_y_desglose_por_linea(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración por defecto");

    let req = crear_request_local(
        "Proveedor Local",
        None,
        Some(vec![
            linea_local("Tomate", "2", "10.00", "10"),
            linea_local("Pan", "3", "2.00", "21"),
        ]),
    );
    let note = BdpPurchaseNoteRepository::crear_local(&pool, user_id, &req)
        .await
        .expect("crear albarán local con líneas");
    /* base = 20 + 6 = 26; iva = 2 + 1.26 = 3.26; total = 29.26 */
    assert_eq!(note.total, Decimal::from_str("29.26").ok());
    assert_eq!(
        note.datos_bdp["importe_base"],
        serde_json::json!(Decimal::from_str("26.00").unwrap())
    );
    assert_eq!(
        note.datos_bdp["importe_iva"],
        serde_json::json!(Decimal::from_str("3.26").unwrap())
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn actualizar_local_edita_solo_origen_local(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración por defecto");

    let req = crear_request_local(
        "Proveedor Local",
        Some(Decimal::from_str("100.00").unwrap()),
        None,
    );
    let local = BdpPurchaseNoteRepository::crear_local(&pool, user_id, &req)
        .await
        .unwrap();

    let update = ActualizarBdpPurchaseNoteRequest {
        numero: None,
        fecha: Some("2026-08-02".to_string()),
        codigo_proveedor: Some("LOC-1".to_string()),
        nombre_proveedor: Some("Proveedor Editado".to_string()),
        total: Some(Decimal::from_str("120.00").unwrap()),
        lineas: None,
    };
    let ok = BdpPurchaseNoteRepository::actualizar_local(&pool, local.id, user_id, &update)
        .await
        .unwrap();
    assert!(ok, "actualizar albarán local debe funcionar");
    let updated = BdpPurchaseNoteRepository::find_by_id(&pool, local.id, user_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        updated.nombre_proveedor.as_deref(),
        Some("Proveedor Editado")
    );
    assert_eq!(updated.total, Decimal::from_str("120.00").ok());

    /* Un albarán importado de BDP no se puede editar (solo local). */
    BdpPurchaseNoteRepository::upsert_from_bdp(
        &pool,
        user_id,
        &sample_purchase_note_data("A", "1100", Decimal::from_str("5.00").ok()),
    )
    .await
    .unwrap();
    let bdp_note_id = BdpPurchaseNoteRepository::listar(&pool, user_id, &default_filters())
        .await
        .unwrap()
        .iter()
        .find(|n| n.serie == "A")
        .unwrap()
        .id;
    let ok = BdpPurchaseNoteRepository::actualizar_local(&pool, bdp_note_id, user_id, &update)
        .await
        .unwrap();
    assert!(!ok, "no se puede actualizar un albarán de origen bdp");
}

#[sqlx::test(migrations = "./migrations")]
async fn eliminar_local_solo_pendiente_o_borrador(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración por defecto");

    let req = crear_request_local(
        "Proveedor Local",
        Some(Decimal::from_str("50.00").unwrap()),
        None,
    );
    let pendiente = BdpPurchaseNoteRepository::crear_local(&pool, user_id, &req)
        .await
        .unwrap();
    assert!(
        BdpPurchaseNoteRepository::eliminar_local(&pool, pendiente.id, user_id)
            .await
            .unwrap(),
        "un albarán pendiente se puede eliminar"
    );

    /* Conciliado no se borra (D5). */
    let conciliable = BdpPurchaseNoteRepository::crear_local(&pool, user_id, &req)
        .await
        .unwrap();
    BdpPurchaseNoteRepository::marcar_borrador(&pool, conciliable.id, user_id)
        .await
        .unwrap();
    let gasto = create_test_gasto(&pool, user_id, Decimal::from_str("50.00").unwrap()).await;
    BdpPurchaseNoteRepository::vincular_gasto(&pool, conciliable.id, user_id, gasto.id)
        .await
        .unwrap();
    assert!(
        !BdpPurchaseNoteRepository::eliminar_local(&pool, conciliable.id, user_id)
            .await
            .unwrap(),
        "un albarán conciliado no se puede eliminar"
    );

    /* Un albarán de origen bdp no se puede eliminar. */
    BdpPurchaseNoteRepository::upsert_from_bdp(
        &pool,
        user_id,
        &sample_purchase_note_data("A", "1200", Decimal::from_str("5.00").ok()),
    )
    .await
    .unwrap();
    let bdp_note_id = BdpPurchaseNoteRepository::listar(&pool, user_id, &default_filters())
        .await
        .unwrap()
        .iter()
        .find(|n| n.serie == "A")
        .unwrap()
        .id;
    assert!(
        !BdpPurchaseNoteRepository::eliminar_local(&pool, bdp_note_id, user_id)
            .await
            .unwrap(),
        "un albarán de origen bdp no se puede eliminar"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn handler_crear_local_standalone_sin_flags(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración por defecto");

    let state = make_app_state(pool.clone());
    let auth = make_auth(user_id);
    let req = crear_request_local(
        "Proveedor Local",
        Some(Decimal::from_str("30.00").unwrap()),
        None,
    );
    let result = crear_purchase_note_local(State(state), auth, Json(req)).await;
    assert!(
        result.is_ok(),
        "crear local debe funcionar sin flags en standalone"
    );
    let Json(note) = result.unwrap();
    assert_eq!(note.origen, "local");
    assert_eq!(note.serie, "L");

    /* Sin proveedor ni total → validación. */
    let state2 = make_app_state(pool);
    let auth2 = make_auth(user_id);
    let req_invalido = CrearBdpPurchaseNoteRequest {
        serie: None,
        numero: None,
        fecha: None,
        codigo_proveedor: None,
        nombre_proveedor: None,
        total: None,
        lineas: None,
    };
    let result = crear_purchase_note_local(State(state2), auth2, Json(req_invalido)).await;
    assert!(matches!(result, Err(AppError::Validation(_))));
}

#[sqlx::test(migrations = "./migrations")]
async fn handler_actualizar_eliminar_local_via_handlers(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración por defecto");

    let req = crear_request_local(
        "Proveedor Local",
        Some(Decimal::from_str("40.00").unwrap()),
        None,
    );
    let note = BdpPurchaseNoteRepository::crear_local(&pool, user_id, &req)
        .await
        .unwrap();

    /* PUT edita el albarán local. */
    let state = make_app_state(pool.clone());
    let auth = make_auth(user_id);
    let update = ActualizarBdpPurchaseNoteRequest {
        numero: None,
        fecha: None,
        codigo_proveedor: None,
        nombre_proveedor: Some("Proveedor Editado".to_string()),
        total: None,
        lineas: None,
    };
    let result =
        actualizar_purchase_note_local(State(state), auth, Path(note.id), Json(update.clone()))
            .await;
    assert!(
        result.is_ok(),
        "actualizar local vía handler debe funcionar"
    );
    let Json(updated) = result.unwrap();
    assert_eq!(
        updated.nombre_proveedor.as_deref(),
        Some("Proveedor Editado")
    );

    /* DELETE elimina el albarán local pendiente. */
    let state2 = make_app_state(pool.clone());
    let auth2 = make_auth(user_id);
    let result = eliminar_purchase_note_local(State(state2), auth2, Path(note.id)).await;
    assert!(
        result.is_ok(),
        "eliminar local pendiente vía handler debe funcionar"
    );
    assert!(
        BdpPurchaseNoteRepository::find_by_id(&pool, note.id, user_id)
            .await
            .unwrap()
            .is_none(),
        "el albarán local debe haber desaparecido"
    );

    /* Editar/eliminar un albarán BDP → 400. */
    BdpPurchaseNoteRepository::upsert_from_bdp(
        &pool,
        user_id,
        &sample_purchase_note_data("A", "1300", Decimal::from_str("5.00").ok()),
    )
    .await
    .unwrap();
    let bdp_note_id = BdpPurchaseNoteRepository::listar(&pool, user_id, &default_filters())
        .await
        .unwrap()
        .iter()
        .find(|n| n.serie == "A")
        .unwrap()
        .id;
    let state3 = make_app_state(pool.clone());
    let auth3 = make_auth(user_id);
    let result =
        actualizar_purchase_note_local(State(state3), auth3, Path(bdp_note_id), Json(update)).await;
    assert!(matches!(result, Err(AppError::Validation(_))));

    let state4 = make_app_state(pool);
    let auth4 = make_auth(user_id);
    let result = eliminar_purchase_note_local(State(state4), auth4, Path(bdp_note_id)).await;
    assert!(matches!(result, Err(AppError::Validation(_))));
}

#[sqlx::test(migrations = "./migrations")]
async fn handler_listar_standalone_sin_flags_devuelve_locales_y_bdp(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración por defecto");

    let req = crear_request_local(
        "Proveedor Local",
        Some(Decimal::from_str("10.00").unwrap()),
        None,
    );
    BdpPurchaseNoteRepository::crear_local(&pool, user_id, &req)
        .await
        .unwrap();

    let state = make_app_state(pool);
    let auth = make_auth(user_id);
    let result = listar_purchase_notes(State(state), auth, Query(default_filters())).await;
    assert!(
        result.is_ok(),
        "listar debe funcionar sin flags en standalone"
    );
    let Json(notes) = result.unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].origen, "local");
}
