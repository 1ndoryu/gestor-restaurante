// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
//! [10AA-3/L5.0] Base T2: las restricciones que sostienen las pruebas de
//! aceptación de la fase D. Solo usan una base temporal local; no hacen HTTP.

use sqlx::PgPool;
use uuid::Uuid;

/// Un aviso repetido no puede mover nada dos veces: la clave de idempotencia
/// (evento + comanda + timestamp) lo impide. Es la base de D04 y D08.
#[sqlx::test(migrations = "./migrations")]
async fn el_buzon_es_idempotente_por_clave(pool: PgPool) {
    let user_id = Uuid::new_v4();
    let insertar = |id: Uuid| {
        sqlx::query(
            r"INSERT INTO bdp_avisos_inbox
               (id, user_id, accion, origen, clave_idempotencia, payload, payload_hash)
               VALUES ($1, $2, 'crear', '127.0.0.1', 'evt-1|comanda-15|2026-10-17T14:00:00Z', $3, $4)",
        )
        .bind(id)
        .bind(user_id)
        .bind(serde_json::json!({"comanda": 15}))
        .bind("a".repeat(64))
        .execute(&pool)
    };

    insertar(Uuid::new_v4()).await.expect("primer aviso");

    let repetido = insertar(Uuid::new_v4()).await;
    assert!(repetido.is_err(), "la misma clave de idempotencia no puede entrar dos veces");

    /* Otra clave sí entra: el índice es por clave, no por acción. */
    sqlx::query(
        r"INSERT INTO bdp_avisos_inbox
           (id, user_id, accion, clave_idempotencia, payload, payload_hash)
           VALUES ($1, $2, 'crear', 'evt-2|comanda-15|2026-10-17T14:01:00Z', '{}'::jsonb, $3)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind("b".repeat(64))
    .execute(&pool)
    .await
    .expect("un aviso distinto del mismo pedido sí entra");

    let total: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM bdp_avisos_inbox WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .expect("contar avisos");
    assert_eq!(total, 2);
}

/// Dos entidades de Glory no pueden reclamar el mismo registro de BDP: es la
/// garantía de «una sola comanda» de D04 y del toque simultáneo de D08.
#[sqlx::test(migrations = "./migrations")]
async fn dos_entidades_no_reclaman_el_mismo_id_bdp(pool: PgPool) {
    let user_id = Uuid::new_v4();
    let venta_a = Uuid::new_v4();
    let venta_b = Uuid::new_v4();
    let insertar = |glory_id: Uuid, bdp_id: Option<&'static str>| {
        sqlx::query(
            r"INSERT INTO bdp_id_map (id, user_id, entidad, glory_id, bdp_id, bdp_serie)
               VALUES ($1, $2, 'venta', $3, $4, 'PRUEBA')",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(glory_id.to_string())
        .bind(bdp_id)
        .execute(&pool)
    };

    insertar(venta_a, Some("9001")).await.expect("primera venta vinculada");

    let robada = insertar(venta_b, Some("9001")).await;
    assert!(robada.is_err(), "dos ventas no pueden apuntar a la misma comanda de BDP");

    /* Sin id de BDP no hay conflicto: BDP no lo devuelve en todas las
     * operaciones, y varias entidades pueden estar a la espera a la vez. */
    insertar(venta_b, None).await.expect("sin id de BDP no hay colisión");
    insertar(Uuid::new_v4(), None).await.expect("ni con varias a la espera");

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bdp_id_map WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("contar mapeos");
    assert_eq!(total, 3);
}

/// Bloqueo por artículo y almacén (D26g): mientras una operación esté
/// `pendiente` no puede empezar otra sobre el mismo par, y el cerrojo se
/// libera al pasar a un estado final.
#[sqlx::test(migrations = "./migrations")]
async fn el_bloqueo_de_stock_es_por_articulo_y_almacen(pool: PgPool) {
    let user_id = Uuid::new_v4();
    let insertar = |articulo: &'static str, almacen: i32, id_local: &'static str| {
        sqlx::query(
            r"INSERT INTO bdp_stock_ledger
               (id, user_id, articulo_codigo, almacen, operacion, variacion,
                stock_previo, stock_esperado, id_local, origen)
               VALUES ($1, $2, $3, $4, 'regularizar', 2.500, 10.000, 12.500, $5, 'web')",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(articulo)
        .bind(almacen)
        .bind(id_local)
        .execute(&pool)
    };

    let primero = insertar("1001", 1, "op-1").await.expect("primera operación");
    drop(primero);

    let mismo_par = insertar("1001", 1, "op-2").await;
    assert!(mismo_par.is_err(), "el par artículo-almacén está bloqueado mientras hay una pendiente");

    insertar("1001", 2, "op-3").await.expect("otro almacén no está bloqueado");
    insertar("2002", 1, "op-4").await.expect("otro artículo no está bloqueado");

    sqlx::query(
        "UPDATE bdp_stock_ledger SET estado = 'confirmado', stock_posterior = 12.500 \
         WHERE user_id = $1 AND id_local = 'op-1'",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("confirmar la primera");

    insertar("1001", 1, "op-5")
        .await
        .expect("al confirmar la primera, el par queda libre");

    /* La variación es decimal y nunca nula (D26f). */
    let (variacion, esperado): (rust_decimal::Decimal, rust_decimal::Decimal) = sqlx::query_as(
        "SELECT variacion, stock_esperado FROM bdp_stock_ledger \
         WHERE user_id = $1 AND id_local = 'op-1'",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("leer la fila confirmada");
    assert_eq!(variacion.to_string(), "2.500");
    assert_eq!(esperado.to_string(), "12.500");
}

/// Un pendiente agotado no se apunta dos veces desde el mismo origen, pero sí
/// puede haber varios sin fila de origen (D29, D31).
#[sqlx::test(migrations = "./migrations")]
async fn los_pendientes_agotados_no_duplican_origen(pool: PgPool) {
    let user_id = Uuid::new_v4();
    let origen_id = Uuid::new_v4();
    let insertar = |origen_id: Option<Uuid>| {
        sqlx::query(
            r"INSERT INTO bdp_pendientes_agotados
               (id, user_id, origen, origen_id, dominio, operacion, intentos, ultimo_error)
               VALUES ($1, $2, 'push', $3, 'articulo', 'crear', 5, 'HTTP 500')",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(origen_id)
        .execute(&pool)
    };

    insertar(Some(origen_id)).await.expect("primer agotado");

    let repetido = insertar(Some(origen_id)).await;
    assert!(repetido.is_err(), "el mismo pendiente no se apunta dos veces");

    insertar(None).await.expect("sin origen, una fila");
    insertar(None).await.expect("sin origen, otra fila: no chocan entre sí");

    let total: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM bdp_pendientes_agotados WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .expect("contar agotados");
    assert_eq!(total, 3);
}

/// El cursor es único por dominio y perfil. `perfil` no admite NULL justo por
/// esto: en PostgreSQL los NULL no chocan entre sí y el cursor dejaría de ser
/// único.
#[sqlx::test(migrations = "./migrations")]
async fn el_cursor_es_unico_por_dominio_y_perfil(pool: PgPool) {
    let user_id = Uuid::new_v4();
    let insertar = |dominio: &'static str, perfil: &'static str| {
        sqlx::query(
            r#"INSERT INTO bdp_sync_cursor (id, user_id, dominio, perfil, posicion)
               VALUES ($1, $2, $3, $4, '{"hasta": "2026-10-17T14:00:00Z"}'::jsonb)"#,
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(dominio)
        .bind(perfil)
        .execute(&pool)
    };

    insertar("articulos", "1").await.expect("primer cursor");

    let repetido = insertar("articulos", "1").await;
    assert!(repetido.is_err(), "un dominio y perfil tienen un solo cursor");

    insertar("articulos", "2").await.expect("otro perfil, otro cursor");
    insertar("clientes", "1").await.expect("otro dominio, otro cursor");

    let nulos: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bdp_sync_cursor WHERE user_id = $1 AND perfil IS NULL",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("contar perfiles nulos");
    assert_eq!(nulos, 0, "perfil nunca es NULL: si lo fuera, el índice no protegería");
}
