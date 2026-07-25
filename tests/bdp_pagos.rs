/* [247A-9] Tests de integración DB para el ledger de pagos parciales BDP.
 * Usa #[sqlx::test(migrations = "./migrations")] — BD temporal, migraciones automáticas.
 * NO contacta al servidor BDP — solo valida operaciones CRUD contra PostgreSQL. */

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;

use glory_backend::repositories::bdp_pago::BdpPagoRepository;
use glory_backend::repositories::venta::{NuevaVenta, VentaRepository};

/* Helper: crea un usuario mínimo para satisfacer FK */
async fn create_test_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let email = format!("bdp-pagos-test-{id}@example.com");
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(&email)
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("create_test_user failed");
    id
}

/* Helper: crea una venta válida */
async fn create_test_venta(pool: &PgPool, user_id: Uuid) -> Uuid {
    let data = NuevaVenta {
        user_id,
        fecha: NaiveDate::from_ymd_opt(2026, 7, 15).unwrap(),
        comensales: Some(2),
        descripcion: "Test venta for bdp_pagos",
        iva_porcentaje: Decimal::from(10),
        turno: "noche",
        canal: "comedor",
        metodo_pago: "efectivo",
        importe_base: Decimal::from_str("100.00").unwrap(),
        importe_iva: Decimal::from_str("10.00").unwrap(),
        reserva_id: None,
        cliente_id: None,
    };
    let venta = VentaRepository::create(pool, &data)
        .await
        .expect("create venta failed");
    venta.id
}

/* ── insertar y listar ───────────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_insertar_y_listar_pago(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    let pago = BdpPagoRepository::insertar(
        &pool,
        venta_id,
        Decimal::from_str("25.00").unwrap(),
        1,
        "key-001",
        Some(12345),
        Some("PMT-001"),
    )
    .await
    .expect("insertar should succeed");

    assert_eq!(pago.venta_id, venta_id);
    assert_eq!(pago.amount, Decimal::from_str("25.00").unwrap());
    assert_eq!(pago.tender_id, 1);
    assert_eq!(pago.idempotency_key, "key-001");
    assert_eq!(pago.bdp_order_id, Some(12345));
    assert_eq!(pago.bdp_payment_id.as_deref(), Some("PMT-001"));
    assert_eq!(pago.resultado, "exito");

    let listado = BdpPagoRepository::listar_por_venta(&pool, venta_id)
        .await
        .expect("listar should succeed");
    assert_eq!(listado.len(), 1);
    assert_eq!(listado[0].id, pago.id);
    assert_eq!(listado[0].amount, Decimal::from_str("25.00").unwrap());
}

/* ── total pagado solo cuenta resultado 'exito' ─────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_total_pagado_solo_exito(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    BdpPagoRepository::insertar(
        &pool,
        venta_id,
        Decimal::from_str("10.00").unwrap(),
        1,
        "key-exito",
        None,
        None,
    )
    .await
    .unwrap();

    let otro = BdpPagoRepository::insertar(
        &pool,
        venta_id,
        Decimal::from_str("20.00").unwrap(),
        2,
        "key-ambiguo",
        None,
        None,
    )
    .await
    .unwrap();

    /* Marcar el segundo como ambiguo */
    BdpPagoRepository::actualizar_resultado(&pool, otro.id, "ambiguo", None, Some("timeout"))
        .await
        .unwrap();

    let total = BdpPagoRepository::total_pagado(&pool, venta_id)
        .await
        .expect("total_pagado should succeed");
    assert_eq!(total, Decimal::from_str("10.00").unwrap());
}

