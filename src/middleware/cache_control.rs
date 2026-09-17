/* [169A-5/D2.3] Respuestas GET cacheables con `Cache-Control` + `ETag`.
 * Clases: `Publica` (igual para todos los usuarios del restaurante: maps,
 * stock, listados base — el `sub` del JWT es el propietario incluso para
 * trabajadores, y el despliegue es single-tenant, así que clavear por URL es
 * seguro; el test negativo D3.3 lo confirma) y `Privada` (dashboard por
 * restaurante: mismo cuerpo para sus usuarios, pero se marca `private` para
 * que ningún borde compartido la sirva a otro inquilino por error).
 * El `Authorization` nunca forma parte de la clave: no se lee aquí.
 * `ETag` débil de proceso (DefaultHasher): si el cliente revalida con
 * `If-None-Match` se responde 304 sin cuerpo. Tras reinicio cambian los
 * ETags — aceptable: el cliente refetch una vez. */

use axum::body::Body;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::errors::AppError;
use crate::AppState;

/// Visibilidad de caché de una respuesta GET lectora.
#[derive(Debug, Clone, Copy)]
pub enum VisibilidadCache {
    /// Igual para todos los usuarios del restaurante; el borde puede compartirla.
    Publica,
    /// Solo para el restaurante autenticado; el borde compartido no la sirve.
    Privada,
}

/// Serializa `cuerpo` a JSON, añade `Cache-Control` + `ETag` y sirve 304 si el
/// cliente ya tiene esta versión (`If-None-Match`).
#[must_use = "la respuesta cacheable debe devolverse al cliente"]
pub fn respuesta_cacheable<T: Serialize>(
    cuerpo: &T,
    visibilidad: VisibilidadCache,
    max_age_secs: u64,
    cabeceras_peticion: &HeaderMap,
) -> Result<Response, AppError> {
    let bytes = serde_json::to_vec(cuerpo)
        .map_err(|e| AppError::Internal(format!("Error serializando respuesta cacheable: {e}")))?;
    respuesta_cacheable_bytes(bytes, visibilidad, max_age_secs, cabeceras_peticion)
}

/// Igual que `respuesta_cacheable` pero parte de bytes ya serializados (vía
/// de la caché de listados: los mismos bytes dan el mismo `ETag`).
#[must_use = "la respuesta cacheable debe devolverse al cliente"]
pub fn respuesta_cacheable_bytes(
    bytes: Vec<u8>,
    visibilidad: VisibilidadCache,
    max_age_secs: u64,
    cabeceras_peticion: &HeaderMap,
) -> Result<Response, AppError> {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let etag = format!("\"{:x}\"", hasher.finish());

    if cabeceras_peticion
        .get(IF_NONE_MATCH)
        .is_some_and(|v| v.as_bytes() == etag.as_bytes())
    {
        return Ok(StatusCode::NOT_MODIFIED.into_response());
    }

    let alcance = match visibilidad {
        VisibilidadCache::Publica => "public",
        VisibilidadCache::Privada => "private",
    };
    let cache_control = format!("{alcance}, max-age={max_age_secs}");
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/json")
        .header(
            CACHE_CONTROL,
            HeaderValue::from_str(&cache_control).map_err(|e| {
                AppError::Internal(format!("Error construyendo Cache-Control: {e}"))
            })?,
        )
        .header(
            ETAG,
            HeaderValue::from_str(&etag)
                .map_err(|e| AppError::Internal(format!("Error construyendo ETag: {e}")))?,
        )
        .body(Body::from(bytes))
        .map_err(|e| AppError::Internal(format!("Error construyendo respuesta cacheable: {e}")))
}

/* [179A-1/F1] Caché en app de listados default. TTL 15 s: un listado de
 * restaurante tolera un cuarto de minuto de dato viejo; a cambio el tráfico
 * lector caliente deja de ejecutar las 2 queries pesadas por petición. */

/// Nombres de endpoint para la clave de `listados_cache`.
pub const ENDPOINT_VENTAS: &str = "ventas";
/// Nombre de endpoint para la clave de `listados_cache`.
pub const ENDPOINT_GASTOS: &str = "gastos";
/// Nombre de endpoint para la clave de `listados_cache`.
pub const ENDPOINT_CLIENTES: &str = "clientes";
/// Nombre de endpoint para la clave de `listados_cache`.
pub const ENDPOINT_RESERVAS: &str = "reservas";

/// TTL de los listados default en memoria.
pub const LISTADOS_TTL: Duration = Duration::from_secs(15);

/// Lee un listado default cacheado. `None` = miss o expirado.
pub async fn leer_listado_cache(
    state: &AppState,
    user_id: Uuid,
    endpoint: &'static str,
) -> Option<Vec<u8>> {
    let cache = state.listados_cache.read().await;
    let (instante, bytes) = cache.get(&(user_id, endpoint))?;
    if instante.elapsed() < LISTADOS_TTL {
        Some(bytes.clone())
    } else {
        None
    }
}

/// Guarda el JSON serializado de un listado default.
pub async fn guardar_listado_cache(
    state: &AppState,
    user_id: Uuid,
    endpoint: &'static str,
    bytes: Vec<u8>,
) {
    state
        .listados_cache
        .write()
        .await
        .insert((user_id, endpoint), (Instant::now(), bytes));
}

/// Invalida el listado default cacheado del restaurante tras una escritura.
/// Llamar en todos los handlers que escriban en la tabla del endpoint; el TTL
/// cubre escrituras de fondo (sync BDP/Haddock), igual que en D2.
pub async fn invalidar_listado(state: &AppState, user_id: Uuid, endpoint: &'static str) {
    state
        .listados_cache
        .write()
        .await
        .retain(|clave, _| clave != &(user_id, endpoint));
}
