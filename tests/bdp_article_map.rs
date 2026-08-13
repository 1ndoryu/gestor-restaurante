/* [BDP-TEST-B] Tests de integración DB para bdp_article_map.
 * Usa #[sqlx::test(migrations = "./migrations")] — crea BD temporal, aplica migraciones, destruye.
 * NO contacta al servidor BDP — solo valida operaciones CRUD contra PostgreSQL. */

use sqlx::PgPool;
use uuid::Uuid;

use glory_backend::models::{ActualizarBdpArticleMapRequest, CrearBdpArticleMapRequest};
use glory_backend::repositories::{
    BdpArticleMapRepository, BdpArticleUpsertData, BdpArticleUpsertStatus,
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

/* Helper: request clásico de mapeo BDP (sin campos locales) */
fn req_bdp(codigo: &str, bdp: &str, nombre: Option<&str>) -> CrearBdpArticleMapRequest {
    CrearBdpArticleMapRequest {
        articulo_glory_codigo: codigo.into(),
        articulo_bdp_codigo: Some(bdp.into()),
        articulo_bdp_nombre: nombre.map(str::to_string),
        descripcion: None,
        precio_tarifa1: None,
        iva_pct: None,
        departamento: None,
        familia: None,
        subfamilia: None,
        activo: None,
        barcode: None,
    }
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
    }
}

