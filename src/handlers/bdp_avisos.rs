/* [199A-1/F1] Receptor de avisos de acciones del TPV (BDP -> Glory) y su buzón.
 *
 *   Público (lo llama el TPV, sin sesión):
 *     GET|POST /api/bdp/avisos/:accion   — una de las siete acciones; el TPV
 *         configura aquí su `Ruta Base` + `Ruta Acción` y elige GET o POST.
 *
 *   Autenticado (lo mira el restaurante desde la web):
 *     GET  /api/bdp/avisos               — listar el buzón
 *     POST /api/bdp/avisos/:id/reintentar — reabrir un aviso para revisión
 *
 * El receptor público no usa sesión porque el TPV no puede autenticarse con
 * JWT: se protege con el secreto compartido `BDP_AVISOS_SECRETO` (parámetro
 * `Secreto` del aviso, o cabecera `X-BDP-Secreto`), con límite de tamaño y con
 * límite de frecuencia. Sin secreto configurado no se admite nada: es preferible
 * que el TPV vea un error a que cualquiera en la red llene el buzón.
 *
 * Cualquier fallo se contesta en JSON y sin tocar el buzón salvo el 409 de
 * conflicto, que deja intacto el aviso original. */

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::UserRole;
use crate::services::bdp_avisos::{
    self, AccionAviso, AvisoPreparado, CamposAviso, RechazoAviso, ResultadoInbox,
};
use crate::services::{BdpAvisosService, FilaAviso};
use crate::AppState;

/// Cuerpo máximo de un aviso: son eventos de comanda, no ficheros.
const MAX_AVISO_BYTES: usize = 16 * 1024;

/// Límite de frecuencia del receptor público, por ventana fija y global. El TPV
/// puede mandar ráfagas (traspasos, facturación), así que es holgado: corta el
/// abuso, no el uso normal.
const VENTANA: Duration = Duration::from_secs(60);
const MAX_PETICIONES_POR_VENTANA: u32 = 120;

/// Rutas con sesión: mirar el buzón y reabrir avisos.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/bdp/avisos", get(listar))
        // sentinel-disable-next-line ruta-post-sin-rate-limit: la regla no ve el limitador propio de este mismo archivo (ventana fija, 120/min, en `dentro_del_limite`). Además esta ruta pide sesión de Admin; el limitador está en el receptor público, que es el que no tiene sesión.
        .route("/bdp/avisos/:id/reintentar", post(reintentar))
}

/// Rutas del receptor: las llama el TPV, sin sesión. Se aíslan en su propio
/// router para que lleven su propio límite de tamaño, más bajo que el general.
pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/bdp/avisos/:accion",
            get(recibir_aviso).post(recibir_aviso),
        )
        .layer(tower_http::limit::RequestBodyLimitLayer::new(MAX_AVISO_BYTES))
}

/* --------------------------------------------------------------------------
 * Receptor público
 * -------------------------------------------------------------------------- */

/// Lo que se le contesta al TPV. El distribuidor no documenta ninguna respuesta
/// esperada, así que se contesta corto y en JSON por si algún día lo mira alguien.
#[derive(Debug, Serialize)]
pub struct RespuestaAviso {
    pub estado: &'static str,
    pub accion: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nota: Option<String>,
}

/// Cuenta de peticiones de la ventana en curso. El restaurante es uno y el
/// proceso también, así que un contador global basta; no hay estado por IP
/// porque el TPV sale siempre por la misma.
struct VentanaFija {
    inicio: Instant,
    cuenta: u32,
}

static CONTADOR: OnceLock<Mutex<VentanaFija>> = OnceLock::new();

fn dentro_del_limite() -> bool {
    let mut ventana = CONTADOR
        .get_or_init(|| {
            Mutex::new(VentanaFija {
                inicio: Instant::now(),
                cuenta: 0,
            })
        })
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    if ventana.inicio.elapsed() >= VENTANA {
        *ventana = VentanaFija {
            inicio: Instant::now(),
            cuenta: 1,
        };
        return true;
    }

    ventana.cuenta += 1;
    ventana.cuenta <= MAX_PETICIONES_POR_VENTANA
}

/// De dónde viene el aviso, tal como lo ve el servidor. Detrás de un túnel o un
/// proxy la IP real es la primera de `X-Forwarded-For`; si no hay ninguna
/// cabecera, se guarda sin origen en vez de inventarlo.
fn origen_de(headers: &HeaderMap) -> Option<String> {
    for cabecera in ["x-forwarded-for", "x-real-ip"] {
        if let Some(valor) = headers.get(cabecera).and_then(|valor| valor.to_str().ok()) {
            let primero = valor.split(',').next().unwrap_or_default().trim();
            if !primero.is_empty() {
                return Some(primero.to_string());
            }
        }
    }
    None
}

