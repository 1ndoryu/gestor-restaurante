// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [128A-1/F4] Tests de integración DB de la fase 4 de independencia BDP:
 * delete D5 (checks per-venta antes que guard de config BDP), anulación local
 * con usuario siempre derivado de auth (F4-3), `total_periodo` por modalidad
 * (F4-4) e idempotencia de anulación scoped por venta (F4-5). No contactan con
 * BDP real — validan repositorios/servicios contra PostgreSQL (BD temporal). */

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;

use glory_backend::errors::AppError;
use glory_backend::models::AnularVentaRequest;
use glory_backend::repositories::venta::{NuevaVenta, VentaRepository};
use glory_backend::repositories::ConfiguracionRepository;
use glory_backend::services::{DashboardService, VentaService};

/* ── Helpers ─────────────────────────────────────────────────────────── */

async fn create_test_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let email = format!("f4-test-{id}@example.com");
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
        descripcion: "Venta F4",
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

/* ── F4-1: delete D5 (checks per-venta antes que config BDP) ─────────── */

#[sqlx::test(migrations = "./migrations")]
async fn delete_venta_local_sin_sync_con_bdp_activo_ok(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    /* Sync BDP activa en config, pero la venta NO está sincronizada: debe
     * poder eliminarse (D5=A: el bloqueo es per-venta, no global). */
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config");
    sqlx::query(
        "UPDATE configuracion_restaurante SET bdp_sync_enabled = true, \
         haddock_sync_enabled = false WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("activar sync BDP");

    VentaService::delete(&pool, venta_id, user_id)
        .await
        .expect("delete local sin sync debe funcionar");

    let existe = VentaRepository::find_by_id(&pool, venta_id, user_id)
        .await
        .expect("buscar venta");
    assert!(existe.is_none(), "la venta local debe haberse eliminado");
}

#[sqlx::test(migrations = "./migrations")]
async fn delete_venta_sincronizada_bdp_bloqueada(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    sqlx::query("UPDATE ventas SET bdp_synced = true, bdp_order_id = 9001 WHERE id = $1")
        .bind(venta_id)
        .execute(&pool)
        .await
        .expect("marcar venta sincronizada BDP");

    let err = VentaService::delete(&pool, venta_id, user_id)
        .await
        .expect_err("venta sincronizada BDP no debe eliminarse");
    assert!(matches!(err, AppError::Conflict(_)));
}

/* ── F4-3: anulación con usuario siempre derivado de auth ────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn anular_registra_usuario_autenticado(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    let venta = VentaService::anular(
        &pool,
        venta_id,
        user_id,
        AnularVentaRequest {
            motivo: Some("Error de caja".into()),
            idempotency_key: Some("anular-f4-3".to_string()),
        },
    )
    .await
    .expect("anular venta");

    assert!(venta.anulada);
    assert_eq!(
        venta.anulacion_usuario,
        Some(user_id),
        "el usuario que anula debe ser siempre el autenticado (F4-3)"
    );
}

/* ── F4-4: total_periodo excluye anuladas solo en credito_completo ────── */

#[sqlx::test(migrations = "./migrations")]
async fn resumen_mes_respeta_modalidad_anulacion(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    VentaService::anular(
        &pool,
        venta_id,
        user_id,
        AnularVentaRequest {
            motivo: Some("Cliente no conforme".into()),
            idempotency_key: Some("anular-f4-4".to_string()),
        },
    )
    .await
    .expect("anular venta");

    /* credito_completo (default): la anulada revierte IVA y se excluye. */
    let resumen = DashboardService::resumen_mes(&pool, user_id, 2026, 8)
        .await
        .expect("resumen mes credito_completo");
    assert_eq!(
        resumen.total_ventas,
        Decimal::ZERO,
        "credito_completo debe excluir la venta anulada"
    );

    /* estado_solo: solo marca estado, la anulada sigue contando. */
    sqlx::query(
        "UPDATE configuracion_restaurante SET anulacion_modalidad = 'estado_solo' \
         WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("cambiar modalidad");

    let resumen = DashboardService::resumen_mes(&pool, user_id, 2026, 8)
        .await
        .expect("resumen mes estado_solo");
    assert_eq!(
        resumen.total_ventas,
        Decimal::from_str("100.00").unwrap(),
        "estado_solo debe incluir la venta anulada"
    );
}

/* ── F4-5: idempotency key scoped por venta en anulación ─────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn idempotency_key_reutilizada_en_otra_venta_conflicto(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_a = create_test_venta(&pool, user_id).await;
    let venta_b = create_test_venta(&pool, user_id).await;
    let key = "idempotencia-f4-5";

    VentaService::anular(
        &pool,
        venta_a,
        user_id,
        AnularVentaRequest {
            motivo: Some("Duplicado".into()),
            idempotency_key: Some(key.to_string()),
        },
    )
    .await
    .expect("anular venta A");

    /* La misma clave en OTRA venta no puede ser un reintento: conflicto. */
    let err = VentaService::anular(
        &pool,
        venta_b,
        user_id,
        AnularVentaRequest {
            motivo: Some("Duplicado".into()),
            idempotency_key: Some(key.to_string()),
        },
    )
    .await
    .expect_err("reutilizar clave en otra venta debe fallar");
    assert!(matches!(err, AppError::Conflict(_)));
}
