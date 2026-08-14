/* [128A-1/F6] Tests de integración DB de las correcciones de la 2a revisión
 * (bloque 128A-1): delete D5 vs facturada_local/ledger (F6-1), idempotencia de
 * factura local alcanzable y scoped por venta (F6-2/F6-5), filas legacy
 * 'error' que ya no bloquean factura local (F6-3) y numeración por (user_id,
 * año) con MAX sin mezclar años (F6-4). F6-6 (contrato de tender_id) vive en
 * el handler HTTP (`ventas.rs::pago_parcial_local`, guard tender_id > 0 ya
 * existente) y se documenta, no se prueba aquí. No contactan con BDP real —
 * validan repositorios/servicios contra PostgreSQL (BD temporal por test). */

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;

use glory_backend::errors::AppError;
use glory_backend::repositories::venta::{NuevaVenta, VentaRepository};
use glory_backend::services::VentaService;

/* ── Helpers ─────────────────────────────────────────────────────────── */

async fn create_test_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let email = format!("f6c-test-{id}@example.com");
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(&email)
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("crear usuario de prueba");
    id
}

/* Venta de 110.00 total (base 100 + IVA 10). */
async fn create_test_venta(pool: &PgPool, user_id: Uuid) -> Uuid {
    let data = NuevaVenta {
        user_id,
        fecha: NaiveDate::from_ymd_opt(2026, 8, 13).unwrap(),
        comensales: Some(2),
        descripcion: "Venta F6C",
        iva_porcentaje: Decimal::from(10),
        turno: "noche",
        canal: "comedor",
        metodo_pago: "efectivo",
        importe_base: Decimal::from_str("100.00").unwrap(),
        importe_iva: Decimal::from_str("10.00").unwrap(),
        reserva_id: None,
        cliente_id: None,
    };
    VentaRepository::create(pool, &data)
        .await
        .expect("crear venta de prueba")
        .id
}

/* ── F6-1: delete D5 vs facturada_local / ledger de pagos ────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn delete_venta_facturada_local_bloqueada(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    VentaService::facturar_local(&pool, venta_id, user_id, None)
        .await
        .expect("facturar local");

    let err = VentaService::delete(&pool, venta_id, user_id)
        .await
        .expect_err("eliminar facturada local debe fallar");
    assert!(matches!(err, AppError::Conflict(_)), "{err:?}");
}

#[sqlx::test(migrations = "./migrations")]
async fn delete_venta_con_filas_bdp_pagos_bloqueada(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    /* Fila legacy (resultado 'error') en el ledger: aunque no cubra saldo,
     * el DELETE no puede cascadear el ledger de pagos (D5). */
    sqlx::query(
        "INSERT INTO bdp_pagos (venta_id, amount, tender_id, idempotency_key, \
         resultado, error_mensaje) VALUES ($1, 60.00, 1, $2, 'error', 'fallo previo BDP')",
    )
    .bind(venta_id)
    .bind(format!("f6c-delete-ledger-{}", Uuid::new_v4()))
    .execute(&pool)
    .await
    .expect("insertar fila legacy en bdp_pagos");

    let err = VentaService::delete(&pool, venta_id, user_id)
        .await
        .expect_err("eliminar venta con ledger de pagos debe fallar");
    assert!(matches!(err, AppError::Conflict(_)), "{err:?}");
}

/* ── F6-2/F6-5: idempotencia de factura local ────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn factura_local_reintento_misma_clave_es_exito_idempotente(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    let f1 = VentaService::facturar_local(&pool, venta_id, user_id, Some("fact-clave-1"))
        .await
        .expect("primera factura");
    assert!(f1.facturada_local);

    /* Reintento con la misma clave sobre la venta ya facturada: la idempotencia
     * se resuelve ANTES de los guards M9 → Ok idempotente, nunca 409. */
    let f2 = VentaService::facturar_local(&pool, venta_id, user_id, Some("fact-clave-1"))
        .await
        .expect("reenvío idempotente");
    assert!(f2.facturada_local);
    assert_eq!(f1.factura_numero, f2.factura_numero, "misma factura");
}

#[sqlx::test(migrations = "./migrations")]
async fn factura_local_clave_reutilizada_otra_venta_conflicto(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_a = create_test_venta(&pool, user_id).await;
    let venta_b = create_test_venta(&pool, user_id).await;

    VentaService::facturar_local(&pool, venta_a, user_id, Some("fact-cross-1"))
        .await
        .expect("facturar venta A");

    let err = VentaService::facturar_local(&pool, venta_b, user_id, Some("fact-cross-1"))
        .await
        .expect_err("reusar clave en otra venta debe fallar");
    assert!(matches!(err, AppError::Conflict(_)), "{err:?}");

    /* La venta B queda sin facturar: no hubo éxito falso. */
    let venta_b = VentaRepository::find_by_id(&pool, venta_b, user_id)
        .await
        .expect("venta B")
        .expect("venta B existe");
    assert!(!venta_b.facturada_local);
    assert!(venta_b.factura_numero.is_none());
}

/* ── F6-3: filas legacy 'error' no bloquean factura local ────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn factura_local_con_fila_legacy_error_ok(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    sqlx::query(
        "INSERT INTO bdp_pagos (venta_id, amount, tender_id, idempotency_key, \
         resultado, error_mensaje) VALUES ($1, 60.00, 1, $2, 'error', 'fallo previo BDP')",
    )
    .bind(venta_id)
    .bind(format!("f6c-legacy-error-{}", Uuid::new_v4()))
    .execute(&pool)
    .await
    .expect("insertar fila legacy error");

    /* El guard de pagos solo mira 'exito'/'ambiguo': la fila legacy no deja la
     * venta bloqueada para siempre. */
    let venta = VentaService::facturar_local(&pool, venta_id, user_id, None)
        .await
        .expect("facturar con fila legacy error");
    assert!(venta.facturada_local);
}

/* ── F6-4: numeración por (user_id, año) con MAX ─────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn numeracion_por_anio_no_mezcla_numeros_previos(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let anio = chrono::Utc::now().format("%Y");

    /* Factura del año anterior ya existente (simula histórico 2025). */
    let venta_prev = create_test_venta(&pool, user_id).await;
    sqlx::query(
        "UPDATE ventas SET facturada_local = true, factura_numero = 'F-2025-0042' WHERE id = $1",
    )
    .bind(venta_prev)
    .execute(&pool)
    .await
    .expect("simular factura del año anterior");

    let venta_1 = create_test_venta(&pool, user_id).await;
    let f1 = VentaService::facturar_local(&pool, venta_1, user_id, None)
        .await
        .expect("facturar venta 1");
    assert_eq!(
        f1.factura_numero.as_deref(),
        Some(format!("F-{anio}-0001").as_str()),
        "el histórico de otro año no debe sumar al contador"
    );

    let venta_2 = create_test_venta(&pool, user_id).await;
    let f2 = VentaService::facturar_local(&pool, venta_2, user_id, None)
        .await
        .expect("facturar venta 2");
    assert_eq!(
        f2.factura_numero.as_deref(),
        Some(format!("F-{anio}-0002").as_str())
    );
}
