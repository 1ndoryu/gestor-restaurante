/* [BDP-TEST-B] Tests de integración DB para bdp_article_map.
 * Usa #[sqlx::test(migrations = "./migrations")] — crea BD temporal, aplica migraciones, destruye.
 * NO contacta al servidor BDP — solo valida operaciones CRUD contra PostgreSQL. */

use sqlx::PgPool;
use uuid::Uuid;

use glory_backend::models::{ActualizarBdpArticleMapRequest, CrearBdpArticleMapRequest};
use glory_backend::repositories::{BdpArticleMapRepository, BdpArticleUpsertData};

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

/* ── CRUD roundtrip ──────────────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_crear_y_listar_article_map(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let req = CrearBdpArticleMapRequest {
        articulo_glory_codigo: "CAFE001".into(),
        articulo_bdp_codigo: "1001".into(),
        articulo_bdp_nombre: Some("CAFE BOMBON".into()),
    };

    let created = BdpArticleMapRepository::crear(&pool, user_id, &req)
        .await
        .expect("crear should succeed");

    assert_eq!(created.articulo_glory_codigo, "CAFE001");
    assert_eq!(created.articulo_bdp_codigo, "1001");
    assert_eq!(created.articulo_bdp_nombre, "CAFE BOMBON");
    assert_eq!(created.user_id, user_id);

    let list = BdpArticleMapRepository::listar(&pool, user_id)
        .await
        .expect("listar should succeed");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, created.id);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_obtener_por_id(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let req = CrearBdpArticleMapRequest {
        articulo_glory_codigo: "TOST01".into(),
        articulo_bdp_codigo: "2001".into(),
        articulo_bdp_nombre: Some("TOSTADA".into()),
    };

    let created = BdpArticleMapRepository::crear(&pool, user_id, &req)
        .await
        .unwrap();

    let found = BdpArticleMapRepository::obtener(&pool, created.id, user_id)
        .await
        .expect("obtener should succeed")
        .expect("should find the map");

    assert_eq!(found.id, created.id);
    assert_eq!(found.articulo_glory_codigo, "TOST01");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_obtener_wrong_user_returns_none(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let other_user = create_test_user(&pool).await;
    let req = CrearBdpArticleMapRequest {
        articulo_glory_codigo: "SEC01".into(),
        articulo_bdp_codigo: "3001".into(),
        articulo_bdp_nombre: None,
    };

    let created = BdpArticleMapRepository::crear(&pool, user_id, &req)
        .await
        .unwrap();

    let result = BdpArticleMapRepository::obtener(&pool, created.id, other_user)
        .await
        .expect("should not error");
    assert!(result.is_none(), "Other user should not see the map");
}

/* ── UPSERT (ON CONFLICT) ───────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_upsert_actualiza_codigo_bdp(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let req1 = CrearBdpArticleMapRequest {
        articulo_glory_codigo: "ZUMO01".into(),
        articulo_bdp_codigo: "5001".into(),
        articulo_bdp_nombre: Some("Zumo viejo".into()),
    };

    let first = BdpArticleMapRepository::crear(&pool, user_id, &req1)
        .await
        .unwrap();

    /* Mismo articulo_glory_codigo → UPSERT */
    let req2 = CrearBdpArticleMapRequest {
        articulo_glory_codigo: "ZUMO01".into(),
        articulo_bdp_codigo: "5002".into(),
        articulo_bdp_nombre: Some("Zumo nuevo".into()),
    };

    let second = BdpArticleMapRepository::crear(&pool, user_id, &req2)
        .await
        .unwrap();

    /* Debe ser el mismo ID (upsert, no insert nuevo) */
    assert_eq!(first.id, second.id, "UPSERT should keep same ID");
    assert_eq!(second.articulo_bdp_codigo, "5002");
    assert_eq!(second.articulo_bdp_nombre, "Zumo nuevo");

    /* Solo debe existir 1 registro */
    let list = BdpArticleMapRepository::listar(&pool, user_id)
        .await
        .unwrap();
    assert_eq!(list.len(), 1, "UPSERT should not create duplicate");
}

