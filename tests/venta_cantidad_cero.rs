// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/*! [18AB-10] Una línea de cantidad 0 no puede llegar a BDP.
 *
 * `Orders/Create` la acepta (probado el 20/11 en el demo), el total no cambia,
 * y después `Orders/Payment/Add` con `Invoice: true` falla con «Intento de
 * dividir por cero»: la comanda queda cobrada y sin documento, y el error no
 * nombra la línea. La venta se corta antes de escribirse. */

use glory_backend::errors::AppError;
use glory_backend::models::{
    CanalVenta, CrearVentaLineaRequest, CrearVentaRequest, MetodoPago, Turno,
};
use glory_backend::repositories::venta_linea::VentaLineaRepository;
use glory_backend::services::VentaService;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

async fn usuario(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(format!("cantidad-{id}@example.com"))
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("crear usuario de prueba");
    id
}

fn linea(descripcion: &str, cantidad: &str) -> CrearVentaLineaRequest {
    CrearVentaLineaRequest {
        articulo_codigo: Some("1001".into()),
        descripcion: descripcion.into(),
        cantidad: Some(cantidad.parse().expect("cantidad válida")),
        precio_unitario: Decimal::new(105, 2),
        iva_pct: Some(Decimal::from(10)),
        descuento: None,
        invitacion: None,
    }
}

fn venta(lineas: Vec<CrearVentaLineaRequest>) -> CrearVentaRequest {
    CrearVentaRequest {
        fecha: chrono::NaiveDate::from_ymd_opt(2026, 11, 20).expect("fecha válida"),
        comensales: Some(1),
        descripcion: Some("Prueba de cantidad cero".into()),
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

/// Una línea de 0 uds se rechaza con 422 y la venta no llega a existir.
#[sqlx::test(migrations = "./migrations")]
async fn una_linea_de_cero_uds_no_crea_la_venta(pool: PgPool) {
    let user_id = usuario(&pool).await;
    let error = VentaService::create(&pool, user_id, venta(vec![linea("COCA-COLA", "0")]))
        .await
        .expect_err("la cantidad 0 debería rechazarse");
    assert!(
        matches!(error, AppError::Validation(_)),
        "se esperaba un error de validación, llegó {error:?}"
    );
    let ventas: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ventas WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("contar ventas");
    assert_eq!(ventas, 0, "no debe quedar la cabecera de la venta");
}

/// El control del mismo camino: la misma venta con 1 ud sí entra y guarda su línea.
#[sqlx::test(migrations = "./migrations")]
async fn el_control_de_una_ud_si_crea_la_venta(pool: PgPool) {
    let user_id = usuario(&pool).await;
    let creada = VentaService::create(&pool, user_id, venta(vec![linea("COCA-COLA", "1")]))
        .await
        .expect("una línea de 1 ud debe aceptarse");
    let lineas = VentaLineaRepository::listar_por_venta(&pool, creada.id)
        .await
        .expect("leer líneas");
    assert_eq!(lineas.len(), 1);
    assert_eq!(lineas[0].cantidad, Decimal::ONE);
}

/// Una cantidad negativa tampoco pasa: no es una devolución, es una venta.
#[sqlx::test(migrations = "./migrations")]
async fn una_cantidad_negativa_tampoco_pasa(pool: PgPool) {
    let user_id = usuario(&pool).await;
    let error = VentaService::create(&pool, user_id, venta(vec![linea("COCA-COLA", "-1")]))
        .await
        .expect_err("la cantidad negativa debería rechazarse");
    assert!(matches!(error, AppError::Validation(_)));
}

/// El reemplazo de líneas de una venta ya enviada tiene la misma frontera: si
/// la venta ya está en BDP, una línea de 0 uds la dejaría sin poder facturar.
#[sqlx::test(migrations = "./migrations")]
async fn reemplazar_lineas_por_una_de_cero_uds_tampoco_pasa(pool: PgPool) {
    let user_id = usuario(&pool).await;
    let creada = VentaService::create(&pool, user_id, venta(vec![linea("COCA-COLA", "1")]))
        .await
        .expect("crear la venta base");
    let error = VentaService::update(
        &pool,
        creada.id,
        user_id,
        glory_backend::models::ActualizarVentaRequest {
            fecha: None,
            comensales: None,
            descripcion: None,
            iva_porcentaje: None,
            turno: None,
            canal: None,
            metodo_pago: None,
            importe_base: None,
            importe_iva: None,
            lineas: Some(vec![linea("COCA-COLA", "0")]),
        },
    )
    .await
    .expect_err("el reemplazo con 0 uds debería rechazarse");
    assert!(matches!(error, AppError::Validation(_)));
    let lineas = VentaLineaRepository::listar_por_venta(&pool, creada.id)
        .await
        .expect("leer líneas");
    assert_eq!(lineas.len(), 1, "la línea original no se toca");
    assert_eq!(lineas[0].cantidad, Decimal::ONE);
}
