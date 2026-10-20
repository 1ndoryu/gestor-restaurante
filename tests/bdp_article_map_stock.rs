// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [BDP-TEST-B] Tests de integración DB para el stock de bdp_article_map.
 * Separados de `bdp_article_map.rs` por dominio (stock vs mapeo) y porque aquel
 * fichero pasaba del límite de 1000 líneas para tests de integración.
 * Usa #[sqlx::test(migrations = "./migrations")] — crea BD temporal, aplica
 * migraciones, destruye. NO contacta al servidor BDP. */

use sqlx::PgPool;
use uuid::Uuid;

use glory_backend::models::CrearBdpArticleMapRequest;
use glory_backend::repositories::{
    AjusteStockError, BdpArticleMapRepository, BdpArticleUpsertData, BdpArticleUpsertStatus,
};
use glory_backend::services::BdpAuditEntry;
use rust_decimal::Decimal;

/* Helper: crea un usuario mínimo para satisfacer FK de bdp_article_map.user_id */
async fn create_test_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let email = format!("bdp-test-{id}@example.com");
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(&email)
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("create_test_user failed");
    id
}

/* Helper: upsert BDP con valores por defecto razonables */
fn bdp_data<'a>(
    codigo: &'a str,
    descripcion: &'a str,
    precio: Decimal,
    activo: bool,
) -> BdpArticleUpsertData<'a> {
    BdpArticleUpsertData {
        bdp_code: codigo,
        descripcion,
        precio_tarifa1: precio,
        iva_pct: Decimal::new(1000, 2),
        departamento: 1,
        familia: 1,
        subfamilia: 1,
        activo,
        barcode: "",
        stock_actual: Decimal::ZERO,
        inventariable: None,
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn test_upsert_stock_crea_almacen_general(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    /* upsert_from_bdp debe propagar el stock a bdp_article_stock */
    let data = BdpArticleUpsertData {
        descripcion: "ARTÍCULO STOCK",
        stock_actual: Decimal::new(12345, 2),
        ..bdp_data("STOCK01", "", Decimal::new(100, 2), true)
    };

    BdpArticleMapRepository::upsert_from_bdp(&pool, user_id, &data)
        .await
        .unwrap();

    let stock = BdpArticleMapRepository::listar_stock(&pool, user_id, None)
        .await
        .unwrap();

    assert_eq!(stock.len(), 1);
    assert_eq!(stock[0].articulo_glory_codigo, "STOCK01");
    assert_eq!(stock[0].warehouse_id, "0");
    assert_eq!(stock[0].warehouse_name, "General");
    assert_eq!(stock[0].stock, Decimal::new(12345, 2));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_upsert_stock_actualiza_stock_existente(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    let data1 = BdpArticleUpsertData {
        descripcion: "ARTÍCULO",
        stock_actual: Decimal::new(50, 2),
        ..bdp_data("STOCK02", "", Decimal::new(100, 2), true)
    };
    let data2 = BdpArticleUpsertData {
        stock_actual: Decimal::new(75, 2),
        ..data1
    };

    BdpArticleMapRepository::upsert_from_bdp(&pool, user_id, &data1)
        .await
        .unwrap();
    BdpArticleMapRepository::upsert_from_bdp(&pool, user_id, &data2)
        .await
        .unwrap();

    let stock = BdpArticleMapRepository::listar_stock(&pool, user_id, Some("0"))
        .await
        .unwrap();
    assert_eq!(stock.len(), 1);
    assert_eq!(stock[0].stock, Decimal::new(75, 2));
}

/* [D26b] El almacén lo elige quien llama: una escritura en el almacén "1" no
 * toca la fila del almacén por defecto "0" del mismo artículo. */
#[sqlx::test(migrations = "./migrations")]
async fn test_upsert_stock_escribe_almacen_indicado(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    /* Fila del almacén por defecto "0". */
    BdpArticleMapRepository::upsert_stock(
        &pool,
        user_id,
        "ALM01",
        "0",
        "General",
        Decimal::new(10, 0),
    )
    .await
    .unwrap();

    /* Mismo artículo en el almacén 1: fila nueva, distinta de la de "0". */
    let insertado = BdpArticleMapRepository::upsert_stock(
        &pool,
        user_id,
        "ALM01",
        "1",
        "Almacén 1",
        Decimal::new(25, 0),
    )
    .await
    .unwrap();
    assert!(insertado, "la fila nueva del almacén 1 debe insertarse");

    let general = BdpArticleMapRepository::listar_stock(&pool, user_id, Some("0"))
        .await
        .unwrap();
    assert_eq!(general.len(), 1);
    assert_eq!(
        general[0].stock,
        Decimal::new(10, 0),
        "el almacén 0 no cambia"
    );

    let almacen1 = BdpArticleMapRepository::listar_stock(&pool, user_id, Some("1"))
        .await
        .unwrap();
    assert_eq!(almacen1.len(), 1);
    assert_eq!(almacen1[0].warehouse_name, "Almacén 1");
    assert_eq!(almacen1[0].stock, Decimal::new(25, 0));
}

/* [128A-1/F3] Stock local editable: ajuste manual con auditoría. */

#[sqlx::test(migrations = "./migrations")]
async fn test_ajustar_stock_crea_fila_con_delta(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    /* Sin fila previa: el ajuste crea la fila con el delta como stock inicial. */
    let (stock, audit_id, resultado_previo) = BdpArticleMapRepository::ajustar_stock(
        &pool,
        user_id,
        "AJ001",
        Decimal::new(1200, 2),
        "Entrada de mercancía",
        None,
        None,
    )
    .await
    .unwrap();

    assert_eq!(stock.articulo_glory_codigo, "AJ001");
    assert_eq!(stock.warehouse_id, "0");
    assert_eq!(stock.warehouse_name, "General");
    assert_eq!(stock.stock, Decimal::new(1200, 2));
    assert!(
        resultado_previo.is_none(),
        "sin clave no hay duplicado previo"
    );

    /* La auditoría registró la operación interna. */
    let audit: BdpAuditEntry = sqlx::query_as("SELECT * FROM bdp_audit_log WHERE id = $1")
        .bind(audit_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(audit.operacion, "stock_ajuste");
    assert_eq!(audit.direccion, "internal");
    assert_eq!(audit.resultado, "exito");
    assert_eq!(audit.target_entity_type.as_deref(), Some("articulo"));

    /* stock_actual del catálogo (snapshot BDP) no se toca. */
    let list = BdpArticleMapRepository::listar(&pool, user_id)
        .await
        .unwrap();
    assert!(list.is_empty(), "el ajuste no crea mapeo de catálogo");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_ajustar_stock_acumula_deltas(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    BdpArticleMapRepository::ajustar_stock(
        &pool,
        user_id,
        "AJ002",
        Decimal::new(1000, 2),
        "Entrada",
        None,
        None,
    )
    .await
    .unwrap();

    /* Salida negativa sobre la fila existente. */
    let (stock, _, _) = BdpArticleMapRepository::ajustar_stock(
        &pool,
        user_id,
        "AJ002",
        Decimal::new(-300, 2),
        "Merma",
        None,
        None,
    )
    .await
    .unwrap();

    assert_eq!(stock.stock, Decimal::new(700, 2));

    let lista = BdpArticleMapRepository::listar_stock(&pool, user_id, None)
        .await
        .unwrap();
    assert_eq!(lista.len(), 1);
    assert_eq!(lista[0].stock, Decimal::new(700, 2));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_ajustar_stock_aisla_usuarios(pool: PgPool) {
    let user_a = create_test_user(&pool).await;
    let user_b = create_test_user(&pool).await;

    BdpArticleMapRepository::ajustar_stock(
        &pool,
        user_a,
        "AJ003",
        Decimal::new(500, 2),
        "Entrada",
        None,
        None,
    )
    .await
    .unwrap();

    let stock_b = BdpArticleMapRepository::listar_stock(&pool, user_b, None)
        .await
        .unwrap();
    assert!(stock_b.is_empty(), "user_b no ve el stock de user_a");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_ajustar_stock_idempotencia_reintento_exitoso(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let key = "ajuste-unica-1";

    let (stock1, _, prev1) = BdpArticleMapRepository::ajustar_stock(
        &pool,
        user_id,
        "AJ004",
        Decimal::new(100, 2),
        "Entrada",
        None,
        Some(key),
    )
    .await
    .unwrap();

    assert_eq!(stock1.stock, Decimal::new(100, 2));
    assert!(prev1.is_none());

    /* Reintento con la misma clave: no aplica el delta de nuevo. */
    let (stock2, _, prev2) = BdpArticleMapRepository::ajustar_stock(
        &pool,
        user_id,
        "AJ004",
        Decimal::new(100, 2),
        "Reintento",
        None,
        Some(key),
    )
    .await
    .unwrap();

    assert_eq!(stock2.stock, Decimal::new(100, 2), "sin doble aplicación");
    assert_eq!(prev2.as_deref(), Some("exito"));
}

/* [128A-1/F3][F3-1] Un ajuste local marca `ajustado_local` y el sync BDP
 * (upsert_from_bdp → upsert_stock) NO lo sobrescribe. */
#[sqlx::test(migrations = "./migrations")]
async fn test_sync_no_pisa_stock_ajustado_local(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    /* Ajuste manual: fuente de verdad local = 50.00, ajustado_local=true. */
    BdpArticleMapRepository::ajustar_stock(
        &pool,
        user_id,
        "STOCK-LOCAL-01",
        Decimal::new(5000, 2),
        "Entrada manual",
        None,
        None,
    )
    .await
    .unwrap();

    /* Import BDP con stock 999.00 → no debe pisar el ajuste local. */
    let data = BdpArticleUpsertData {
        descripcion: "ARTICULO LOCAL AJUSTADO",
        stock_actual: Decimal::new(99900, 2),
        ..bdp_data("STOCK-LOCAL-01", "", Decimal::new(100, 2), true)
    };
    BdpArticleMapRepository::upsert_from_bdp(&pool, user_id, &data)
        .await
        .unwrap();

    let stock = BdpArticleMapRepository::listar_stock(&pool, user_id, Some("0"))
        .await
        .unwrap();
    assert_eq!(stock.len(), 1);
    assert_eq!(
        stock[0].stock,
        Decimal::new(5000, 2),
        "el sync no pisa el stock ajustado localmente"
    );
    assert!(stock[0].ajustado_local);

    /* Sin ajuste local, el sync sigue actualizando (comportamiento normal). */
    let data2 = BdpArticleUpsertData {
        descripcion: "ARTICULO SIN AJUSTE",
        stock_actual: Decimal::new(700, 2),
        ..bdp_data("STOCK-LOCAL-02", "", Decimal::new(100, 2), true)
    };
    BdpArticleMapRepository::upsert_from_bdp(&pool, user_id, &data2)
        .await
        .unwrap();
    let stock2 = BdpArticleMapRepository::listar_stock(&pool, user_id, Some("0"))
        .await
        .unwrap();
    assert_eq!(
        stock2
            .iter()
            .find(|s| s.articulo_glory_codigo == "STOCK-LOCAL-02")
            .unwrap()
            .stock,
        Decimal::new(700, 2)
    );
    assert!(
        !stock2
            .iter()
            .find(|s| s.articulo_glory_codigo == "STOCK-LOCAL-02")
            .unwrap()
            .ajustado_local
    );
}

/* [128A-1/F3][F3-3] Un ajuste que dejaría el stock negativo se rechaza y la
 * transacción se revierte (la fila conserva su valor previo). */
#[sqlx::test(migrations = "./migrations")]
async fn test_ajustar_stock_rechaza_negativo(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    BdpArticleMapRepository::ajustar_stock(
        &pool,
        user_id,
        "NEG01",
        Decimal::new(1000, 2),
        "Entrada",
        None,
        None,
    )
    .await
    .unwrap();

    let err = BdpArticleMapRepository::ajustar_stock(
        &pool,
        user_id,
        "NEG01",
        Decimal::new(-2000, 2),
        "Merma erronea",
        None,
        None,
    )
    .await
    .expect_err("stock negativo debe rechazarse");

    assert!(
        matches!(err, AjusteStockError::StockNegativo(_)),
        "se esperaba StockNegativo, got {err:?}"
    );

    let stock = BdpArticleMapRepository::listar_stock(&pool, user_id, None)
        .await
        .unwrap();
    assert_eq!(stock.len(), 1);
    assert_eq!(
        stock[0].stock,
        Decimal::new(1000, 2),
        "la transacción se revierte y el stock previo se conserva"
    );
}

/* [128A-1/F3][F3-4] El nombre del almacén se deriva del id: 'General' solo
 * para el almacén por defecto "0". */
#[sqlx::test(migrations = "./migrations")]
async fn test_ajustar_stock_warehouse_name_derivado(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    let (general, _, _) = BdpArticleMapRepository::ajustar_stock(
        &pool,
        user_id,
        "WH0",
        Decimal::new(100, 2),
        "Entrada",
        Some("0"),
        None,
    )
    .await
    .unwrap();
    assert_eq!(general.warehouse_id, "0");
    assert_eq!(general.warehouse_name, "General");

    let (otro, _, _) = BdpArticleMapRepository::ajustar_stock(
        &pool,
        user_id,
        "WH7",
        Decimal::new(100, 2),
        "Entrada",
        Some("7"),
        None,
    )
    .await
    .unwrap();
    assert_eq!(otro.warehouse_id, "7");
    assert_eq!(
        otro.warehouse_name, "7",
        "el nombre se deriva del id cuando no es el almacén por defecto"
    );
}

/* [128A-1/F2][F2-3] Fila Omitido* no propaga stock BDP a bdp_article_stock:
 * el import solo escribe stock para filas creadas/actualizadas. */
#[sqlx::test(migrations = "./migrations")]
async fn test_upsert_omite_dirty_sin_escritura_stock(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    /* Alta local (F2-2): local_dirty=true desde el inicio, sin fila de stock. */
    let req = CrearBdpArticleMapRequest {
        articulo_glory_codigo: "F23-01".into(),
        articulo_bdp_codigo: None,
        articulo_bdp_nombre: None,
        descripcion: Some("Articulo local".into()),
        precio_tarifa1: Some(Decimal::new(999, 2)),
        iva_pct: None,
        departamento: None,
        familia: None,
        subfamilia: None,
        activo: None,
        barcode: None,
    };
    BdpArticleMapRepository::crear(&pool, user_id, &req)
        .await
        .unwrap();

    /* El import con stock distinto se omite → el stock local no se toca. */
    let data = BdpArticleUpsertData {
        descripcion: "VERSION BDP NUEVA",
        stock_actual: Decimal::new(555, 2),
        ..bdp_data("F23-01", "", Decimal::new(50, 2), true)
    };
    let status = BdpArticleMapRepository::upsert_from_bdp(&pool, user_id, &data)
        .await
        .unwrap();
    assert_eq!(status, BdpArticleUpsertStatus::OmitidoLocalDirty);

    let stock = BdpArticleMapRepository::listar_stock(&pool, user_id, None)
        .await
        .unwrap();
    assert!(
        stock.is_empty(),
        "una fila omitida por ediciones locales no escribe stock BDP"
    );
}