/* ── buscar_por_codigo ───────────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_buscar_por_codigo(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let req = CrearBdpArticleMapRequest {
        articulo_glory_codigo: "ENSALADA01".into(),
        articulo_bdp_codigo: "7001".into(),
        articulo_bdp_nombre: Some("Ensalada César".into()),
    };
    BdpArticleMapRepository::crear(&pool, user_id, &req)
        .await
        .unwrap();

    let found = BdpArticleMapRepository::buscar_por_codigo(&pool, user_id, "ENSALADA01")
        .await
        .expect("buscar_por_codigo should succeed")
        .expect("should find by code");

    assert_eq!(found.articulo_bdp_codigo, "7001");
    assert_eq!(found.articulo_bdp_nombre, "Ensalada César");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_buscar_por_codigo_inexistente(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    let result = BdpArticleMapRepository::buscar_por_codigo(&pool, user_id, "NOEXISTE")
        .await
        .expect("should not error");
    assert!(result.is_none(), "Non-existent code should return None");
}

/* ── actualizar ──────────────────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_actualizar_parcial(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let req = CrearBdpArticleMapRequest {
        articulo_glory_codigo: "UPD01".into(),
        articulo_bdp_codigo: "8001".into(),
        articulo_bdp_nombre: Some("Original".into()),
    };
    let created = BdpArticleMapRepository::crear(&pool, user_id, &req)
        .await
        .unwrap();

    let patch = ActualizarBdpArticleMapRequest {
        articulo_bdp_codigo: Some("8002".into()),
        articulo_bdp_nombre: Some("Actualizado".into()),
    };
    let updated = BdpArticleMapRepository::actualizar(&pool, created.id, user_id, &patch)
        .await
        .expect("actualizar should succeed")
        .expect("should find and update");

    assert_eq!(updated.articulo_bdp_codigo, "8002");
    assert_eq!(updated.articulo_bdp_nombre, "Actualizado");
    /* articulo_glory_codigo no cambia */
    assert_eq!(updated.articulo_glory_codigo, "UPD01");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_actualizar_wrong_user_returns_none(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let other_user = create_test_user(&pool).await;
    let req = CrearBdpArticleMapRequest {
        articulo_glory_codigo: "UPD02".into(),
        articulo_bdp_codigo: "8101".into(),
        articulo_bdp_nombre: None,
    };
    let created = BdpArticleMapRepository::crear(&pool, user_id, &req)
        .await
        .unwrap();

    let patch = ActualizarBdpArticleMapRequest {
        articulo_bdp_codigo: Some("HACK".into()),
        articulo_bdp_nombre: None,
    };
    let result = BdpArticleMapRepository::actualizar(&pool, created.id, other_user, &patch)
        .await
        .expect("should not error");
    assert!(result.is_none(), "Other user cannot update");
}

/* ── eliminar ────────────────────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_eliminar_article_map(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let req = CrearBdpArticleMapRequest {
        articulo_glory_codigo: "DEL01".into(),
        articulo_bdp_codigo: "9001".into(),
        articulo_bdp_nombre: None,
    };
    let created = BdpArticleMapRepository::crear(&pool, user_id, &req)
        .await
        .unwrap();

    let deleted = BdpArticleMapRepository::eliminar(&pool, created.id, user_id)
        .await
        .expect("eliminar should succeed");
    assert!(deleted, "Should report deleted");

    let not_found = BdpArticleMapRepository::obtener(&pool, created.id, user_id)
        .await
        .expect("obtener should not error");
    assert!(not_found.is_none(), "Deleted map should not exist");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_eliminar_wrong_user_returns_false(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let other_user = create_test_user(&pool).await;
    let req = CrearBdpArticleMapRequest {
        articulo_glory_codigo: "DEL02".into(),
        articulo_bdp_codigo: "9101".into(),
        articulo_bdp_nombre: None,
    };
    let created = BdpArticleMapRepository::crear(&pool, user_id, &req)
        .await
        .unwrap();

    let deleted = BdpArticleMapRepository::eliminar(&pool, created.id, other_user)
        .await
        .expect("should not error");
    assert!(!deleted, "Other user cannot delete");

    /* Original still exists */
    let still_there = BdpArticleMapRepository::obtener(&pool, created.id, user_id)
        .await
        .unwrap();
    assert!(still_there.is_some(), "Original should still exist");
}