/* ── idempotencia por clave ─────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_obtener_por_idempotency_key(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    let insertado = BdpPagoRepository::insertar(
        &pool,
        venta_id,
        Decimal::from_str("15.00").unwrap(),
        3,
        "unique-key",
        None,
        None,
    )
    .await
    .unwrap();

    let encontrado = BdpPagoRepository::obtener_por_idempotency_key(&pool, "unique-key")
        .await
        .expect("lookup should succeed")
        .expect("pago should exist");

    assert_eq!(encontrado.id, insertado.id);
    assert_eq!(encontrado.amount, Decimal::from_str("15.00").unwrap());

    let no_encontrado = BdpPagoRepository::obtener_por_idempotency_key(&pool, "missing-key")
        .await
        .expect("lookup should succeed");
    assert!(no_encontrado.is_none());
}

/* ── aislamiento entre ventas ───────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_aislamiento_entre_ventas(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_a = create_test_venta(&pool, user_id).await;
    let venta_b = create_test_venta(&pool, user_id).await;

    BdpPagoRepository::insertar(
        &pool,
        venta_a,
        Decimal::from_str("5.00").unwrap(),
        1,
        "key-a",
        None,
        None,
    )
    .await
    .unwrap();

    BdpPagoRepository::insertar(
        &pool,
        venta_b,
        Decimal::from_str("7.00").unwrap(),
        2,
        "key-b",
        None,
        None,
    )
    .await
    .unwrap();

    let total_a = BdpPagoRepository::total_pagado(&pool, venta_a)
        .await
        .unwrap();
    let total_b = BdpPagoRepository::total_pagado(&pool, venta_b)
        .await
        .unwrap();

    assert_eq!(total_a, Decimal::from_str("5.00").unwrap());
    assert_eq!(total_b, Decimal::from_str("7.00").unwrap());
}

/* ── idempotencia: clave duplicada para la misma venta falla ─── */

#[sqlx::test(migrations = "./migrations")]
async fn test_idempotency_key_duplicada_falla(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    BdpPagoRepository::insertar(
        &pool,
        venta_id,
        Decimal::from_str("10.00").unwrap(),
        1,
        "shared-key",
        None,
        None,
    )
    .await
    .unwrap();

    let result = BdpPagoRepository::insertar(
        &pool,
        venta_id,
        Decimal::from_str("15.00").unwrap(),
        2,
        "shared-key",
        None,
        None,
    )
    .await;

    assert!(result.is_err(), "duplicate idempotency_key should fail");
}

/* ── idempotencia: clave duplicada para otra venta falla ─────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_idempotency_key_otra_venta_falla(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_a = create_test_venta(&pool, user_id).await;
    let venta_b = create_test_venta(&pool, user_id).await;

    BdpPagoRepository::insertar(
        &pool,
        venta_a,
        Decimal::from_str("10.00").unwrap(),
        1,
        "cross-venta-key",
        None,
        None,
    )
    .await
    .unwrap();

    let result = BdpPagoRepository::insertar(
        &pool,
        venta_b,
        Decimal::from_str("10.00").unwrap(),
        1,
        "cross-venta-key",
        None,
        None,
    )
    .await;

    assert!(
        result.is_err(),
        "reusing idempotency_key across ventas should fail"
    );
}

/* ── actualizar resultado persiste cambios ──────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_actualizar_resultado(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    let pago = BdpPagoRepository::insertar(
        &pool,
        venta_id,
        Decimal::from_str("30.00").unwrap(),
        1,
        "key-resultado",
        None,
        None,
    )
    .await
    .unwrap();

    let respuesta = serde_json::json!({"ok": true});
    BdpPagoRepository::actualizar_resultado(
        &pool,
        pago.id,
        "ambiguo",
        Some(&respuesta),
        Some("timeout de red"),
    )
    .await
    .expect("actualizar_resultado should succeed");

    let actualizado = BdpPagoRepository::obtener_por_idempotency_key(&pool, "key-resultado")
        .await
        .expect("lookup should succeed")
        .expect("pago should exist");

    assert_eq!(actualizado.resultado, "ambiguo");
    assert_eq!(actualizado.error_mensaje.as_deref(), Some("timeout de red"));
    assert_eq!(actualizado.datos_respuesta, Some(respuesta));
}
