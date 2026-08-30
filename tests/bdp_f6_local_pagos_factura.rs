// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [128A-1/F6] Tests de integración DB de la fase 6 de independencia BDP:
 * auditoría local (`origen_operacion`), pagos parciales locales (A8/M13) y
 * factura local mínima (A7/D9). No contactan con BDP real — validan los
 * repositorios/servicios contra PostgreSQL (BD temporal por test). */

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;

use glory_backend::errors::AppError;
use glory_backend::models::AnularVentaRequest;
use glory_backend::repositories::bdp_pago::BdpPagoRepository;
use glory_backend::repositories::venta::{NuevaVenta, VentaRepository};
use glory_backend::services::{BdpBackupService, VentaService};

/* ── Helpers ─────────────────────────────────────────────────────────── */

async fn create_test_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let email = format!("f6-test-{id}@example.com");
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
        descripcion: "Venta F6",
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

/* ── Auditoría: origen_operacion ─────────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn listar_audit_incluye_origen_local(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    VentaService::pago_parcial_local(
        &pool,
        user_id,
        venta_id,
        Decimal::from_str("25.00").unwrap(),
        1,
        Some("audit-key-001"),
    )
    .await
    .expect("pago local");

    let entradas = BdpBackupService::listar_audit(&pool, user_id, 10)
        .await
        .expect("listar_audit");
    let pago = entradas
        .iter()
        .find(|e| e.operacion == "pago_parcial_local")
        .expect("entrada de pago local");
    assert_eq!(pago.origen_operacion, "local");
    assert_eq!(pago.target_entity_id, Some(venta_id));

    /* Las entradas legacy/BDP siguen leyéndose con default 'bdp'. */
    let audit_id: Uuid = sqlx::query_scalar(
        "INSERT INTO bdp_audit_log \
         (user_id, operacion, direccion, datos_enviados, resultado, \
          target_entity_type, target_entity_id, authorization_reason) \
         VALUES ($1, 'create_order', 'glory_to_bdp', '{}'::jsonb, 'exito', 'venta', $2, 'test') \
         RETURNING id",
    )
    .bind(user_id)
    .bind(venta_id)
    .fetch_one(&pool)
    .await
    .expect("insert audit legacy");
    let _ = audit_id;

    let entradas = BdpBackupService::listar_audit(&pool, user_id, 10)
        .await
        .expect("listar_audit");
    let bdp = entradas
        .iter()
        .find(|e| e.operacion == "create_order")
        .expect("entrada BDP");
    assert_eq!(bdp.origen_operacion, "bdp");
}

