/* [BDP-TEST-B] Tests de integración DB para venta_lineas.
 * Usa #[sqlx::test(migrations = "./migrations")] — crea BD temporal, aplica migraciones, destruye.
 * NO contacta al servidor BDP — solo valida operaciones CRUD contra PostgreSQL. */

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;

use glory_backend::models::CrearVentaLineaRequest;
use glory_backend::repositories::venta::{NuevaVenta, VentaRepository};
use glory_backend::repositories::venta_linea::VentaLineaRepository;

/* Helper: crea un usuario mínimo para satisfacer FK */
async fn create_test_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let email = format!("linea-test-{id}@example.com");
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(&email)
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("create_test_user failed");
    id
}

/* Helper: crea una venta válida para usar como padre de las líneas */
async fn create_test_venta(pool: &PgPool, user_id: Uuid) -> Uuid {
    let data = NuevaVenta {
        user_id,
        fecha: NaiveDate::from_ymd_opt(2026, 7, 15).unwrap(),
        comensales: Some(2),
        descripcion: "Test venta for lineas",
        iva_porcentaje: Decimal::from(10),
        turno: "noche",
        canal: "comedor",
        metodo_pago: "efectivo",
        importe_base: Decimal::from_str("20.00").unwrap(),
        importe_iva: Decimal::from_str("2.00").unwrap(),
        reserva_id: None,
        cliente_id: None,
    };
    let venta = VentaRepository::create(pool, &data)
        .await
        .expect("create venta failed");
    venta.id
}

/* Helper: construye una CrearVentaLineaRequest de ejemplo */
fn linea_req(codigo: &str, desc: &str, qty: &str, price: &str) -> CrearVentaLineaRequest {
    CrearVentaLineaRequest {
        articulo_codigo: Some(codigo.into()),
        descripcion: desc.into(),
        cantidad: Some(Decimal::from_str(qty).unwrap()),
        precio_unitario: Decimal::from_str(price).unwrap(),
        iva_pct: Some(Decimal::from(10)),
        descuento: Some(Decimal::ZERO),
    }
}

/* ── crear_batch ─────────────────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_crear_batch_y_listar(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    let lineas = vec![
        linea_req("1001", "Café bombón", "2", "5.00"),
        linea_req("2002", "Tostada", "1", "3.50"),
    ];

    let created = VentaLineaRepository::crear_batch(&pool, venta_id, &lineas)
        .await
        .expect("crear_batch should succeed");

    assert_eq!(created.len(), 2);
    assert_eq!(created[0].descripcion, "Café bombón");
    assert_eq!(created[1].descripcion, "Tostada");

    let listed = VentaLineaRepository::listar_por_venta(&pool, venta_id)
        .await
        .expect("listar_por_venta should succeed");

    assert_eq!(listed.len(), 2);
    /* Verificar que los datos persistieron correctamente */
    assert_eq!(listed[0].articulo_codigo, "1001");
    assert_eq!(listed[1].articulo_codigo, "2002");
    assert_eq!(listed[0].cantidad, Decimal::from_str("2").unwrap());
    assert_eq!(
        listed[1].precio_unitario,
        Decimal::from_str("3.50").unwrap()
    );
}

