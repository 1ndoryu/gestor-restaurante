/* [263A-23] Handlers de campañas de marketing.
 * CRUD + preview segmento + envío (stub).
 * Todos los endpoints protegidos con AuthUser (JWT). */

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    ActualizarCampanaRequest, Campana, CampanasPaginadas, CampanasQuery, CrearCampanaRequest,
    SegmentoPreview, SegmentoPreviewQuery,
};
use crate::services::verificar_seccion;
use crate::services::CampanaService;
use crate::AppState;

/// Crear una campaña de marketing
#[utoipa::path(
    post,
    path = "/api/campanas",
    tag = "Campanas",
    request_body = CrearCampanaRequest,
    responses(
        (status = 201, description = "Campaña creada", body = Campana),
        (status = 401, description = "No autorizado"),
        (status = 422, description = "Error de validación")
    ),
    security(("bearer_auth" = []))
)]
pub async fn crear_campana(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CrearCampanaRequest>,
) -> Result<(StatusCode, Json<Campana>), AppError> {
    /* [169A-3] Enviar WhatsApp a clientes: exige sección "campanas".
     * El guard va primero (fail-closed antes de validar). */
    verificar_seccion(&state.pool, &auth, "campanas").await?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let campana = CampanaService::create(&state.pool, auth.user_id, req).await?;
    Ok((StatusCode::CREATED, Json(campana)))
}

/// Obtener una campaña por ID
#[utoipa::path(
    get,
    path = "/api/campanas/{id}",
    tag = "Campanas",
    params(("id" = Uuid, Path, description = "ID de la campaña")),
    responses(
        (status = 200, description = "Campaña encontrada", body = Campana),
        (status = 404, description = "No encontrada"),
        (status = 401, description = "No autorizado")
    ),
    security(("bearer_auth" = []))
)]
pub async fn obtener_campana(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Campana>, AppError> {
    let campana = CampanaService::get(&state.pool, id, auth.user_id).await?;
    Ok(Json(campana))
}

/// Listar campañas con paginación y filtro de estado
#[utoipa::path(
    get,
    path = "/api/campanas",
    tag = "Campanas",
    params(CampanasQuery),
    responses(
        (status = 200, description = "Lista de campañas", body = CampanasPaginadas),
        (status = 401, description = "No autorizado")
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_campanas(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<CampanasQuery>,
) -> Result<Json<CampanasPaginadas>, AppError> {
    let resultado = CampanaService::list(&state.pool, auth.user_id, query).await?;
    Ok(Json(resultado))
}

/// Actualizar una campaña (solo borradores)
#[utoipa::path(
    put,
    path = "/api/campanas/{id}",
    tag = "Campanas",
    params(("id" = Uuid, Path, description = "ID de la campaña")),
    request_body = ActualizarCampanaRequest,
    responses(
        (status = 200, description = "Campaña actualizada", body = Campana),
        (status = 404, description = "No encontrada o no editable"),
        (status = 401, description = "No autorizado")
    ),
    security(("bearer_auth" = []))
)]
pub async fn actualizar_campana(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ActualizarCampanaRequest>,
) -> Result<Json<Campana>, AppError> {
    /* [169A-3] Ver crear_campana (guard primero). */
    verificar_seccion(&state.pool, &auth, "campanas").await?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let campana = CampanaService::update(&state.pool, id, auth.user_id, req).await?;
    Ok(Json(campana))
}

/// Eliminar una campaña
#[utoipa::path(
    delete,
    path = "/api/campanas/{id}",
    tag = "Campanas",
    params(("id" = Uuid, Path, description = "ID de la campaña")),
    responses(
        (status = 204, description = "Campaña eliminada"),
        (status = 404, description = "No encontrada"),
        (status = 401, description = "No autorizado")
    ),
    security(("bearer_auth" = []))
)]
pub async fn eliminar_campana(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    /* [169A-3] Ver crear_campana. */
    verificar_seccion(&state.pool, &auth, "campanas").await?;
    CampanaService::delete(&state.pool, id, auth.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Preview de segmentación: cuántos clientes recibirían la campaña
#[utoipa::path(
    get,
    path = "/api/campanas/segmentos/preview",
    tag = "Campanas",
    params(SegmentoPreviewQuery),
    responses(
        (status = 200, description = "Preview del segmento", body = SegmentoPreview),
        (status = 401, description = "No autorizado"),
        (status = 422, description = "Segmento inválido")
    ),
    security(("bearer_auth" = []))
)]
pub async fn preview_segmento(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<SegmentoPreviewQuery>,
) -> Result<Json<SegmentoPreview>, AppError> {
    let preview =
        CampanaService::preview_segmento(&state.pool, auth.user_id, &query.segmento).await?;
    Ok(Json(preview))
}

/// Enviar una campaña (genera destinatarios y dispara envío)
#[utoipa::path(
    post,
    path = "/api/campanas/{id}/enviar",
    tag = "Campanas",
    params(("id" = Uuid, Path, description = "ID de la campaña")),
    responses(
        (status = 200, description = "Campaña enviada", body = Campana),
        (status = 404, description = "No encontrada"),
        (status = 422, description = "Campaña no válida para envío"),
        (status = 401, description = "No autorizado")
    ),
    security(("bearer_auth" = []))
)]
pub async fn enviar_campana(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Campana>, AppError> {
    /* [169A-3] Ver crear_campana. */
    verificar_seccion(&state.pool, &auth, "campanas").await?;
    let campana = CampanaService::enviar(&state.pool, id, auth.user_id).await?;
    Ok(Json(campana))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/campanas", post(crear_campana).get(listar_campanas))
        .route("/campanas/segmentos/preview", get(preview_segmento))
        .route(
            "/campanas/:id",
            get(obtener_campana)
                .put(actualizar_campana)
                .delete(eliminar_campana),
        )
        .route("/campanas/:id/enviar", post(enviar_campana))
}
