/* [D28-auto-R] Documentos de venta exportados desde el BDP.
 *
 *   GET  /api/bdp/documentos           — últimos documentos guardados
 *   POST /api/bdp/documentos/sincronizar — lee la ventana pendiente del perfil
 *
 * Por qué existe: el servicio y la tabla llevaban desde D28 sin ninguna ruta ni
 * pantalla, y su disparo automático solo se podía encender con variables de
 * entorno. La web no podía ni ver los documentos ni forzar una lectura.
 *
 * Ambas rutas exigen Admin: el documento de venta es dato fiscal y la lectura
 * toca la conexión con el TPV. Ninguna escribe en BDP. */

use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::UserRole;
use crate::repositories::{BdpDocumentosVentaRepository, DocumentoVentaGuardado};
use crate::services::bdp_documentos_venta::{self, ResultadoDocumentos};
use crate::services::ConfiguracionService;
use crate::AppState;

use super::bdp_guard::exigir_modo_bdp;
use super::rate_limit::{limitador_escrituras, limitar};

/// Tope de filas por consulta: la pantalla muestra lo reciente, no el histórico.
const LIMITE_LISTADO: i64 = 100;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/bdp/documentos", get(listar))
        // sentinel-disable-next-line ruta-post-sin-rate-limit: el limitador está en el `layer` de abajo, como en el resto de escrituras BDP.
        .route("/bdp/documentos/sincronizar", post(sincronizar))
        .layer(axum::middleware::from_fn_with_state(
            limitador_escrituras(),
            limitar,
        ))
}

#[derive(Debug, Deserialize)]
pub struct ListarQuery {
    /// Perfil de exportación; sin él, los documentos de todos los perfiles.
    pub perfil: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct SincronizarRequest {
    /// Perfil de exportación. Ausente = el guardado en la configuración.
    pub perfil: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct RespuestaSincronizacion {
    pub ok: bool,
    pub resultado: ResultadoDocumentos,
    /// Perfil que se ha leído: el que se pidió o el de la configuración.
    pub perfil: i32,
}

/// Documentos ya guardados, del más reciente al más antiguo.
pub async fn listar(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListarQuery>,
) -> Result<Json<Vec<DocumentoVentaGuardado>>, AppError> {
    auth.require_role(&[UserRole::Admin])?;
    if let Some(perfil) = query.perfil {
        if perfil <= 0 {
            return Err(AppError::Validation(
                "El perfil de documentos debe ser mayor que 0".into(),
            ));
        }
    }
    let documentos = BdpDocumentosVentaRepository::listar(
        &state.pool,
        auth.user_id,
        query.perfil,
        LIMITE_LISTADO,
    )
    .await
    .map_err(|error| AppError::Internal(format!("Error listando documentos: {error}")))?;
    Ok(Json(documentos))
}

/// Lee ahora la ventana pendiente del perfil y guarda lo que traiga.
pub async fn sincronizar(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<SincronizarRequest>,
) -> Result<Json<RespuestaSincronizacion>, AppError> {
    auth.require_role(&[UserRole::Admin])?;
    exigir_modo_bdp(&state, auth.user_id).await?;

    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
    /* Sin perfil en el cuerpo se usa el configurado. Si tampoco lo hay, se pide
     * explícitamente en vez de inventar uno: leer el perfil equivocado traería
     * documentos que no son de esta cuenta. */
    let perfil = req.perfil.or(config.bdp_documentos_perfil).ok_or_else(|| {
        AppError::Validation(
            "Indica el perfil de exportación de documentos o guárdalo en la configuración".into(),
        )
    })?;

    let resultado =
        bdp_documentos_venta::sincronizar_ahora(&state.pool, auth.user_id, &config, perfil).await?;
    Ok(Json(RespuestaSincronizacion {
        ok: true,
        resultado,
        perfil,
    }))
}
