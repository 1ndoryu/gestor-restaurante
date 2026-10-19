// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [247A-12] Tests de integración del ciclo de vida de albaranes de compra BDP.
 * No contactan con BDP real; validan el flujo local: pendiente → borrador → conciliado.
 * Usan #[sqlx::test] para crear una BD temporal por test con migraciones aplicadas. */

mod common;
use common::*;

use chrono::NaiveDate;
use glory_backend::models::BdpPurchaseNoteEstado;
use glory_backend::repositories::gasto::NuevoGasto;
use glory_backend::repositories::{BdpPurchaseNoteRepository, GastoRepository};
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;

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