/* ── CRUD roundtrip ──────────────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_crear_y_listar_article_map(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let req = req_bdp("CAFE001", "1001", Some("CAFE BOMBON"));

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
    let req = req_bdp("TOST01", "2001", Some("TOSTADA"));

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
    let req = req_bdp("SEC01", "3001", None);

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
    let req1 = req_bdp("ZUMO01", "5001", Some("Zumo viejo"));

    let first = BdpArticleMapRepository::crear(&pool, user_id, &req1)
        .await
        .unwrap();

    /* Mismo articulo_glory_codigo → UPSERT */
    let req2 = req_bdp("ZUMO01", "5002", Some("Zumo nuevo"));

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
    let req = req_bdp("ENSALADA01", "7001", Some("Ensalada César"));
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
    let req = req_bdp("UPD01", "8001", Some("Original"));
    let created = BdpArticleMapRepository::crear(&pool, user_id, &req)
        .await
        .unwrap();

    let patch = ActualizarBdpArticleMapRequest {
        articulo_bdp_codigo: Some("8002".into()),
        articulo_bdp_nombre: Some("Actualizado".into()),
        descripcion: None,
        precio_tarifa1: None,
        iva_pct: None,
        departamento: None,
        familia: None,
        subfamilia: None,
        activo: None,
        barcode: None,
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
    let req = req_bdp("UPD02", "8101", None);
    let created = BdpArticleMapRepository::crear(&pool, user_id, &req)
        .await
        .unwrap();

    let patch = ActualizarBdpArticleMapRequest {
        articulo_bdp_codigo: Some("HACK".into()),
        articulo_bdp_nombre: None,
        descripcion: None,
        precio_tarifa1: None,
        iva_pct: None,
        departamento: None,
        familia: None,
        subfamilia: None,
        activo: None,
        barcode: None,
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
    let req = req_bdp("DEL01", "9001", None);
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
    let req = req_bdp("DEL02", "9101", None);
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
        let req = req_bdp(codigo, bdp_codigo, None);
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

    let req_a = req_bdp("SHARED_CODE", "A001", Some("Artículo A"));
    let req_b = req_bdp("SHARED_CODE", "B001", Some("Artículo B"));

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

/* ── [128A-1/F2] Catálogo local: origen / local_dirty (M5) ──── */

#[sqlx::test(migrations = "./migrations")]
async fn test_crear_local_marca_origen_local(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let req = CrearBdpArticleMapRequest {
        articulo_glory_codigo: "LOCAL001".into(),
        articulo_bdp_codigo: None,
        articulo_bdp_nombre: None,
        descripcion: Some("Hamburguesa local".into()),
        precio_tarifa1: Some(Decimal::new(500, 2)),
        iva_pct: Some(Decimal::new(1600, 2)),
        departamento: Some(5),
        familia: None,
        subfamilia: None,
        activo: Some(true),
        barcode: None,
    };

    let created = BdpArticleMapRepository::crear(&pool, user_id, &req)
        .await
        .expect("crear should succeed");

    assert_eq!(created.origen, "local");
    assert!(!created.local_dirty, "alta nueva local no es dirty");
    assert_eq!(created.descripcion, "Hamburguesa local");
    assert_eq!(created.precio_tarifa1, Decimal::new(500, 2));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_crear_bdp_sobre_existente_marca_local_dirty(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    /* Fila importada de BDP → origen='bdp', no dirty */
    BdpArticleMapRepository::upsert_from_bdp(
        &pool,
        user_id,
        &bdp_data("DIRTY01", "ARTICULO BDP", Decimal::new(100, 2), true),
    )
    .await
    .unwrap();

    /* Edición local (crear/upsert con campos locales) sobre la fila BDP */
    let req = CrearBdpArticleMapRequest {
        articulo_glory_codigo: "DIRTY01".into(),
        articulo_bdp_codigo: Some("DIRTY01".into()),
        articulo_bdp_nombre: Some("ARTICULO BDP".into()),
        descripcion: Some("Descripción editada localmente".into()),
        precio_tarifa1: None,
        iva_pct: None,
        departamento: None,
        familia: None,
        subfamilia: None,
        activo: None,
        barcode: None,
    };
    let updated = BdpArticleMapRepository::crear(&pool, user_id, &req)
        .await
        .unwrap();

    assert_eq!(updated.origen, "local");
    assert!(updated.local_dirty, "edición local marca dirty (M6)");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_actualizar_campos_locales_marca_dirty(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    BdpArticleMapRepository::upsert_from_bdp(
        &pool,
        user_id,
        &bdp_data("PATCH01", "ORIGINAL BDP", Decimal::new(100, 2), true),
    )
    .await
    .unwrap();

    let map = BdpArticleMapRepository::buscar_por_codigo(&pool, user_id, "PATCH01")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(map.origen, "bdp");
    assert!(!map.local_dirty);

    /* PATCH que toca solo el precio → local + dirty */
    let patch = ActualizarBdpArticleMapRequest {
        articulo_bdp_codigo: None,
        articulo_bdp_nombre: None,
        descripcion: None,
        precio_tarifa1: Some(Decimal::new(999, 2)),
        iva_pct: None,
        departamento: None,
        familia: None,
        subfamilia: None,
        activo: None,
        barcode: None,
    };
    let updated = BdpArticleMapRepository::actualizar(&pool, map.id, user_id, &patch)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(updated.origen, "local");
    assert!(updated.local_dirty);
    assert_eq!(updated.precio_tarifa1, Decimal::new(999, 2));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_actualizar_solo_mapeo_no_marca_dirty(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    BdpArticleMapRepository::upsert_from_bdp(
        &pool,
        user_id,
        &bdp_data("MAP01", "SIN EDITAR", Decimal::new(100, 2), true),
    )
    .await
    .unwrap();

    let map = BdpArticleMapRepository::buscar_por_codigo(&pool, user_id, "MAP01")
        .await
        .unwrap()
        .unwrap();

    /* PATCH que solo cambia el código BDP (mapeo) → no dirty */
    let patch = ActualizarBdpArticleMapRequest {
        articulo_bdp_codigo: Some("MAP99".into()),
        articulo_bdp_nombre: None,
        descripcion: None,
        precio_tarifa1: None,
        iva_pct: None,
        departamento: None,
        familia: None,
        subfamilia: None,
        activo: None,
        barcode: None,
    };
    let updated = BdpArticleMapRepository::actualizar(&pool, map.id, user_id, &patch)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(updated.articulo_bdp_codigo, "MAP99");
    assert_eq!(updated.origen, "bdp");
    assert!(!updated.local_dirty, "cambio de mapeo puro no marca dirty");
}

/* ── [128A-1/F2] M6/M7: el import respeta ediciones locales ── */

#[sqlx::test(migrations = "./migrations")]
async fn test_upsert_bdp_omite_fila_dirty(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    BdpArticleMapRepository::upsert_from_bdp(
        &pool,
        user_id,
        &bdp_data("M6-01", "VERSIÓN BDP", Decimal::new(100, 2), true),
    )
    .await
    .unwrap();

    /* Edición local */
    let map = BdpArticleMapRepository::buscar_por_codigo(&pool, user_id, "M6-01")
        .await
        .unwrap()
        .unwrap();
    let patch = ActualizarBdpArticleMapRequest {
        articulo_bdp_codigo: None,
        articulo_bdp_nombre: None,
        descripcion: None,
        precio_tarifa1: Some(Decimal::new(999, 2)),
        iva_pct: None,
        departamento: None,
        familia: None,
        subfamilia: None,
        activo: None,
        barcode: None,
    };
    BdpArticleMapRepository::actualizar(&pool, map.id, user_id, &patch)
        .await
        .unwrap();

    /* El import trae otro precio, pero la fila está dirty → se omite */
    let status = BdpArticleMapRepository::upsert_from_bdp(
        &pool,
        user_id,
        &bdp_data("M6-01", "VERSIÓN BDP NUEVA", Decimal::new(50, 2), true),
    )
    .await
    .unwrap();

    assert_eq!(status, BdpArticleUpsertStatus::OmitidoLocalDirty);
    assert!(status.es_omitido());

    let after = BdpArticleMapRepository::buscar_por_codigo(&pool, user_id, "M6-01")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        after.precio_tarifa1,
        Decimal::new(999, 2),
        "versión local intacta"
    );
    assert_eq!(after.descripcion, "VERSIÓN BDP", "versión local intacta");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_upsert_bdp_no_reactiva_desactivado_local(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    BdpArticleMapRepository::upsert_from_bdp(
        &pool,
        user_id,
        &bdp_data("M7-01", "PLATO RETIRADO", Decimal::new(100, 2), true),
    )
    .await
    .unwrap();

    /* Desactivación local vía PATCH (no marca dirty: solo activo=false;
     * no es edición de datos, es estado de disponibilidad) */
    let map = BdpArticleMapRepository::buscar_por_codigo(&pool, user_id, "M7-01")
        .await
        .unwrap()
        .unwrap();
    let patch = ActualizarBdpArticleMapRequest {
        articulo_bdp_codigo: None,
        articulo_bdp_nombre: None,
        descripcion: None,
        precio_tarifa1: None,
        iva_pct: None,
        departamento: None,
        familia: None,
        subfamilia: None,
        activo: Some(false),
        barcode: None,
    };
    BdpArticleMapRepository::actualizar(&pool, map.id, user_id, &patch)
        .await
        .unwrap();

    /* BDP la trae activa de nuevo → no se reactiva (M7) */
    let status = BdpArticleMapRepository::upsert_from_bdp(
        &pool,
        user_id,
        &bdp_data("M7-01", "PLATO RETIRADO", Decimal::new(100, 2), true),
    )
    .await
    .unwrap();

    assert_eq!(status, BdpArticleUpsertStatus::OmitidoDesactivado);
    assert!(status.es_omitido());

    let after = BdpArticleMapRepository::buscar_por_codigo(&pool, user_id, "M7-01")
        .await
        .unwrap()
        .unwrap();
    assert!(!after.activo, "el import no reactiva artículos locales");
}

/* ── [128A-1/F2] Defaults de la migración ────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_migracion_defaults_origen_bdp(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    /* Import BDP → defaults de la columna: origen='bdp', local_dirty=false */
    BdpArticleMapRepository::upsert_from_bdp(
        &pool,
        user_id,
        &bdp_data("DEF01", "POR DEFECTO", Decimal::new(100, 2), true),
    )
    .await
    .unwrap();

    let map = BdpArticleMapRepository::buscar_por_codigo(&pool, user_id, "DEF01")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(map.origen, "bdp");
    assert!(!map.local_dirty);
}

/* ── upsert_from_bdp (F9.1) ──────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_upsert_from_bdp_crea_nuevo(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let data = BdpArticleUpsertData {
        barcode: "8412345678901",
        stock_actual: Decimal::new(100, 2),
        ..bdp_data("1001", "CAFE BOMBON", Decimal::new(250, 2), true)
    };

    let status = BdpArticleMapRepository::upsert_from_bdp(&pool, user_id, &data)
        .await
        .expect("upsert should succeed");

    assert_eq!(status, BdpArticleUpsertStatus::Creado);
    assert!(status.es_cambio());

    let list = BdpArticleMapRepository::listar(&pool, user_id)
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].articulo_glory_codigo, "1001");
    assert_eq!(list[0].descripcion, "CAFE BOMBON");
    assert_eq!(list[0].articulo_bdp_codigo, "1001");
    /* [128A-1/F2] Los imports BDP nunca marcan la fila como local. */
    assert_eq!(list[0].origen, "bdp");
    assert!(!list[0].local_dirty);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_upsert_from_bdp_actualiza_existente(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let data1 = BdpArticleUpsertData {
        familia: 2,
        stock_actual: Decimal::new(50, 2),
        ..bdp_data("2001", "TOSTADA", Decimal::new(350, 2), true)
    };

    BdpArticleMapRepository::upsert_from_bdp(&pool, user_id, &data1)
        .await
        .unwrap();

    /* Segundo upsert con precio cambiado */
    let data2 = BdpArticleUpsertData {
        precio_tarifa1: Decimal::new(400, 2), /* precio cambió */
        ..data1
    };

    let status = BdpArticleMapRepository::upsert_from_bdp(&pool, user_id, &data2)
        .await
        .unwrap();

    assert_eq!(status, BdpArticleUpsertStatus::Actualizado);

    let list = BdpArticleMapRepository::listar(&pool, user_id)
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].precio_tarifa1, Decimal::new(400, 2));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_upsert_from_bdp_sin_cambios(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let data = BdpArticleUpsertData {
        descripcion: "AGUA",
        stock_actual: Decimal::new(100, 2),
        ..bdp_data("3001", "", Decimal::new(100, 2), true)
    };

    BdpArticleMapRepository::upsert_from_bdp(&pool, user_id, &data)
        .await
        .unwrap();

    /* Mismo upsert idéntico — no debería reportar cambio */
    let status = BdpArticleMapRepository::upsert_from_bdp(&pool, user_id, &data)
        .await
        .unwrap();

    assert_eq!(status, BdpArticleUpsertStatus::SinCambios);
    assert!(!status.es_cambio());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_upsert_from_bdp_desactiva_articulo(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let data_active = BdpArticleUpsertData {
        descripcion: "VINO",
        precio_tarifa1: Decimal::new(800, 2),
        iva_pct: Decimal::new(2100, 2),
        departamento: 2,
        familia: 3,
        barcode: "123456789",
        stock_actual: Decimal::new(20, 2),
        ..bdp_data("4001", "", Decimal::ZERO, true)
    };

    BdpArticleMapRepository::upsert_from_bdp(&pool, user_id, &data_active)
        .await
        .unwrap();

    let data_inactive = BdpArticleUpsertData {
        activo: false,
        ..data_active
    };

    let status = BdpArticleMapRepository::upsert_from_bdp(&pool, user_id, &data_inactive)
        .await
        .unwrap();

    assert_eq!(status, BdpArticleUpsertStatus::Actualizado);

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
        descripcion: "ARTICULO A",
        stock_actual: Decimal::new(30, 2),
        ..bdp_data("5001", "", Decimal::new(100, 2), true)
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
