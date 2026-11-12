// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
/* [M21] Cadencia de la cola cuando BDP no está.
 *
 * Antes el tope de reintentos era un abandono: cinco barridos de 60 s con el
 * TPV apagado dejaban las filas muertas para siempre, y el fallo de `Login`
 * (que no envía nada) también gastaba un intento. Ahora:
 *   - con el TPV caído la pasada termina sin tocar ninguna fila;
 *   - agotada la serie, la fila descansa y después se reabre sola.
 * Lo que se prueba aquí es exactamente eso: los intentos no se queman por un
 * TPV apagado y la fila vuelve a estar disponible sin intervención. */

use glory_backend::services::bdp_push::{
    BdpPushService, DOMINIO_ARTICULO, ESTADO_PENDIENTE, OPERACION_CREAR, REINTENTOS_MAX,
};
use glory_backend::services::{fijar_permiso, BdpPushFlushService, ConfiguracionService};
use sqlx::PgPool;
use uuid::Uuid;

/* Cuenta en `automatic` con la categoría de artículos encendida: aunque hubiera
 * TPV, nada más frenaría el envío. `127.0.0.1:59999` no escucha: `Health` falla
 * por red, que es el «TPV apagado» del caso real. */
async fn usuario_con_tpv_apagado(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(format!("m21-{user_id}@example.com"))
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("crear usuario de prueba");
    ConfiguracionService::obtener(pool, user_id)
        .await
        .expect("crear configuración");
    sqlx::query(
        "UPDATE configuracion_restaurante \
         SET modo_operacion = 'bdp', bdp_sync_enabled = TRUE, \
             bdp_base_url = 'http://127.0.0.1:59999', \
             bdp_login = 'm21', bdp_password = 'm21', bdp_integrator_code = 'M21', \
             bdp_pos_id = 1, bdp_employee_id = 1, bdp_items_profile_id = 1, \
             bdp_sync_mode = 'automatic', push_modalidad = 'automatico', \
             bdp_auto_backup_before_write = TRUE \
         WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(pool)
    .await
    .expect("configurar TPV inalcanzable");
    fijar_permiso(pool, user_id, "articulos_clientes", true)
        .await
        .expect("encender la categoría de artículos");
    user_id
}

async fn encolar_fila(pool: &PgPool, user_id: Uuid, entidad: &str) {
    BdpPushService::encolar(
        pool,
        user_id,
        DOMINIO_ARTICULO,
        entidad,
        OPERACION_CREAR,
        &serde_json::json!({
            "AutomaticCode": false,
            "ArticleData": { "ArtCode": 90_000_123, "ArtDescription": "Plato" },
            "AllProfiles": true
        }),
    )
    .await
    .expect("encolar fila")
}

async fn fila(
    pool: &PgPool,
    user_id: Uuid,
) -> (String, i32, Option<chrono::DateTime<chrono::Utc>>) {
    sqlx::query_as(
        "SELECT estado, reintentos, proximo_intento_at FROM bdp_push_pendientes \
         WHERE user_id = $1 AND dominio = $2",
    )
    .bind(user_id)
    .bind(DOMINIO_ARTICULO)
    .fetch_one(pool)
    .await
    .expect("leer la fila")
}

/// Un TPV apagado no gasta intentos: la pasada termina en espera con las filas
/// intactas, y el envío sigue pendiente para cuando vuelva.
#[sqlx::test(migrations = "./migrations")]
async fn con_el_tpv_apagado_no_se_queman_reintentos(pool: PgPool) {
    let user_id = usuario_con_tpv_apagado(&pool).await;
    encolar_fila(&pool, user_id, "90000123").await;

    let resumen = BdpPushFlushService::flush(&pool, user_id, false)
        .await
        .expect("flush con el TPV apagado no debe fallar");
    assert_eq!(resumen.bdp_no_disponible, 1, "se detecta el TPV apagado");
    assert_eq!(resumen.procesados, 0, "no se envió nada");
    assert_eq!(resumen.errores, 0, "no es un error de la fila");

    let (estado, reintentos, _) = fila(&pool, user_id).await;
    assert_eq!(estado, ESTADO_PENDIENTE, "sigue pendiente de enviar");
    assert_eq!(reintentos, 0, "un TPV apagado no es un intento fallido");
}

/// Agotada la serie, la fila descansa en vez de morir; y pasada la espera la
/// serie se reabre sola. Ese es el punto: un TPV apagado un fin de semana no
/// deja la cola muerta.
#[sqlx::test(migrations = "./migrations")]
async fn agotada_la_serie_la_fila_espera_y_se_reabre(pool: PgPool) {
    let user_id = usuario_con_tpv_apagado(&pool).await;
    encolar_fila(&pool, user_id, "90000123").await;
    sqlx::query(
        "UPDATE bdp_push_pendientes \
         SET reintentos = $2, proximo_intento_at = NOW() + INTERVAL '90 seconds' \
         WHERE user_id = $1",
    )
    .bind(user_id)
    .bind(REINTENTOS_MAX)
    .execute(&pool)
    .await
    .expect("dejar la serie agotada con su espera");

    /* Dentro de la espera no se toca: ni se reabre ni se envía. */
    let resumen = BdpPushFlushService::flush(&pool, user_id, false)
        .await
        .expect("flush en espera");
    assert_eq!(resumen.en_espera, 1);
    assert_eq!(resumen.procesados, 0);
    let (_, reintentos, _) = fila(&pool, user_id).await;
    assert_eq!(reintentos, REINTENTOS_MAX, "la espera no consume intentos");

    /* Cumplida la espera, la serie vuelve a empezar. */
    sqlx::query(
        "UPDATE bdp_push_pendientes SET proximo_intento_at = NOW() - INTERVAL '1 second' \
         WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("vencer la espera");
    let resumen = BdpPushFlushService::flush(&pool, user_id, false)
        .await
        .expect("flush tras la espera");
    assert_eq!(resumen.en_espera, 0, "ya no espera");
    assert_eq!(resumen.bdp_no_disponible, 1, "el TPV sigue apagado");
    let (estado, reintentos, _) = fila(&pool, user_id).await;
    assert_eq!(reintentos, 0, "la fila vuelve a tener sus intentos");
    assert_eq!(estado, ESTADO_PENDIENTE);
}

/// El botón manual es el respaldo: no espera a la cadencia. Con la serie
/// agotada y el TPV apagado, el envío forzado sí lo intenta (y solo ahí se
/// gasta un intento, porque el TPV tampoco contesta).
#[sqlx::test(migrations = "./migrations")]
async fn el_envio_manual_no_espera_la_cadencia(pool: PgPool) {
    let user_id = usuario_con_tpv_apagado(&pool).await;
    encolar_fila(&pool, user_id, "90000123").await;
    sqlx::query(
        "UPDATE bdp_push_pendientes \
         SET reintentos = $2, proximo_intento_at = NOW() + INTERVAL '3600 seconds' \
         WHERE user_id = $1",
    )
    .bind(user_id)
    .bind(REINTENTOS_MAX)
    .execute(&pool)
    .await
    .expect("dejar la serie agotada con su espera");

    let resumen = BdpPushFlushService::flush(&pool, user_id, true)
        .await
        .expect("flush manual");
    assert_eq!(resumen.en_espera, 0, "el manual no respeta la espera");
    assert_eq!(
        resumen.procesados, 1,
        "el manual sí intenta el envío aunque el TPV no esté"
    );
}
