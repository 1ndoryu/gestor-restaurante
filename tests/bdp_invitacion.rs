// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [18AB-4] Guillermo (18/11): la web impide marcar como invitación un artículo
 * cuya ficha BDP tiene «No permitir como invitación». BDP no lo comprueba
 * (`Orders/Create` acepta `Invitation: true` y deja la línea a cero, probado
 * el 18/11 en el demo), así que la frontera es esta: `InvitacionService`
 * consulta `bdp_article_map` antes de escribir la venta. */

use rust_decimal::Decimal;
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;

use glory_backend::errors::AppError;
use glory_backend::models::CrearVentaLineaRequest;
use glory_backend::repositories::{
    BdpArticleMapRepository, BdpArticleUpsertData, VentaLineaRepository,
};
use glory_backend::services::InvitacionService;

async fn create_test_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let email = format!("invitacion-test-{id}@example.com");
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(&email)
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("create_test_user failed");
    id
}

/* Mapeo BDP con la marca de la ficha. `prohibida: None` reproduce el caso de
 * `ExportArticles`, que no manda el campo. */
async fn sembrar_articulo(pool: &PgPool, user_id: Uuid, codigo: &str, prohibida: Option<bool>) {
    BdpArticleMapRepository::upsert_from_bdp(
        pool,
        user_id,
        &BdpArticleUpsertData {
            bdp_code: codigo,
            descripcion: "ARTÍCULO DE PRUEBA",
            precio_tarifa1: Decimal::from_str("6.00").expect("precio"),
            iva_pct: Decimal::from_str("10.00").expect("iva"),
            departamento: 1,
            familia: 1,
            subfamilia: 1,
            activo: true,
            barcode: "",
            stock_actual: Decimal::ZERO,
            inventariable: None,
            invitacion_prohibida: prohibida,
        },
    )
    .await
    .expect("upsert del mapeo");
}

fn linea(codigo: &str, descripcion: &str, invitada: bool) -> CrearVentaLineaRequest {
    CrearVentaLineaRequest {
        articulo_codigo: Some(codigo.into()),
        descripcion: descripcion.into(),
        cantidad: Some(Decimal::ONE),
        precio_unitario: Decimal::from_str("6.00").expect("precio"),
        iva_pct: Some(Decimal::from_str("10.00").expect("iva")),
        descuento: Some(Decimal::ZERO),
        invitacion: Some(invitada),
        comentario: None,
    }
}

/* [21AB-5b] Con el interruptor apagado, ninguna invitación pasa: es lo que
 * Guillermo pidió («deja las invitaciones desactivadas» hasta que el ticket
 * muestre el artículo pedido). Este test fija ese estado. */
#[sqlx::test(migrations = "./migrations")]
async fn con_el_interruptor_apagado_ninguna_invitacion_pasa(pool: PgPool) {
    /* `if` en vez de `assert!`: con el valor constante clippy lo marca como
     * aserción trivial, y aquí lo que se quiere es avisar de que este test
     * caduca el día que la usuaria encienda el interruptor. */
    if glory_backend::services::INVITACIONES_HABILITADAS {
        panic!("si se enciende, este test debe reescribirse con la prueba del ticket");
    }
    let user_id = create_test_user(&pool).await;
    sembrar_articulo(&pool, user_id, "1004", Some(false)).await;

    let error =
        InvitacionService::validar_lineas(&pool, user_id, &[linea("1004", "Permitida", true)])
            .await
            .expect_err("el interruptor apagado bloquea cualquier invitación");
    match error {
        AppError::Validation(mensaje) => assert!(
            mensaje.contains("invitaciones están desactivadas"),
            "mensaje: {mensaje}"
        ),
        otro => panic!("se esperaba Validation (422), llegó {otro:?}"),
    }
}

/* Con el interruptor apagado, la marca de la ficha no llega a mirarse: el
 * servicio rechaza cualquier invitación antes de consultar el catálogo. Las
 * reglas por artículo se prueban en `src/services/invitacion.rs`, contra las
 * funciones internas, para que sigan cubiertas mientras el interruptor esté
 * apagado. */
#[sqlx::test(migrations = "./migrations")]
async fn deja_pasar_la_misma_linea_sin_invitar(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    sembrar_articulo(&pool, user_id, "1002", Some(true)).await;

    InvitacionService::validar_lineas(&pool, user_id, &[linea("1002", "Copa", false)])
        .await
        .expect("cobrada sí se puede");
}

/* La marca sobrevive al siguiente sync del catálogo. */
#[sqlx::test(migrations = "./migrations")]
async fn la_marca_sobrevive_al_sync_sin_el_campo(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    sembrar_articulo(&pool, user_id, "1005", Some(true)).await;
    sembrar_articulo(&pool, user_id, "1005", None).await;

    let map = BdpArticleMapRepository::buscar_por_codigo(&pool, user_id, "1005")
        .await
        .expect("buscar")
        .expect("el mapeo existe");
    assert_eq!(
        map.invitacion_prohibida,
        Some(true),
        "un sync sin el campo no puede borrar la marca"
    );
}

/* La invitación se persiste en la línea: es de donde la lee el payload BDP. */
#[sqlx::test(migrations = "./migrations")]
async fn la_linea_guarda_la_invitacion(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta_id: Uuid = sqlx::query_scalar(
        "INSERT INTO ventas (id, user_id, fecha, iva_porcentaje, turno, canal, metodo_pago, \
         importe_base, importe_iva) VALUES ($1, $2, CURRENT_DATE, 10, 'manana', 'comedor', \
         'efectivo', 6.00, 0.60) RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("venta de prueba");

    let creadas =
        VentaLineaRepository::crear_batch(&pool, venta_id, &[linea("1002", "Copa", true)])
            .await
            .expect("insertar líneas");

    assert!(creadas[0].invitacion, "la marca llega a la base");
    let leidas = VentaLineaRepository::listar_por_venta(&pool, venta_id)
        .await
        .expect("leer líneas");
    assert!(leidas[0].invitacion, "y se lee de vuelta");
}