/// Secreto presentado por el TPV: cabecera propia, `Authorization: Bearer` o el
/// parámetro del aviso.
fn secreto_recibido(headers: &HeaderMap, campos: &CamposAviso) -> Option<String> {
    for cabecera in ["x-bdp-secreto", "x-bdp-secret"] {
        if let Some(valor) = headers.get(cabecera).and_then(|valor| valor.to_str().ok()) {
            let valor = valor.trim();
            if !valor.is_empty() {
                return Some(valor.to_string());
            }
        }
    }
    if let Some(valor) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        if let Some(resto) = valor.strip_prefix("Bearer ") {
            let resto = resto.trim();
            if !resto.is_empty() {
                return Some(resto.to_string());
            }
        }
    }
    campos.secreto().map(str::to_string)
}

/// Convierte la query string en un objeto JSON: es el payload que se guarda
/// cuando el aviso llega por GET y no hay cuerpo.
fn cuerpo_desde_query(query: &BTreeMap<String, String>) -> Value {
    Value::Object(
        query
            .iter()
            .map(|(clave, valor)| (clave.clone(), json!(valor)))
            .collect(),
    )
}

pub async fn recibir_aviso(
    state: State<AppState>,
    accion: Path<String>,
    query: Query<BTreeMap<String, String>>,
    headers: HeaderMap,
    cuerpo: Bytes,
) -> Result<(StatusCode, Json<RespuestaAviso>), AppError> {
    recibir_aviso_con_secreto(
        state,
        accion,
        query,
        headers,
        cuerpo,
        bdp_avisos::secreto_esperado(),
    )
    .await
}

/// El receptor, con el secreto esperado ya resuelto. Se separa del anterior para
/// poder probar los dos casos (con secreto y sin él) sin tocar el entorno del
/// proceso: `esperado = None` significa receptor sin configurar.
pub async fn recibir_aviso_con_secreto(
    State(state): State<AppState>,
    Path(accion): Path<String>,
    Query(query): Query<BTreeMap<String, String>>,
    headers: HeaderMap,
    cuerpo: Bytes,
    esperado: Option<String>,
) -> Result<(StatusCode, Json<RespuestaAviso>), AppError> {
    if !dentro_del_limite() {
        tracing::warn!("[avisos-bdp] límite de frecuencia alcanzado; se rechaza el aviso");
        return Err(AppError::TooManyRequests(
            "Demasiados avisos seguidos. Reintenta en un minuto.".to_string(),
        ));
    }

    let accion = AccionAviso::desde_slug(&accion).ok_or_else(|| {
        AppError::NotFound(format!(
            "Acción de aviso desconocida: {}. Se admite crear, modificar, cambiar-estado, \
             borrar, finalizar, traspasar y traspasar-linea.",
            accion.trim()
        ))
    })?;

    /* El cuerpo vacío es legítimo: el TPV puede mandar todo por query string. */
    let cuerpo_json: Value = if cuerpo.is_empty() {
        Value::Object(serde_json::Map::new())
    } else {
        serde_json::from_slice(&cuerpo).map_err(|error| {
            AppError::BadRequest(format!(
                "El aviso no es JSON válido ({error}). Se admite un objeto JSON o los \
                 parámetros en la query string."
            ))
        })?
    };
    if !cuerpo_json.is_object() && !cuerpo_json.is_null() {
        return Err(AppError::BadRequest(
            "El aviso debe ser un objeto JSON, no una lista ni un valor suelto.".to_string(),
        ));
    }

    let campos = CamposAviso::desde_query(&query).fusionar(&CamposAviso::desde_json(&cuerpo_json));

    /* Secreto compartido. Sin secreto configurado el receptor está apagado. */
    let esperado = esperado.ok_or_else(|| {
        AppError::ServicioNoDisponible(
            "El receptor de avisos no tiene secreto configurado (BDP_AVISOS_SECRETO).".to_string(),
        )
    })?;
    let recibido = secreto_recibido(&headers, &campos).unwrap_or_default();
    if !bdp_avisos::secreto_valido(&esperado, &recibido) {
        tracing::warn!(
            "[avisos-bdp] aviso rechazado por secreto inválido (acción {})",
            accion.slug()
        );
        return Err(AppError::Unauthorized);
    }

    /* El payload guardado es el cuerpo si lo hay; si no, la propia query. */
    let cuerpo_trae_datos = cuerpo_json
        .as_object()
        .is_some_and(|objeto| !objeto.is_empty());
    let payload_crudo = if cuerpo_trae_datos {
        cuerpo_json
    } else {
        cuerpo_desde_query(&query)
    };

    let aviso = bdp_avisos::preparar(accion, &campos, &payload_crudo).map_err(|rechazo| match rechazo
    {
        RechazoAviso::AccionDesconocida(texto) => AppError::NotFound(texto),
        RechazoAviso::Invalido(motivo) => AppError::Validation(motivo),
    })?;

    let user_id = BdpAvisosService::resolver_usuario(&state.pool)
        .await
        .map_err(AppError::ServicioNoDisponible)?;

    let resultado = BdpAvisosService::recibir(
        &state.pool,
        user_id,
        origen_de(&headers).as_deref(),
        &aviso,
    )
    .await
    .map_err(AppError::Internal)?;

    respuesta_del_buzon(resultado, accion, &aviso)
}

