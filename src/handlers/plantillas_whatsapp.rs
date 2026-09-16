/* [263A-24] Handlers de plantillas WhatsApp.
 * CRUD + envío a Meta (stub). Protegidos con AuthUser.
 * Nota: /plantillas-whatsapp/... se registra ANTES de /:id */

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    ActualizarPlantillaRequest, CrearPlantillaRequest, PlantillaWhatsapp, PlantillasPaginadas,
    PlantillasQuery,
};
use crate::services::PlantillaService;
use crate::services::verificar_seccion;
use crate::AppState;

/// Crear una plantilla `WhatsApp`
#[utoipa::path(
    post,
    path = "/api/plantillas-whatsapp",
    tag = "Plantillas WhatsApp",
    request_body = CrearPlantillaRequest,
    responses(
        (status = 201, description = "Plantilla creada", body = PlantillaWhatsapp),
        (status = 401, description = "No autorizado"),
        (status = 422, description = "Error de validación")
    ),
    security(("bearer_auth" = []))
)]
pub async fn crear_plantilla(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CrearPlantillaRequest>,
) -> Result<(StatusCode, Json<PlantillaWhatsapp>), AppError> {
    /* [169A-3] Plantillas que salen a Meta/clientes: exige "plantillas_wa".
     * El guard va primero (fail-closed antes de validar). */
    verificar_seccion(&state.pool, &auth, "plantillas_wa").await?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let plantilla = PlantillaService::create(&state.pool, auth.user_id, req).await?;
    Ok((StatusCode::CREATED, Json(plantilla)))
}

/// Listar plantillas con paginación y filtro de estado
#[utoipa::path(
    get,
    path = "/api/plantillas-whatsapp",
    tag = "Plantillas WhatsApp",
    params(PlantillasQuery),
    responses(
        (status = 200, description = "Lista de plantillas", body = PlantillasPaginadas),
        (status = 401, description = "No autorizado")
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_plantillas(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<PlantillasQuery>,
) -> Result<Json<PlantillasPaginadas>, AppError> {
    let result = PlantillaService::list(&state.pool, auth.user_id, query).await?;
    Ok(Json(result))
}

/// Obtener una plantilla por ID
#[utoipa::path(
    get,
    path = "/api/plantillas-whatsapp/{id}",
    tag = "Plantillas WhatsApp",
    params(("id" = Uuid, Path, description = "ID de la plantilla")),
    responses(
        (status = 200, description = "Plantilla encontrada", body = PlantillaWhatsapp),
        (status = 401, description = "No autorizado"),
        (status = 404, description = "No encontrada")
    ),
    security(("bearer_auth" = []))
)]
pub async fn obtener_plantilla(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<PlantillaWhatsapp>, AppError> {
    let plantilla = PlantillaService::get(&state.pool, id, auth.user_id).await?;
    Ok(Json(plantilla))
}

/// Actualizar una plantilla (solo borradores)
#[utoipa::path(
    put,
    path = "/api/plantillas-whatsapp/{id}",
    tag = "Plantillas WhatsApp",
    params(("id" = Uuid, Path, description = "ID de la plantilla")),
    request_body = ActualizarPlantillaRequest,
    responses(
        (status = 200, description = "Plantilla actualizada", body = PlantillaWhatsapp),
        (status = 401, description = "No autorizado"),
        (status = 404, description = "No encontrada"),
        (status = 422, description = "Error de validación")
    ),
    security(("bearer_auth" = []))
)]
pub async fn actualizar_plantilla(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ActualizarPlantillaRequest>,
) -> Result<Json<PlantillaWhatsapp>, AppError> {
    /* [169A-3] Ver crear_plantilla. */
    verificar_seccion(&state.pool, &auth, "plantillas_wa").await?;
    let plantilla = PlantillaService::update(&state.pool, id, auth.user_id, req).await?;
    Ok(Json(plantilla))
}

/// Eliminar una plantilla
#[utoipa::path(
    delete,
    path = "/api/plantillas-whatsapp/{id}",
    tag = "Plantillas WhatsApp",
    params(("id" = Uuid, Path, description = "ID de la plantilla")),
    responses(
        (status = 204, description = "Plantilla eliminada"),
        (status = 401, description = "No autorizado"),
        (status = 404, description = "No encontrada")
    ),
    security(("bearer_auth" = []))
)]
pub async fn eliminar_plantilla(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    /* [169A-3] Ver crear_plantilla. */
    verificar_seccion(&state.pool, &auth, "plantillas_wa").await?;
    PlantillaService::delete(&state.pool, id, auth.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Enviar plantilla a Meta para aprobación
#[utoipa::path(
    post,
    path = "/api/plantillas-whatsapp/{id}/enviar",
    tag = "Plantillas WhatsApp",
    params(("id" = Uuid, Path, description = "ID de la plantilla")),
    responses(
        (status = 200, description = "Plantilla enviada a Meta", body = PlantillaWhatsapp),
        (status = 401, description = "No autorizado"),
        (status = 404, description = "No encontrada"),
        (status = 422, description = "Error de validación (estado incorrecto)")
    ),
    security(("bearer_auth" = []))
)]
pub async fn enviar_a_meta(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<PlantillaWhatsapp>, AppError> {
    /* [169A-3] Ver crear_plantilla. */
    verificar_seccion(&state.pool, &auth, "plantillas_wa").await?;
    let plantilla = PlantillaService::enviar_a_meta(&state.pool, id, auth.user_id).await?;
    Ok(Json(plantilla))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/plantillas-whatsapp",
            post(crear_plantilla).get(listar_plantillas),
        )
        .route(
            "/plantillas-whatsapp/:id",
            get(obtener_plantilla)
                .put(actualizar_plantilla)
                .delete(eliminar_plantilla),
        )
        .route("/plantillas-whatsapp/:id/enviar", post(enviar_a_meta))
}
