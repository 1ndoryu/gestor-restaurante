//! Lectura programada del catálogo de maestros del BDP (artículos, precios y
//! stock agregado).
//!
//! [D24-auto] Antes solo se leía pulsando «Importar del BDP»; ahora cada cuenta
//! puede encender una cadencia propia. El barrido solo decide *cuándo*: el
//! camino de lectura es el mismo `sync_catalog` del botón, así el upsert, los
//! perfiles y la contabilidad de cambios no se duplican.

use sqlx::PgPool;
use uuid::Uuid;

use crate::services::{
    BdpSyncService, BdpWeblinkClient, ConfiguracionService, ModoEfectivo, ServicioModoOperacion,
};

/// Cadencia por defecto recomendada al encenderla (15 min).
pub const SEGUNDOS_RECOMENDADOS: i32 = 900;

/// Cada cuánto mira el bucle qué cuenta tiene su lectura vencida. No es la
/// cadencia de lectura: es el grano con el que se comprueban las cadencias de
/// cada cuenta y cuesta una consulta por pasada, sin llamar a BDP.
pub const GRANO_BUCLE_SEGUNDOS: u64 = 60;

/// Una cuenta está vencida si nunca leyó o si pasó su cadencia. El `UPDATE`
/// reclama el turno en la misma sentencia para que dos instancias no lean a la
/// vez; si no reclama, otra ya lo hizo.
async fn reclamar(pool: &PgPool, user_id: Uuid) -> Result<bool, String> {
    sqlx::query(
        "UPDATE configuracion_restaurante \
         SET bdp_maestros_ultima_lectura = NOW() \
         WHERE user_id = $1 AND bdp_maestros_auto_segundos > 0 \
           AND (bdp_maestros_ultima_lectura IS NULL \
                OR bdp_maestros_ultima_lectura \
                   <= NOW() - bdp_maestros_auto_segundos * INTERVAL '1 second')",
    )
    .bind(user_id)
    .execute(pool)
    .await
    .map(|resultado| resultado.rows_affected() > 0)
    .map_err(|error| format!("no se pudo reclamar la lectura: {error}"))
}

/// Lee el catálogo de las cuentas cuyo turno ha vencido. Un fallo de una cuenta
/// se registra y no corta a las demás. Devuelve los artículos creados o
/// actualizados en esta pasada.
pub async fn sincronizar_vencidas(pool: &PgPool, modo: &ServicioModoOperacion) -> usize {
    let usuarios: Vec<Uuid> = match sqlx::query_scalar(
        "SELECT user_id FROM configuracion_restaurante WHERE bdp_maestros_auto_segundos > 0",
    )
    .fetch_all(pool)
    .await
    {
        Ok(usuarios) => usuarios,
        Err(error) => {
            tracing::warn!("[D24-auto] No se pudieron listar cuentas para maestros: {error}");
            return 0;
        }
    };

    let mut total = 0;
    for user_id in usuarios {
        if !matches!(
            modo.modo_efectivo(pool, user_id).await,
            Ok(ModoEfectivo::Bdp)
        ) {
            continue;
        }
        /* El turno se reclama antes de leer: si dos instancias coinciden, solo
         * una llama a BDP y la otra pasa de largo. */
        match reclamar(pool, user_id).await {
            Ok(true) => {}
            Ok(false) => continue,
            Err(error) => {
                tracing::warn!("[D24-auto] Lectura de {user_id} no reclamada: {error}");
                continue;
            }
        }
        let Ok(config) = ConfiguracionService::obtener(pool, user_id).await else {
            continue;
        };
        if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
            continue;
        }
        let client = BdpWeblinkClient::new(&config);
        if let Err(error) = client.login().await {
            tracing::warn!("[D24-auto] Login BDP fallido para {user_id}: {error}");
            continue;
        }
        match BdpSyncService::sync_catalog(&client, pool, user_id, config.bdp_catalog_price_type)
            .await
        {
            Ok(resultado) => {
                total += usize::try_from(resultado.actualizados + resultado.creados).unwrap_or(0);
            }
            Err(error) => tracing::warn!("[D24-auto] Lectura de maestros fallida: {error}"),
        }
    }
    total
}
