// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* Tests de albaranes locales (CRUD, M18) y rastro de auditoría del ciclo local. */

mod common;
use common::*;

use axum::extract::{Path, Query, State};
use axum::Json;
use glory_backend::errors::AppError;
use glory_backend::handlers::{
    actualizar_purchase_note_local, conciliar_purchase_note, crear_purchase_note_local,
    eliminar_purchase_note_local, listar_purchase_notes, marcar_borrador_purchase_note,
};
use glory_backend::models::{
    ActualizarBdpPurchaseNoteRequest, BdpPurchaseNoteDraftRequest, BdpPurchaseNoteEstado,
    BdpPurchaseNoteLineaLocal, BdpPurchaseNoteReconcileRequest, CrearBdpPurchaseNoteRequest,
};
use glory_backend::repositories::{BdpPurchaseNoteRepository, ConfiguracionRepository};
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;

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

#[sqlx::test(migrations = "./migrations")]
async fn handler_listar_modo_bdp_flag_off_devuelve_locales(pool: PgPool) {
    /* Los albaranes locales no dependen del BDP: listar no se gatea por
     * `ff_bdp_purchase_notes_read` en ningún modo. Solo `sync` lo exige. */
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración por defecto");
    sqlx::query("UPDATE configuracion_restaurante SET modo_operacion = 'bdp' WHERE user_id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();

    let req = crear_request_local(
        "Proveedor Local",
        Some(Decimal::from_str("10.00").unwrap()),
        None,
    );
    let state = make_app_state(pool.clone());
    let auth = make_auth(user_id);
    let creado = crear_purchase_note_local(State(state), auth, Json(req))
        .await
        .expect("crear local debe funcionar en modo bdp con flag apagado");
    assert_eq!(creado.0.origen, "local");

    let state2 = make_app_state(pool);
    let auth2 = make_auth(user_id);
    let result = listar_purchase_notes(State(state2), auth2, Query(default_filters())).await;
    assert!(
        result.is_ok(),
        "listar debe funcionar en modo bdp con flag apagado"
    );
    let Json(notes) = result.unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].origen, "local");
}

/* ── Tests 169A-1: rastro de auditoría del ciclo local ─────────────────── */