/* ── crear_batch con descuento ───────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_crear_batch_con_descuento(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    let lineas = vec![CrearVentaLineaRequest {
        articulo_codigo: Some("9999".into()),
        descripcion: "Menú del día".into(),
        cantidad: Some(Decimal::from(2)),
        precio_unitario: Decimal::from_str("12.00").unwrap(),
        iva_pct: Some(Decimal::from(10)),
        descuento: Some(Decimal::from_str("4.00").unwrap()),
    }];

    let created = VentaLineaRepository::crear_batch(&pool, venta_id, &lineas)
        .await
        .unwrap();

    assert_eq!(created[0].descuento, Decimal::from_str("4.00").unwrap());
    assert_eq!(created[0].cantidad, Decimal::from(2));
    assert_eq!(
        created[0].precio_unitario,
        Decimal::from_str("12.00").unwrap()
    );
}

/* ── crear_batch vacío ───────────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_crear_batch_vacio(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    let created = VentaLineaRepository::crear_batch(&pool, venta_id, &[])
        .await
        .expect("empty batch should succeed");

    assert_eq!(created.len(), 0);

    let listed = VentaLineaRepository::listar_por_venta(&pool, venta_id)
        .await
        .unwrap();
    assert_eq!(listed.len(), 0);
}

/* ── listar_por_venta vacío ──────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_listar_por_venta_sin_lineas(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    let listed = VentaLineaRepository::listar_por_venta(&pool, venta_id)
        .await
        .expect("listar should succeed on empty");

    assert_eq!(listed.len(), 0);
}

/* ── eliminar_por_venta ──────────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_eliminar_por_venta(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    let lineas = vec![
        linea_req("1001", "Café", "1", "5.00"),
        linea_req("2002", "Tostada", "1", "3.50"),
        linea_req("3003", "Zumo", "1", "2.00"),
    ];
    VentaLineaRepository::crear_batch(&pool, venta_id, &lineas)
        .await
        .unwrap();

    let deleted = VentaLineaRepository::eliminar_por_venta(&pool, venta_id)
        .await
        .expect("eliminar_por_venta should succeed");

    assert_eq!(deleted, 3, "Should delete all 3 lines");

    let remaining = VentaLineaRepository::listar_por_venta(&pool, venta_id)
        .await
        .unwrap();
    assert_eq!(remaining.len(), 0, "No lines should remain");
}

/* ── eliminar_por_venta con 0 líneas ─────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_eliminar_por_venta_sin_lineas(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id = create_test_venta(&pool, user_id).await;

    let deleted = VentaLineaRepository::eliminar_por_venta(&pool, venta_id)
        .await
        .expect("eliminar on empty should succeed");

    assert_eq!(deleted, 0, "Nothing to delete");
}

/* ── FK constraint: venta inexistente ────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_crear_batch_venta_inexistente_falla(pool: PgPool) {
    let fake_venta_id = Uuid::new_v4();
    let lineas = vec![linea_req("1001", "Ghost item", "1", "5.00")];

    let result = VentaLineaRepository::crear_batch(&pool, fake_venta_id, &lineas).await;
    assert!(
        result.is_err(),
        "FK violation should fail when venta does not exist"
    );
}

/* ── aislamiento: líneas de otra venta no se mezclan ─────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_aislamiento_entre_ventas(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_a = create_test_venta(&pool, user_id).await;
    let venta_b = create_test_venta(&pool, user_id).await;

    VentaLineaRepository::crear_batch(&pool, venta_a, &[linea_req("A001", "Item A", "1", "10.00")])
        .await
        .unwrap();

    VentaLineaRepository::crear_batch(
        &pool,
        venta_b,
        &[
            linea_req("B001", "Item B1", "1", "20.00"),
            linea_req("B002", "Item B2", "1", "30.00"),
        ],
    )
    .await
    .unwrap();

    let lineas_a = VentaLineaRepository::listar_por_venta(&pool, venta_a)
        .await
        .unwrap();
    let lineas_b = VentaLineaRepository::listar_por_venta(&pool, venta_b)
        .await
        .unwrap();

    assert_eq!(lineas_a.len(), 1, "Venta A should have 1 line");
    assert_eq!(lineas_b.len(), 2, "Venta B should have 2 lines");
    assert_eq!(lineas_a[0].articulo_codigo, "A001");
    assert_eq!(lineas_b[0].articulo_codigo, "B001");
    assert_eq!(lineas_b[1].articulo_codigo, "B002");
}

/* ── eliminar_por_venta no afecta otras ventas ───────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_eliminar_no_afecta_otras_ventas(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_a = create_test_venta(&pool, user_id).await;
    let venta_b = create_test_venta(&pool, user_id).await;

    VentaLineaRepository::crear_batch(&pool, venta_a, &[linea_req("A001", "Item A", "1", "10.00")])
        .await
        .unwrap();

    VentaLineaRepository::crear_batch(&pool, venta_b, &[linea_req("B001", "Item B", "1", "20.00")])
        .await
        .unwrap();

    /* Delete lines from venta_a only */
    VentaLineaRepository::eliminar_por_venta(&pool, venta_a)
        .await
        .unwrap();

    let remaining_b = VentaLineaRepository::listar_por_venta(&pool, venta_b)
        .await
        .unwrap();
    assert_eq!(remaining_b.len(), 1, "Venta B lines should not be affected");
    assert_eq!(remaining_b[0].articulo_codigo, "B001");
}
