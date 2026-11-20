// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
//! [18AB-5] Una comanda de BDP se crea una vez. Dos solicitudes simultáneas
//! para la misma venta no pueden producir dos `create_order`: el índice único
//! parcial sobre la auditoría viva y el de `bdp_id_map` lo impiden en la base,
//! no solo en memoria.

use glory_backend::repositories::ConfiguracionRepository;
use glory_backend::services::{fijar_permiso, BdpWriteGuard, EscrituraAutorizar};
use sqlx::PgPool;
use uuid::Uuid;

async fn usuario(pool: &PgPool) -> (Uuid, glory_backend::models::ConfiguracionRestaurante) {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(format!("bdp-unica-{id}@example.com"))
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("crear usuario");
    let mut config = ConfiguracionRepository::obtener_o_crear(pool, id)
        .await
        .expect("configuración");
    config.bdp_base_url = "http://127.0.0.1:18765".into();
    config.bdp_sync_mode = "automatic".into();
    sqlx::query(
        "UPDATE configuracion_restaurante SET bdp_base_url = $2, bdp_sync_mode = 'automatic' \
         WHERE user_id = $1",
    )
    .bind(id)
    .bind(&config.bdp_base_url)
    .execute(pool)
    .await
    .expect("configurar BDP");
    fijar_permiso(pool, id, "comandas", true)
        .await
        .expect("permitir comandas");
    (id, config)
}

fn intencion<'a>(venta_id: Uuid, datos: &'a serde_json::Value) -> EscrituraAutorizar<'a> {
    EscrituraAutorizar {
        scope: "create_order",
        target_entity_type: "venta",
        target_entity_id: venta_id,
        entity_json_field: "venta_id",
        datos_enviados: datos,
        snapshot_pre_id: None,
        idempotency_key: None,
    }
}

/// Dos autorizaciones simultáneas de la misma venta: una pasa, la otra no.
#[sqlx::test(migrations = "./migrations")]
async fn dos_autorizaciones_a_la_vez_dejan_una_sola_comanda(pool: PgPool) {
    let (user_id, config) = usuario(&pool).await;
    let venta_id = Uuid::new_v4();
    let datos = serde_json::json!({ "venta_id": venta_id });

    /* Se lanzan las dos antes de esperar ninguna: si el bloqueo dependiera de
     * que la primera termine, el test pasaría igual y no probaría nada. */
    let primera = BdpWriteGuard::authorize(&pool, user_id, &config, intencion(venta_id, &datos));
    let segunda = BdpWriteGuard::authorize(&pool, user_id, &config, intencion(venta_id, &datos));
    let (a, b) = tokio::join!(primera, segunda);

    assert!(
        a.is_ok() ^ b.is_ok(),
        "exactamente una autorización debe pasar; pasaron {} de 2",
        usize::from(a.is_ok()) + usize::from(b.is_ok())
    );
    /* La que falla lo hace por la comanda ya viva, no por un error de base
     * disfrazado: el mensaje del guard es el que ve el operador. */
    let error = a.err().or_else(|| b.err()).expect("una debe fallar");
    assert!(
        error.contains("pendiente o ambigua"),
        "el rechazo debe explicar que hay una operación viva, fue: {error}"
    );

    let vivas: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bdp_audit_log WHERE user_id = $1 AND target_entity_id = $2 \
           AND operacion = 'create_order' AND resultado IN ('pendiente', 'ambiguo')",
    )
    .bind(user_id)
    .bind(venta_id)
    .fetch_one(&pool)
    .await
    .expect("contar auditorías vivas");
    assert_eq!(vivas, 1, "una sola intención de comanda viva por venta");
}

/// El índice parcial sobre la auditoría viva también aguanta a quien no pasa
/// por el guard: sin él, dos INSERT directos coexistirían.
#[sqlx::test(migrations = "./migrations")]
async fn la_base_rechaza_dos_auditorias_vivas_de_la_misma_venta(pool: PgPool) {
    let user_id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(format!("bdp-unica-idx-{user_id}@example.com"))
        .bind("argon2_hash_placeholder")
        .execute(&pool)
        .await
        .expect("crear usuario");
    let venta_id = Uuid::new_v4();

    let insertar = |estado: &'static str| {
        let pool = pool.clone();
        async move {
            sqlx::query(
                "INSERT INTO bdp_audit_log (user_id, operacion, direccion, datos_enviados, \
                   resultado, target_base_url, target_entity_type, target_entity_id) \
                 VALUES ($1, 'create_order', 'glory_to_bdp', '{}'::jsonb, $2, \
                   'http://127.0.0.1:18765', 'venta', $3)",
            )
            .bind(user_id)
            .bind(estado)
            .bind(venta_id)
            .execute(&pool)
            .await
        }
    };

    insertar("pendiente").await.expect("la primera entra");
    let segunda = insertar("ambiguo").await;
    assert!(
        segunda.is_err(),
        "la base debe rechazar dos intenciones vivas de la misma venta"
    );

    /* Cerrada la primera, la segunda ya cabe: el índice es parcial, no un
     * candado permanente sobre la venta. */
    sqlx::query(
        "UPDATE bdp_audit_log SET resultado = 'error' WHERE user_id = $1 AND target_entity_id = $2",
    )
    .bind(user_id)
    .bind(venta_id)
    .execute(&pool)
    .await
    .expect("cerrar la primera");
    insertar("pendiente")
        .await
        .expect("cerrada la anterior, la venta vuelve a admitir un envío");
}

/// `MarketId` + `MarketplaceOrderId` identifican una comanda de BDP: dos filas
/// del mapeo no pueden reclamar el mismo par.
#[sqlx::test(migrations = "./migrations")]
async fn market_id_y_marketplace_order_id_son_unicos(pool: PgPool) {
    let user_id = Uuid::new_v4();
    let insertar = |glory_id: String| {
        let pool = pool.clone();
        async move {
            sqlx::query(
                "INSERT INTO bdp_id_map (id, user_id, entidad, glory_id, bdp_id, \
                   market_id, marketplace_order_id) \
                 VALUES ($1, $2, 'venta', $3, NULL, '9900', 'G0123456789abc')",
            )
            .bind(Uuid::new_v4())
            .bind(user_id)
            .bind(glory_id)
            .execute(&pool)
            .await
        }
    };

    insertar("venta-a".into()).await.expect("la primera entra");
    let segunda = insertar("venta-b".into()).await;
    assert!(
        segunda.is_err(),
        "dos ventas distintas no pueden reclamar la misma comanda de BDP"
    );

    /* Un par distinto sí entra: el índice es del par, no del mercado. */
    sqlx::query(
        "INSERT INTO bdp_id_map (id, user_id, entidad, glory_id, market_id, marketplace_order_id) \
         VALUES ($1, $2, 'venta', 'venta-c', '9900', 'G9999999999xyz')",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("otra comanda del mismo mercado debe entrar");
}
