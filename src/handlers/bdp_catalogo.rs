/* [198A-1/D7] Clasificaciones locales (departamento/familia) con código BDP
 * secuencial. CRUD local 100% operativo sin BDP; el alta encola el push y el
 * worker decide según el modo (en standalone no envía nada).
 *
 * GET  /api/bdp/catalogo/:tipo   — listar por tipo (departamento | familia)
 * POST /api/bdp/catalogo          — crear { tipo, nombre }
 * POST /api/bdp/catalogo/importar-departamentos — importar departamentos del
 *   perfil BDP a clasificaciones locales (solo lectura en BDP, ver F0 de 159A-2).
 */

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    BdpCatalogoClasificacion, CrearBdpClasificacionRequest, TIPO_DEPARTAMENTO, TIPO_FAMILIA,
};
use crate::repositories::BdpCatalogoClasificacionRepository;
use crate::services::{
    payload_crear_departamento, payload_crear_familia, verificar_permiso, AccionPermiso,
    BdpImportDepartamentosResult, BdpImportDepartamentosService, BdpPushService,
    ConfiguracionService,
};
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::BdpDepartmentsExportFromProfileRequest;
use crate::AppState;

use super::bdp_guard::exigir_modo_bdp;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/bdp/catalogo/:tipo", get(listar_clasificaciones))
        .route("/bdp/catalogo", axum::routing::post(crear_clasificacion))
        .route(
            "/bdp/catalogo/importar-departamentos",
            axum::routing::post(importar_departamentos),
        )
}

fn validar_tipo(tipo: &str) -> Result<(), AppError> {
    match tipo {
        TIPO_DEPARTAMENTO | TIPO_FAMILIA => Ok(()),
        _ => Err(AppError::Validation(
            "tipo debe ser 'departamento' o 'familia'".into(),
        )),
    }
}

pub async fn listar_clasificaciones(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(tipo): Path<String>,
) -> Result<Json<Vec<BdpCatalogoClasificacion>>, AppError> {
    validar_tipo(&tipo)?;
    let items =
        BdpCatalogoClasificacionRepository::listar(&state.pool, auth.user_id, &tipo).await?;
    Ok(Json(items))
}

pub async fn crear_clasificacion(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CrearBdpClasificacionRequest>,
) -> Result<Json<BdpCatalogoClasificacion>, AppError> {
    /* D7/edición de catálogo: mismo permiso que el CRUD de artículos. */
    verificar_permiso(&state.pool, AccionPermiso::CatalogoEdicion, &auth).await?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    validar_tipo(&req.tipo)?;

    let item = BdpCatalogoClasificacionRepository::crear(&state.pool, auth.user_id, &req).await?;

    /* Encolar el push BDP (el worker no envía nada en standalone). */
    let (dominio, payload) = if req.tipo == TIPO_DEPARTAMENTO {
        (
            crate::services::bdp_push::DOMINIO_DEPARTAMENTO,
            payload_crear_departamento(item.code, &item.nombre),
        )
    } else {
        (
            crate::services::bdp_push::DOMINIO_FAMILIA,
            payload_crear_familia(item.code, &item.nombre),
        )
    };
    let payload = payload.map_err(AppError::Internal)?;
    BdpPushService::encolar(
        &state.pool,
        auth.user_id,
        dominio,
        &item.id.to_string(),
        crate::services::bdp_push::OPERACION_CREAR,
        &payload,
    )
    .await
    .map_err(AppError::Internal)?;

    Ok(Json(item))
}

/* [159A-2/F1] Importar departamentos del perfil BDP a clasificaciones locales.
 * Solo LEE del BDP (ExportDepartmentsFromProfile); nunca escribe en BDP, así que
 * no requiere armado de escritura, pero sí modo BDP + permiso de edición de
 * catálogo (modifica datos locales). No pisa filas locales (ver servicio). */
#[utoipa::path(
    post,
    path = "/api/bdp/catalogo/importar-departamentos",
    tag = "BDP Catálogo",
    responses(
        (status = 200, description = "Departamentos importados", body = BdpImportDepartamentosResult),
        (status = 400, description = "BDP no configurado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn importar_departamentos(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<BdpImportDepartamentosResult>, AppError> {
    exigir_modo_bdp(&state, auth.user_id).await?;
    /* Modifica el catálogo local: mismo permiso que el alta de clasificaciones. */
    verificar_permiso(&state.pool, AccionPermiso::CatalogoEdicion, &auth).await?;
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation(
            "BDP no está configurado. Configura URL y credenciales primero.".into(),
        ));
    }

    let client = BdpWeblinkClient::new(&config);
    let _session = client
        .login()
        .await
        .map_err(|e| AppError::Internal(format!("Error login BDP: {e}")))?;

    let valor = client
        .export_departments_from_profile(&BdpDepartmentsExportFromProfileRequest {
            profile_id: config.bdp_items_profile_id,
        })
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;

    let items = crate::services::aplanar_departamentos(&valor);
    let resultado = BdpImportDepartamentosService::importar(
        &state.pool,
        auth.user_id,
        &items,
        TIPO_DEPARTAMENTO,
    )
    .await
    .map_err(AppError::Internal)?;

    Ok(Json(resultado))
}
