// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/*! [21AB-5d] La cabecera de una venta con líneas sale de las líneas.
 *
 * El frontend manda `importe_base`/`importe_iva` junto con las líneas, pero la
 * API es pública: un cliente que solo manda `lineas` dejaba la cabecera con el
 * total viejo. En el TPV demo quedó una venta de 1,05 con una línea de 6
 * unidades y 6,30 en BDP. El importe no se confía a lo que no venga. */

use glory_backend::models::{
    ActualizarVentaRequest, CanalVenta, CrearVentaLineaRequest, CrearVentaRequest, MetodoPago,
    Turno,
};
use glory_backend::repositories::venta::VentaRepository;
use glory_backend::repositories::venta_linea::VentaLineaRepository;
use glory_backend::services::VentaService;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;

async fn usuario(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(format!("importe-{id}@example.com"))
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("crear usuario de prueba");
    id
}

fn linea(cantidad: u32, precio: &str) -> CrearVentaLineaRequest {
    CrearVentaLineaRequest {
        articulo_codigo: Some("1001".into()),
        descripcion: "COCA-COLA".into(),
        cantidad: Some(Decimal::from(cantidad)),
        precio_unitario: Decimal::from_str(precio).expect("precio válido"),
        iva_pct: Some(Decimal::from(10)),
        descuento: Some(Decimal::ZERO),
        invitacion: None,
        comentario: None,
    }
}

fn venta(lineas: Vec<CrearVentaLineaRequest>) -> CrearVentaRequest {
    CrearVentaRequest {
        fecha: chrono::NaiveDate::from_ymd_opt(2026, 11, 23).expect("fecha válida"),
        comensales: Some(1),
        descripcion: Some("Prueba de importe derivado".into()),
        iva_porcentaje: Decimal::from(10),
        turno: Turno::Mediodia,
        canal: CanalVenta::Comedor,
        metodo_pago: MetodoPago::Efectivo,
        importe_base: Decimal::new(105, 2),
        importe_iva: Decimal::ZERO,
        lineas: Some(lineas),
        mesa_id: None,
    }
}

fn actualizar(lineas: Vec<CrearVentaLineaRequest>) -> ActualizarVentaRequest {
    ActualizarVentaRequest {
        fecha: None,
        comensales: None,
        descripcion: None,
        iva_porcentaje: None,
        turno: None,
        canal: None,
        metodo_pago: None,
        importe_base: None,
        importe_iva: None,
        lineas: Some(lineas),
    }
}

/// Solo líneas, sin importes: la cabecera se recalcula (6 × 1,05 = 6,30 + 0,63).
#[sqlx::test(migrations = "./migrations")]
async fn sin_importes_la_cabecera_sale_de_las_lineas(pool: PgPool) {
    let user_id = usuario(&pool).await;
    let creada = VentaService::create(&pool, user_id, venta(vec![linea(1, "1.05")]))
        .await
        .expect("crear la venta base");

    let editada = VentaService::update(
        &pool,
        creada.id,
        user_id,
        actualizar(vec![linea(6, "1.05")]),
    )
    .await
    .expect("editar solo las líneas");

    assert_eq!(
        editada.importe_base,
        Decimal::new(630, 2),
        "base = 6 × 1,05"
    );
    assert_eq!(
        editada.importe_iva,
        Decimal::new(63, 2),
        "iva = 10 % de la base"
    );

    let guardada = VentaRepository::find_by_id(&pool, creada.id, user_id)
        .await
        .expect("releer la venta")
        .expect("la venta existe");
    assert_eq!(
        guardada.importe_base,
        Decimal::new(630, 2),
        "lo guardado coincide con lo devuelto"
    );
}

/// El control: si el cliente manda los importes, se respetan tal cual.
#[sqlx::test(migrations = "./migrations")]
async fn con_importes_explicitos_no_se_tocan(pool: PgPool) {
    let user_id = usuario(&pool).await;
    let creada = VentaService::create(&pool, user_id, venta(vec![linea(1, "1.05")]))
        .await
        .expect("crear la venta base");

    let mut req = actualizar(vec![linea(6, "1.05")]);
    req.importe_base = Some(Decimal::new(100, 2));
    req.importe_iva = Some(Decimal::new(10, 2));
    let editada = VentaService::update(&pool, creada.id, user_id, req)
        .await
        .expect("editar con importes");

    assert_eq!(editada.importe_base, Decimal::new(100, 2));
    assert_eq!(editada.importe_iva, Decimal::new(10, 2));
}

/// Invitada y descuento: la invitada no suma y el descuento va en porcentaje,
/// igual que `calcularTotales` del frontend. La invitación se escribe por el
/// repositorio porque el servicio la bloquea a propósito (`21AB-5b`): aquí se
/// comprueba solo la fórmula de la cabecera.
#[sqlx::test(migrations = "./migrations")]
async fn invitada_y_descuento_cuentan_como_el_frontend(pool: PgPool) {
    let user_id = usuario(&pool).await;
    let creada = VentaService::create(&pool, user_id, venta(vec![linea(1, "1.05")]))
        .await
        .expect("crear la venta base");

    let mut con_descuento = linea(2, "10.00");
    con_descuento.descuento = Some(Decimal::from(50));
    let mut invitada = linea(3, "5.00");
    invitada.invitacion = Some(true);
    let mut tx = pool.begin().await.expect("abrir transacción");
    VentaLineaRepository::reemplazar_conn(&mut tx, creada.id, &[con_descuento, invitada])
        .await
        .expect("escribir las líneas");
    tx.commit().await.expect("confirmar las líneas");

    let editada = VentaRepository::recalcular_importes(&pool, creada.id, user_id)
        .await
        .expect("recalcular")
        .expect("la venta existe");

    assert_eq!(
        editada.importe_base,
        Decimal::new(1000, 2),
        "2 × 10,00 − 50 %"
    );
    assert_eq!(editada.importe_iva, Decimal::new(100, 2));
}
