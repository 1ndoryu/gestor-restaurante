// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
/* [D12c] Movimientos de puntos sobre la misma cola: son deltas y se suman.
 *   - dos movimientos pendientes se acumulan en una sola fila (+10 y +5 = 15);
 *   - un movimiento que anula el pendiente deja la fila `descartado` (neto 0);
 *   - tras `sincronizado`, un nuevo movimiento crea fila nueva (no la reescribe). */

use glory_backend::repositories::ConfiguracionRepository;
use glory_backend::services::{payload_puntos, BdpPushService};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

async fn crear_usuario(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(format!("puntos-{id}@example.com"))
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("crear usuario de prueba");
    ConfiguracionRepository::obtener_o_crear(pool, id)
        .await
        .expect("config por defecto");
    id
}

async fn encolar_puntos(pool: &PgPool, user_id: Uuid, cliente: &str, puntos: i64) {
    let delta = Decimal::from(puntos);
    let payload = payload_puntos(900_001, delta, "prueba").expect("payload de puntos");
    BdpPushService::acumular_puntos(pool, user_id, cliente, delta, &payload)
        .await
        .expect("acumular puntos");
}

/// Filas de puntos del cliente como (estado, PointsAdded), en orden de creación.
async fn filas_puntos(pool: &PgPool, user_id: Uuid, cliente: &str) -> Vec<(String, String)> {
    sqlx::query_as(
        "SELECT estado, payload_json->>'PointsAdded' FROM bdp_push_pendientes \
         WHERE user_id = $1 AND dominio = 'cliente_puntos' AND entidad_id = $2 \
         ORDER BY created_at",
    )
    .bind(user_id)
    .bind(cliente)
    .fetch_all(pool)
    .await
    .expect("leer filas de puntos")
}

#[sqlx::test(migrations = "./migrations")]
async fn dos_movimientos_pendientes_se_suman(pool: PgPool) {
    let user_id = crear_usuario(&pool).await;
    encolar_puntos(&pool, user_id, "cli-1", 10).await;
    encolar_puntos(&pool, user_id, "cli-1", 5).await;

    let filas = filas_puntos(&pool, user_id, "cli-1").await;
    assert_eq!(filas.len(), 1, "una sola fila activa, no dos que se pisan");
    assert_eq!(filas[0].0, "pendiente");
    assert_eq!(filas[0].1, "15");
}

#[sqlx::test(migrations = "./migrations")]
async fn movimiento_que_anula_el_pendiente_descarta_la_fila(pool: PgPool) {
    let user_id = crear_usuario(&pool).await;
    encolar_puntos(&pool, user_id, "cli-2", 10).await;
    encolar_puntos(&pool, user_id, "cli-2", -10).await;

    let filas = filas_puntos(&pool, user_id, "cli-2").await;
    assert_eq!(filas.len(), 1);
    assert_eq!(filas[0].0, "descartado", "neto 0 no se envía nada");
}

#[sqlx::test(migrations = "./migrations")]
async fn tras_sincronizar_un_nuevo_movimiento_crea_fila_nueva(pool: PgPool) {
    let user_id = crear_usuario(&pool).await;
    encolar_puntos(&pool, user_id, "cli-3", 10).await;
    sqlx::query(
        "UPDATE bdp_push_pendientes SET estado = 'sincronizado' \
         WHERE user_id = $1 AND entidad_id = 'cli-3'",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("marcar sincronizado");
    encolar_puntos(&pool, user_id, "cli-3", 4).await;

    let filas = filas_puntos(&pool, user_id, "cli-3").await;
    assert_eq!(filas.len(), 2, "la fila enviada no se reescribe");
    assert_eq!(filas[0], ("sincronizado".to_string(), "10".to_string()));
    assert_eq!(filas[1], ("pendiente".to_string(), "4".to_string()));
}