/* ── listar con múltiples registros ──────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_listar_ordenado_por_codigo(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    for (codigo, bdp_codigo) in [("ZZZ", "3"), ("AAA", "1"), ("MMM", "2")] {
        let req = CrearBdpArticleMapRequest {
            articulo_glory_codigo: codigo.into(),
            articulo_bdp_codigo: bdp_codigo.into(),
            articulo_bdp_nombre: None,
        };
        BdpArticleMapRepository::crear(&pool, user_id, &req)
            .await
            .unwrap();
    }

    let list = BdpArticleMapRepository::listar(&pool, user_id)
        .await
        .unwrap();
    assert_eq!(list.len(), 3);
    /* ORDER BY articulo_glory_codigo → AAA, MMM, ZZZ */
    assert_eq!(list[0].articulo_glory_codigo, "AAA");
    assert_eq!(list[1].articulo_glory_codigo, "MMM");
    assert_eq!(list[2].articulo_glory_codigo, "ZZZ");
}

/* ── aislamiento entre usuarios ──────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_aislamiento_entre_usuarios(pool: PgPool) {
    let user_a = create_test_user(&pool).await;
    let user_b = create_test_user(&pool).await;

    let req_a = CrearBdpArticleMapRequest {
        articulo_glory_codigo: "SHARED_CODE".into(),
        articulo_bdp_codigo: "A001".into(),
        articulo_bdp_nombre: Some("Artículo A".into()),
    };
    let req_b = CrearBdpArticleMapRequest {
        articulo_glory_codigo: "SHARED_CODE".into(),
        articulo_bdp_codigo: "B001".into(),
        articulo_bdp_nombre: Some("Artículo B".into()),
    };

    BdpArticleMapRepository::crear(&pool, user_a, &req_a)
        .await
        .unwrap();
    BdpArticleMapRepository::crear(&pool, user_b, &req_b)
        .await
        .unwrap();

    let list_a = BdpArticleMapRepository::listar(&pool, user_a)
        .await
        .unwrap();
    let list_b = BdpArticleMapRepository::listar(&pool, user_b)
        .await
        .unwrap();

    assert_eq!(list_a.len(), 1);
    assert_eq!(list_b.len(), 1);
    assert_eq!(list_a[0].articulo_bdp_codigo, "A001");
    assert_eq!(list_b[0].articulo_bdp_codigo, "B001");
}

/* ── upsert_from_bdp (F9.1) ──────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_upsert_from_bdp_crea_nuevo(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let data = BdpArticleUpsertData {
        bdp_code: "1001",
        descripcion: "CAFE BOMBON",
        precio_tarifa1: rust_decimal::Decimal::new(250, 2),
        iva_pct: rust_decimal::Decimal::new(1000, 2),
        departamento: 1,
        familia: 1,
        subfamilia: 1,
        activo: true,
        barcode: "8412345678901",
        stock_actual: rust_decimal::Decimal::new(100, 2),
    };

    let changed = BdpArticleMapRepository::upsert_from_bdp(&pool, user_id, &data)
        .await
        .expect("upsert should succeed");

    assert!(changed, "new article should return true (created)");

    let list = BdpArticleMapRepository::listar(&pool, user_id)
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].articulo_glory_codigo, "1001");
    assert_eq!(list[0].descripcion, "CAFE BOMBON");
    assert_eq!(list[0].articulo_bdp_codigo, "1001");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_upsert_from_bdp_actualiza_existente(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let data1 = BdpArticleUpsertData {
        bdp_code: "2001",
        descripcion: "TOSTADA",
        precio_tarifa1: rust_decimal::Decimal::new(350, 2),
        iva_pct: rust_decimal::Decimal::new(1000, 2),
        departamento: 1,
        familia: 2,
        subfamilia: 1,
        activo: true,
        barcode: "",
        stock_actual: rust_decimal::Decimal::new(50, 2),
    };

    BdpArticleMapRepository::upsert_from_bdp(&pool, user_id, &data1)
        .await
        .unwrap();

    /* Segundo upsert con precio cambiado */
    let data2 = BdpArticleUpsertData {
        bdp_code: "2001",
        descripcion: "TOSTADA",
        precio_tarifa1: rust_decimal::Decimal::new(400, 2), /* precio cambió */
        iva_pct: rust_decimal::Decimal::new(1000, 2),
        departamento: 1,
        familia: 2,
        subfamilia: 1,
        activo: true,
        barcode: "",
        stock_actual: rust_decimal::Decimal::new(50, 2),
    };

    let changed = BdpArticleMapRepository::upsert_from_bdp(&pool, user_id, &data2)
        .await
        .unwrap();

    assert!(changed, "updated price should return true");

    let list = BdpArticleMapRepository::listar(&pool, user_id)
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].precio_tarifa1, rust_decimal::Decimal::new(400, 2));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_upsert_from_bdp_sin_cambios(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let data = BdpArticleUpsertData {
        bdp_code: "3001",
        descripcion: "AGUA",
        precio_tarifa1: rust_decimal::Decimal::new(100, 2),
        iva_pct: rust_decimal::Decimal::new(1000, 2),
        departamento: 1,
        familia: 1,
        subfamilia: 1,
        activo: true,
        barcode: "",
        stock_actual: rust_decimal::Decimal::new(100, 2),
    };

    BdpArticleMapRepository::upsert_from_bdp(&pool, user_id, &data)
        .await
        .unwrap();

    /* Mismo upsert idéntico — no debería reportar cambio */
    let changed = BdpArticleMapRepository::upsert_from_bdp(&pool, user_id, &data)
        .await
        .unwrap();

    assert!(
        !changed,
        "identical upsert should return false (no changes)"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn test_upsert_from_bdp_desactiva_articulo(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let data_active = BdpArticleUpsertData {
        bdp_code: "4001",
        descripcion: "VINO",
        precio_tarifa1: rust_decimal::Decimal::new(800, 2),
        iva_pct: rust_decimal::Decimal::new(2100, 2),
        departamento: 2,
        familia: 3,
        subfamilia: 1,
        activo: true,
        barcode: "123456789",
        stock_actual: rust_decimal::Decimal::new(20, 2),
    };

    BdpArticleMapRepository::upsert_from_bdp(&pool, user_id, &data_active)
        .await
        .unwrap();

    let data_inactive = BdpArticleUpsertData {
        activo: false,
        ..data_active
    };

    let changed = BdpArticleMapRepository::upsert_from_bdp(&pool, user_id, &data_inactive)
        .await
        .unwrap();

    assert!(changed, "deactivating should return true");

    let list = BdpArticleMapRepository::listar(&pool, user_id)
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert!(!list[0].activo);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_upsert_from_bdp_aisla_usuarios(pool: PgPool) {
    let user_a = create_test_user(&pool).await;
    let user_b = create_test_user(&pool).await;

    let data_a = BdpArticleUpsertData {
        bdp_code: "5001",
        descripcion: "ARTICULO A",
        precio_tarifa1: rust_decimal::Decimal::new(100, 2),
        iva_pct: rust_decimal::Decimal::new(1000, 2),
        departamento: 1,
        familia: 1,
        subfamilia: 1,
        activo: true,
        barcode: "",
        stock_actual: rust_decimal::Decimal::new(30, 2),
    };

    BdpArticleMapRepository::upsert_from_bdp(&pool, user_a, &data_a)
        .await
        .unwrap();

    let list_a = BdpArticleMapRepository::listar(&pool, user_a)
        .await
        .unwrap();
    let list_b = BdpArticleMapRepository::listar(&pool, user_b)
        .await
        .unwrap();

    assert_eq!(list_a.len(), 1);
    assert_eq!(list_b.len(), 0, "user_b should not see user_a articles");
}

/* [247A-10/S1] Stock por almacén por defecto */

#[sqlx::test(migrations = "./migrations")]
async fn test_upsert_stock_crea_almacen_general(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    /* upsert_from_bdp debe propagar el stock a bdp_article_stock */
    let data = BdpArticleUpsertData {
        bdp_code: "STOCK01",
        descripcion: "ARTÍCULO STOCK",
        precio_tarifa1: rust_decimal::Decimal::new(100, 2),
        iva_pct: rust_decimal::Decimal::new(1000, 2),
        departamento: 1,
        familia: 1,
        subfamilia: 1,
        activo: true,
        barcode: "",
        stock_actual: rust_decimal::Decimal::new(12345, 2),
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
    assert_eq!(stock[0].stock, rust_decimal::Decimal::new(12345, 2));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_upsert_stock_actualiza_stock_existente(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    let data1 = BdpArticleUpsertData {
        bdp_code: "STOCK02",
        descripcion: "ARTÍCULO",
        precio_tarifa1: rust_decimal::Decimal::new(100, 2),
        iva_pct: rust_decimal::Decimal::new(1000, 2),
        departamento: 1,
        familia: 1,
        subfamilia: 1,
        activo: true,
        barcode: "",
        stock_actual: rust_decimal::Decimal::new(50, 2),
    };
    let data2 = BdpArticleUpsertData {
        stock_actual: rust_decimal::Decimal::new(75, 2),
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
    assert_eq!(stock[0].stock, rust_decimal::Decimal::new(75, 2));
}