/// Traduce lo que ha pasado en el buzón a la respuesta que ve el TPV. En el log
/// va la clave de idempotencia, no el contenido: el payload puede traer datos de
/// cliente y, aunque está censurado, no pinta nada en un log.
fn respuesta_del_buzon(
    resultado: ResultadoInbox,
    accion: AccionAviso,
    aviso: &AvisoPreparado,
) -> Result<(StatusCode, Json<RespuestaAviso>), AppError> {
    match resultado {
        ResultadoInbox::Nuevo(id) => {
            tracing::info!(
                "[avisos-bdp] {} recibido (clave {}), estado {}",
                accion.slug(),
                aviso.clave_idempotencia,
                aviso.estado
            );
            Ok((
                StatusCode::ACCEPTED,
                Json(RespuestaAviso {
                    estado: aviso.estado,
                    accion: accion.slug(),
                    id: Some(id),
                    nota: aviso.nota.clone(),
                }),
            ))
        }
        ResultadoInbox::Duplicado => {
            tracing::info!(
                "[avisos-bdp] {} repetido (clave {}), se ignora",
                accion.slug(),
                aviso.clave_idempotencia
            );
            Ok((
                StatusCode::OK,
                Json(RespuestaAviso {
                    estado: "duplicado",
                    accion: accion.slug(),
                    id: None,
                    nota: None,
                }),
            ))
        }
        ResultadoInbox::Conflicto => {
            tracing::warn!(
                "[avisos-bdp] {} con la misma clave y otro contenido (clave {}); se conserva el original",
                accion.slug(),
                aviso.clave_idempotencia
            );
            Err(AppError::Conflict(
                "Ya hay un aviso con esa acción, comanda y marca de tiempo, pero con otro \
                 contenido. Se conserva el primero."
                    .to_string(),
            ))
        }
    }
}

/* --------------------------------------------------------------------------
 * Buzón (con sesión)
 * -------------------------------------------------------------------------- */

#[derive(Debug, Default, Deserialize)]
pub struct ListarQuery {
    /// `recibido`, `requiere_revision`, `procesado`, `rechazado`, `procesando`.
    pub estado: Option<String>,
    /// Cuántos avisos devolver (1-200; por defecto 50).
    pub limite: Option<i64>,
}

const ESTADOS_VALIDOS: [&str; 5] = [
    "recibido",
    "procesando",
    "procesado",
    "rechazado",
    "requiere_revision",
];

pub async fn listar(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListarQuery>,
) -> Result<Json<Vec<FilaAviso>>, AppError> {
    auth.require_role(&[UserRole::Admin])?;

    let estado = match query.estado.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(estado) => {
            if !ESTADOS_VALIDOS.contains(&estado) {
                return Err(AppError::BadRequest(format!(
                    "Estado desconocido: {estado}. Se admite {}.",
                    ESTADOS_VALIDOS.join(", ")
                )));
            }
            Some(estado)
        }
    };

    let avisos = BdpAvisosService::listar(
        &state.pool,
        auth.user_id,
        estado,
        query.limite.unwrap_or(50),
    )
    .await
    .map_err(AppError::Internal)?;

    Ok(Json(avisos))
}

pub async fn reintentar(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    auth.require_role(&[UserRole::Admin])?;

    let reabierto = BdpAvisosService::reintentar(&state.pool, auth.user_id, id)
        .await
        .map_err(AppError::Internal)?;

    if !reabierto {
        return Err(AppError::NotFound(
            "No hay ningún aviso pendiente de revisión con ese identificador.".to_string(),
        ));
    }

    Ok(Json(json!({"id": id, "estado": "recibido"})))
}
