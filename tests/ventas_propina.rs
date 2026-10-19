// sentinel-disable-file sqlx-query-sin-macro
/* [D11a–c] Propina local por venta: `add_tip=true` suma al valor actual y
 * `add_tip=false` lo sustituye, incluido 0 (quitar la propina). Se prueba la
 * secuencia de las fichas: 10 base, +2 (suma), 3 y luego 5 (sustitución), 0. */

use glory_backend::repositories::VentaRepository;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

async fn crear_venta(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(format!("propina-{user_id}@example.com"))
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("crear usuario de prueba");
    let venta_id = Uuid::new_v4();
    sqlx::query(
        r"INSERT INTO ventas (id, user_id, fecha, descripcion, iva_porcentaje, turno, canal,
           metodo_pago, importe_base, importe_iva)
          VALUES ($1, $2, CURRENT_DATE, 'Test propina', 10, 'mediodia',
                  'comedor', 'efectivo', 10.00, 0)",
    )
    .bind(venta_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("crear venta de prueba");
    venta_id
}

async fn propina_tras(pool: &PgPool, venta_id: Uuid, importe: i64, add_tip: bool) -> Decimal {
    let user_id: Uuid = sqlx::query_scalar("SELECT user_id FROM ventas WHERE id = $1")
        .bind(venta_id)
        .fetch_one(pool)
        .await
        .expect("leer usuario de la venta");
    VentaRepository::actualizar_propina(pool, venta_id, user_id, Decimal::from(importe), add_tip)
        .await
        .expect("actualizar propina")
        .expect("venta encontrada")
        .propina
}

#[sqlx::test(migrations = "./migrations")]
async fn d11a_suma_con_add_tip_true(pool: PgPool) {
    let venta = crear_venta(&pool).await;
    assert_eq!(propina_tras(&pool, venta, 2, true).await, Decimal::from(2));
}

#[sqlx::test(migrations = "./migrations")]
async fn d11b_sustituye_con_add_tip_false(pool: PgPool) {
    let venta = crear_venta(&pool).await;
    propina_tras(&pool, venta, 3, true).await;
    assert_eq!(propina_tras(&pool, venta, 5, false).await, Decimal::from(5));
}

#[sqlx::test(migrations = "./migrations")]
async fn d11c_cero_con_add_tip_false_deja_sin_residuo(pool: PgPool) {
    let venta = crear_venta(&pool).await;
    propina_tras(&pool, venta, 5, true).await;
    assert_eq!(propina_tras(&pool, venta, 0, false).await, Decimal::ZERO);
}