/* ── Pagos parciales locales (A8/M13) ────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn pago_en_dos_partes_saldo_correcto(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    VentaService::pago_parcial_local(
        &pool,
        user_id,
        venta_id,
        Decimal::from_str("60.00").unwrap(),
        1,
        Some("parte-1"),
    )
    .await
    .expect("primera parte");
    VentaService::pago_parcial_local(
        &pool,
        user_id,
        venta_id,
        Decimal::from_str("50.00").unwrap(),
        1,
        Some("parte-2"),
    )
    .await
    .expect("segunda parte");

    let total = BdpPagoRepository::total_pagado(&pool, venta_id)
        .await
        .expect("total_pagado");
    assert_eq!(total, Decimal::from_str("110.00").unwrap());

    let listado = BdpPagoRepository::listar_por_venta(&pool, venta_id)
        .await
        .expect("listar_por_venta");
    assert_eq!(listado.len(), 2);
    assert!(listado.iter().all(|p| p.bdp_order_id.is_none()));
}

#[sqlx::test(migrations = "./migrations")]
async fn pago_sin_clave_no_colisiona(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    /* Dos pagos de la misma venta sin idempotency_key: cada uno es un pago
     * independiente (la clave vacía no puede colapsarlos en uno solo). */
    VentaService::pago_parcial_local(
        &pool,
        user_id,
        venta_id,
        Decimal::from_str("30.00").unwrap(),
        1,
        None,
    )
    .await
    .expect("pago 1 sin clave");
    VentaService::pago_parcial_local(
        &pool,
        user_id,
        venta_id,
        Decimal::from_str("40.00").unwrap(),
        1,
        None,
    )
    .await
    .expect("pago 2 sin clave");

    let total = BdpPagoRepository::total_pagado(&pool, venta_id)
        .await
        .expect("total_pagado");
    assert_eq!(total, Decimal::from_str("70.00").unwrap());
    let listado = BdpPagoRepository::listar_por_venta(&pool, venta_id)
        .await
        .expect("listar_por_venta");
    assert_eq!(listado.len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn idempotencia_misma_clave_mismo_pago(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    let (pago1, audit1) = VentaService::pago_parcial_local(
        &pool,
        user_id,
        venta_id,
        Decimal::from_str("25.00").unwrap(),
        1,
        Some("dup-key"),
    )
    .await
    .expect("primer envío");
    assert!(audit1.is_some());

    let (pago2, audit2) = VentaService::pago_parcial_local(
        &pool,
        user_id,
        venta_id,
        Decimal::from_str("25.00").unwrap(),
        1,
        Some("dup-key"),
    )
    .await
    .expect("reenvío idempotente");

    assert_eq!(pago1.id, pago2.id, "misma fila en el ledger");
    assert!(audit2.is_none(), "segundo envío no audita de nuevo");

    let listado = BdpPagoRepository::listar_por_venta(&pool, venta_id)
        .await
        .expect("listar_por_venta");
    assert_eq!(listado.len(), 1, "sin duplicados");
}

#[sqlx::test(migrations = "./migrations")]
async fn idempotencia_clave_otra_venta_conflicto(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_a = create_test_venta(&pool, user_id).await;
    let venta_b = create_test_venta(&pool, user_id).await;

    VentaService::pago_parcial_local(
        &pool,
        user_id,
        venta_a,
        Decimal::from_str("10.00").unwrap(),
        1,
        Some("cross-key"),
    )
    .await
    .expect("pago en venta A");

    let err = VentaService::pago_parcial_local(
        &pool,
        user_id,
        venta_b,
        Decimal::from_str("10.00").unwrap(),
        1,
        Some("cross-key"),
    )
    .await
    .expect_err("reusar clave en otra venta debe fallar");
    assert!(matches!(err, AppError::Conflict(_)));
}

#[sqlx::test(migrations = "./migrations")]
async fn pago_excede_pendiente_error(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    VentaService::pago_parcial_local(
        &pool,
        user_id,
        venta_id,
        Decimal::from_str("100.00").unwrap(),
        1,
        Some("parcial-1"),
    )
    .await
    .expect("pago parcial");

    let err = VentaService::pago_parcial_local(
        &pool,
        user_id,
        venta_id,
        Decimal::from_str("20.00").unwrap(),
        1,
        Some("parcial-2"),
    )
    .await
    .expect_err("exceder pendiente debe fallar");
    assert!(matches!(err, AppError::Validation(_)));
}

/* ── Factura local mínima (A7/D9) ────────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn factura_local_secuencial_por_usuario(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_1 = create_test_venta(&pool, user_id).await;
    let venta_2 = create_test_venta(&pool, user_id).await;
    let anio = chrono::Utc::now().format("%Y");

    let f1 = VentaService::facturar_local(&pool, venta_1, user_id, None)
        .await
        .expect("facturar venta 1");
    assert!(f1.facturada_local);
    assert_eq!(
        f1.factura_numero.as_deref(),
        Some(format!("F-{anio}-0001").as_str())
    );
    assert!(f1.factura_fecha.is_some());

    let f2 = VentaService::facturar_local(&pool, venta_2, user_id, None)
        .await
        .expect("facturar venta 2");
    assert!(f2.facturada_local);
    assert_eq!(
        f2.factura_numero.as_deref(),
        Some(format!("F-{anio}-0002").as_str())
    );

    /* La numeración es por usuario: otro usuario empieza en 0001. */
    let otro_user = create_test_user(&pool).await;
    let venta_otro = create_test_venta(&pool, otro_user).await;
    let f3 = VentaService::facturar_local(&pool, venta_otro, otro_user, None)
        .await
        .expect("facturar venta de otro usuario");
    assert_eq!(
        f3.factura_numero.as_deref(),
        Some(format!("F-{anio}-0001").as_str())
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn factura_local_doble_bloqueada(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    VentaService::facturar_local(&pool, venta_id, user_id, Some("fact-1"))
        .await
        .expect("primera factura");

    let err = VentaService::facturar_local(&pool, venta_id, user_id, Some("fact-2"))
        .await
        .expect_err("doble facturación debe fallar");
    assert!(matches!(err, AppError::Conflict(_)));
}

#[sqlx::test(migrations = "./migrations")]
async fn factura_local_anulada_bloqueada(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    VentaService::anular(
        &pool,
        venta_id,
        user_id,
        AnularVentaRequest {
            motivo: Some("Error de caja".into()),
            idempotency_key: Some("anular-fact".to_string()),
        },
    )
    .await
    .expect("anular venta");

    let err = VentaService::facturar_local(&pool, venta_id, user_id, None)
        .await
        .expect_err("facturar anulada debe fallar");
    assert!(matches!(err, AppError::Conflict(_)));
}

#[sqlx::test(migrations = "./migrations")]
async fn anular_facturada_local_bloqueada_m9(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    VentaService::facturar_local(&pool, venta_id, user_id, None)
        .await
        .expect("facturar venta");

    let err = VentaService::anular(
        &pool,
        venta_id,
        user_id,
        AnularVentaRequest {
            motivo: Some("Intento posterior".into()),
            idempotency_key: Some("anular-facturada".to_string()),
        },
    )
    .await
    .expect_err("anular facturada local debe fallar (M9)");
    assert!(matches!(err, AppError::Conflict(_)));
}

#[sqlx::test(migrations = "./migrations")]
async fn factura_local_con_pagos_pendientes_bloqueada(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    /* Un pago parcial de 60 deja 50 pendientes en el ledger. */
    VentaService::pago_parcial_local(
        &pool,
        user_id,
        venta_id,
        Decimal::from_str("60.00").unwrap(),
        1,
        Some("medio-pago"),
    )
    .await
    .expect("pago parcial");

    let err = VentaService::facturar_local(&pool, venta_id, user_id, None)
        .await
        .expect_err("facturar con pagos pendientes debe fallar");
    assert!(matches!(err, AppError::Validation(_)));

    /* Completar el pago permite facturar. */
    VentaService::pago_parcial_local(
        &pool,
        user_id,
        venta_id,
        Decimal::from_str("50.00").unwrap(),
        1,
        Some("resto-pago"),
    )
    .await
    .expect("completar pago");
    VentaService::facturar_local(&pool, venta_id, user_id, None)
        .await
        .expect("facturar con saldo cubierto");
}
