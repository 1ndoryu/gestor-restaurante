//! [D24-auto-2] Lectura programada de los tres maestros que seguían sin bucle:
//! clientes, plano de sala y departamentos.
//!
//! El alcance de la fase D promete que estas tres tablas viajan solas. Los
//! artículos ya tenían su cadencia (`bdp_maestros_auto`); estas tres se
//! refrescaban solo pulsando un botón. El barrido decide *cuándo*; el camino de
//! lectura es el mismo del botón, así que el upsert y sus protecciones no se
//! duplican.
//!
//! Las tres son **solo lectura sobre el TPV**: nunca escriben en BDP, así que no
//! piden armado. Solo respetan el modo efectivo (en standalone no se llama).

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::ConfiguracionRestaurante;
use crate::services::{
    BdpDepartmentsExportFromProfileRequest, BdpWeblinkClient, ConfiguracionService, ModoEfectivo,
    ServicioModoOperacion,
};

/// La cadencia recomendada es la misma que la de artículos: si se encienden
/// todas, el restaurante ve los maestros con el mismo retraso.
pub const SEGUNDOS_RECOMENDADOS: i32 = 900;

/// Grano del bucle. Igual que en artículos: no es la cadencia de lectura, es
/// cada cuánto se comprueba qué cuenta tiene el turno vencido. Cuesta una
/// consulta por pasada y no llama a BDP.
pub const GRANO_BUCLE_SEGUNDOS: u64 = 60;

/// Qué maestro se lee. Cada variante sabe su columna de cadencia y la marca de
/// última lectura, para que las tres no repitan el mismo SQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Maestro {
    Clientes,
    Plano,
    Departamentos,
}

impl Maestro {
    /// Las columnas no son datos de usuario: son nombres fijos de esta misma
    /// tabla, escritos aquí y nunca interpolados desde fuera.
    #[must_use]
    pub const fn nombre(self) -> &'static str {
        match self {
            Self::Clientes => "clientes",
            Self::Plano => "plano",
            Self::Departamentos => "departamentos",
        }
    }

    fn columna_cadencia(self) -> &'static str {
        match self {
            Self::Clientes => "bdp_clientes_auto_segundos",
            Self::Plano => "bdp_plano_auto_segundos",
            Self::Departamentos => "bdp_departamentos_auto_segundos",
        }
    }

    fn columna_ultima(self) -> &'static str {
        match self {
            Self::Clientes => "bdp_clientes_ultima_lectura",
            Self::Plano => "bdp_plano_ultima_lectura",
            Self::Departamentos => "bdp_departamentos_ultima_lectura",
        }
    }
}

/// Reclama el turno de una cuenta en la misma sentencia que lo comprueba: si dos
/// instancias coinciden, solo una recibe `true` y la otra pasa de largo. La marca
/// se pone antes de leer a propósito: si la lectura falla, se espera una cadencia
/// entera en vez de martillear el TPV con el mismo error.
async fn reclamar(pool: &PgPool, user_id: Uuid, maestro: Maestro) -> Result<bool, sqlx::Error> {
    let sql = format!(
        "UPDATE configuracion_restaurante \
         SET {ultima} = NOW() \
         WHERE user_id = $1 AND {cadencia} > 0 \
           AND ({ultima} IS NULL \
                OR {ultima} <= NOW() - {cadencia} * INTERVAL '1 second')",
        ultima = maestro.columna_ultima(),
        cadencia = maestro.columna_cadencia(),
    );
    sqlx::query(&sql)
        .bind(user_id)
        .execute(pool)
        .await
        .map(|resultado| resultado.rows_affected() > 0)
}

/// Cuentas con esa cadencia encendida.
async fn cuentas(pool: &PgPool, maestro: Maestro) -> Result<Vec<Uuid>, sqlx::Error> {
    let sql = format!(
        "SELECT user_id FROM configuracion_restaurante WHERE {} > 0",
        maestro.columna_cadencia()
    );
    sqlx::query_scalar(&sql).fetch_all(pool).await
}

