// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* Tests de handlers de albaranes de compra BDP (flags, modo BDP y conciliación). */

mod common;
use common::*;

use axum::extract::{Path, State};
use axum::Json;
use glory_backend::errors::AppError;
use glory_backend::handlers::{conciliar_purchase_note, marcar_borrador_purchase_note};
use glory_backend::models::{
    BdpPurchaseNoteDraftRequest, BdpPurchaseNoteEstado, BdpPurchaseNoteReconcileRequest,
};
use glory_backend::repositories::{
    BdpPurchaseNoteRepository, ConfiguracionRepository, GastoRepository,
};
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::str::FromStr;

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