/// Lee el rastro de auditoría de un usuario en orden cronológico:
/// (`operacion`, `origen_operacion`, `target_entity_type`, `target_entity_id`).
async fn audit_rows(
    pool: &PgPool,
    user_id: Uuid,
) -> Vec<(String, String, Option<String>, Option<Uuid>)> {
    sqlx::query_as::<_, (String, String, Option<String>, Option<Uuid>)>(
        "SELECT operacion, origen_operacion, target_entity_type, target_entity_id \
         FROM bdp_audit_log WHERE user_id = $1 ORDER BY created_at",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .expect("leer auditoría")
}

#[sqlx::test(migrations = "./migrations")]
async fn ciclo_local_crear_borrador_conciliar_deja_rastro_auditoria(pool: PgPool) {
    /* [169A-1] Crear → borrador → conciliar (gasto nuevo) deja 4 filas de
     * auditoría local: albaran_local_crear, albaran_local_borrador,
     * albaran_local_conciliar y gasto_local_crear. */
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración por defecto");

    let req = crear_request_local(
        "Proveedor Auditado",
        Some(Decimal::from_str("30.00").unwrap()),
        None,
    );
    let state = make_app_state(pool.clone());
    let Json(note) = crear_purchase_note_local(State(state), make_auth(user_id), Json(req))
        .await
        .expect("crear local");

    let rows = audit_rows(&pool, user_id).await;
    assert_eq!(
        rows,
        vec![(
            "albaran_local_crear".to_string(),
            "local".to_string(),
            Some("albaran".to_string()),
            Some(note.id),
        )],
        "el alta debe auditarse con origen local y target albarán"
    );

    let state = make_app_state(pool.clone());
    let _ = marcar_borrador_purchase_note(
        State(state),
        make_auth(user_id),
        Path(note.id),
        Json(BdpPurchaseNoteDraftRequest {}),
    )
    .await
    .expect("pasar a borrador");

    let rows = audit_rows(&pool, user_id).await;
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[1],
        (
            "albaran_local_borrador".to_string(),
            "local".to_string(),
            Some("albaran".to_string()),
            Some(note.id),
        )
    );

    let state = make_app_state(pool.clone());
    let req = BdpPurchaseNoteReconcileRequest {
        gasto_existente_id: None,
        categoria_id: None,
    };
    let Json(result) =
        conciliar_purchase_note(State(state), make_auth(user_id), Path(note.id), Json(req))
            .await
            .expect("conciliar con gasto nuevo");

    let rows = audit_rows(&pool, user_id).await;
    assert_eq!(rows.len(), 4);
    /* conciliar + gasto se insertan en la misma tx (mismo NOW()): el orden
     * entre ambas filas no está garantizado; se comparan como conjunto. */
    let mut ultimas: Vec<(String, String, Option<String>, Option<Uuid>)> =
        rows.into_iter().skip(2).collect();
    ultimas.sort();
    let mut esperadas = vec![
        (
            "albaran_local_conciliar".to_string(),
            "local".to_string(),
            Some("albaran".to_string()),
            Some(note.id),
        ),
        (
            "gasto_local_crear".to_string(),
            "local".to_string(),
            Some("gasto".to_string()),
            Some(result.gasto_id),
        ),
    ];
    esperadas.sort();
    assert_eq!(ultimas, esperadas);

    /* Reconciliar de nuevo falla (guard) y no añade rastro. */
    let state = make_app_state(pool.clone());
    let req = BdpPurchaseNoteReconcileRequest {
        gasto_existente_id: None,
        categoria_id: None,
    };
    let result =
        conciliar_purchase_note(State(state), make_auth(user_id), Path(note.id), Json(req)).await;
    assert!(result.is_err(), "conciliar un conciliado debe fallar");
    assert_eq!(audit_rows(&pool, user_id).await.len(), 4);
}

#[sqlx::test(migrations = "./migrations")]
async fn conciliar_gasto_existente_no_fabrica_auditoria_gasto(pool: PgPool) {
    /* [169A-1] Al vincular un gasto preexistente solo se audita la
     * conciliación: el alta del gasto es anterior al rastro y no se fabrica. */
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración por defecto");

    let req = crear_request_local(
        "Proveedor Auditado",
        Some(Decimal::from_str("20.00").unwrap()),
        None,
    );
    let state = make_app_state(pool.clone());
    let Json(note) = crear_purchase_note_local(State(state), make_auth(user_id), Json(req))
        .await
        .expect("crear local");

    let state = make_app_state(pool.clone());
    let _ = marcar_borrador_purchase_note(
        State(state),
        make_auth(user_id),
        Path(note.id),
        Json(BdpPurchaseNoteDraftRequest {}),
    )
    .await
    .expect("pasar a borrador");

    let gasto = create_test_gasto(&pool, user_id, Decimal::from_str("20.00").unwrap()).await;

    let state = make_app_state(pool.clone());
    let req = BdpPurchaseNoteReconcileRequest {
        gasto_existente_id: Some(gasto.id),
        categoria_id: None,
    };
    let Json(result) =
        conciliar_purchase_note(State(state), make_auth(user_id), Path(note.id), Json(req))
            .await
            .expect("conciliar con gasto existente");
    assert_eq!(result.gasto_id, gasto.id);

    let rows = audit_rows(&pool, user_id).await;
    assert_eq!(
        rows.len(),
        3,
        "crear + borrador + conciliar, sin gasto_local_crear"
    );
    assert!(rows.iter().all(|r| r.0 != "gasto_local_crear"));
    assert_eq!(rows[2].0, "albaran_local_conciliar");
}