/// Lee un maestro para una cuenta ya reclamada. Devuelve una descripción corta
/// de lo que trajo, o el error para el registro.
async fn leer(
    pool: &PgPool,
    user_id: Uuid,
    config: &ConfiguracionRestaurante,
    maestro: Maestro,
) -> Result<String, String> {
    let client = BdpWeblinkClient::new(config);
    client
        .login()
        .await
        .map_err(|error| format!("login: {error}"))?;

    match maestro {
        Maestro::Clientes => {
            let resultado =
                crate::handlers::importar_clientes(pool, user_id, true).await
                    .map_err(|error| error.to_string())?;
            let importados = resultado.get("imported").and_then(serde_json::Value::as_u64).unwrap_or(0);
            let actualizados = resultado.get("updated").and_then(serde_json::Value::as_u64).unwrap_or(0);
            let conflictos = resultado.get("conflicts").and_then(serde_json::Value::as_u64).unwrap_or(0);
            Ok(format!("{importados} nuevos, {actualizados} vinculados, {conflictos} conflictos"))
        }
        Maestro::Plano => {
            let resultado = crate::services::BdpSyncService::sync_tables(
                &client, pool, user_id, true,
            )
            .await?;
            Ok(format!(
                "{} salones, {} zonas y {} mesas nuevas",
                resultado.salones_bdp, resultado.zonas_creadas, resultado.mesas_creadas
            ))
        }
        Maestro::Departamentos => {
            let valor = client
                .export_departments_from_profile(
                    &BdpDepartmentsExportFromProfileRequest {
                        profile_id: config.bdp_items_profile_id,
                    },
                )
                .await
                .map_err(|error| error.to_string())?;
            let items = crate::services::aplanar_departamentos(&valor);
            let resultado = crate::services::BdpImportDepartamentosService::importar(
                pool,
                user_id,
                &items,
                crate::models::TIPO_DEPARTAMENTO,
            )
            .await?;
            Ok(format!(
                "{} nuevos, {} vinculados, {} conflictos",
                resultado.nuevos, resultado.vinculados, resultado.conflictos_nombre
            ))
        }
    }
}

/// Pasa por las cuentas vencidas de un maestro. Un fallo de una cuenta se
/// registra y no corta a las demás. Devuelve cuántas cuentas se leyeron.
pub async fn sincronizar_vencidas(
    pool: &PgPool,
    modo: &ServicioModoOperacion,
    maestro: Maestro,
) -> usize {
    let usuarios = match cuentas(pool, maestro).await {
        Ok(usuarios) => usuarios,
        Err(error) => {
            tracing::warn!(
                "[D24-auto-2] No se pudieron listar cuentas para {}: {error}",
                maestro.nombre()
            );
            return 0;
        }
    };

    let mut leidas = 0;
    for user_id in usuarios {
        /* En standalone no hay TPV al que preguntar. */
        if !matches!(
            modo.modo_efectivo(pool, user_id).await,
            Ok(ModoEfectivo::Bdp)
        ) {
            continue;
        }
        match reclamar(pool, user_id, maestro).await {
            Ok(true) => {}
            Ok(false) => continue,
            Err(error) => {
                tracing::warn!(
                    "[D24-auto-2] Turno de {} no reclamado para {user_id}: {error}",
                    maestro.nombre()
                );
                continue;
            }
        }
        let Ok(config) = ConfiguracionService::obtener(pool, user_id).await else {
            continue;
        };
        if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
            continue;
        }
        match leer(pool, user_id, &config, maestro).await {
            Ok(resumen) => {
                leidas += 1;
                tracing::info!("[D24-auto-2] {} de {user_id}: {resumen}", maestro.nombre());
            }
            Err(error) => tracing::warn!(
                "[D24-auto-2] Lectura de {} fallida para {user_id}: {error}",
                maestro.nombre()
            ),
        }
    }
    leidas
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Los nombres de columna son la parte que no puede fallar en silencio: un
    /// nombre mal escrito da un error de SQL en cada pasada del bucle.
    #[test]
    fn cada_maestro_declara_sus_columnas() {
        for maestro in [Maestro::Clientes, Maestro::Plano, Maestro::Departamentos] {
            assert!(maestro.columna_cadencia().starts_with("bdp_"));
            assert!(maestro.columna_cadencia().ends_with("_auto_segundos"));
            assert!(maestro.columna_ultima().starts_with("bdp_"));
            assert!(maestro.columna_ultima().ends_with("_ultima_lectura"));
            /* Las columnas llevan el nombre del maestro: así una variante nueva
             * copiada sin cambiar las columnas se detecta aquí. */
            assert!(maestro.columna_cadencia().contains(maestro.nombre()));
        }
    }
}
