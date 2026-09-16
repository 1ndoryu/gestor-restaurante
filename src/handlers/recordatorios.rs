/* [263A-25] Handlers de reglas de recordatorio automático.
 * CRUD reglas + historial de envíos. Protegidos con AuthUser. */

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    ActualizarReglaRequest, CrearReglaRequest, HistorialRecordatorios, ReglaRecordatorio,
    ReglasPaginadas, ReglasQuery,
};
use crate::services::RecordatorioService;
use crate::services::verificar_seccion;
use crate::AppState;

/// Crear una regla de recordatorio automático
#[utoipa::path(
    post,
    path = "/api/recordatorios/reglas",
    tag = "Recordatorios",
    request_body = CrearReglaRequest,
    responses(
        (status = 201, description = "Regla creada", body = ReglaRecordatorio),
        (status = 401, description = "No autorizado"),
        (status = 422, description = "Error de validación")
    ),
    security(("bearer_auth" = []))
)]
pub async fn crear_regla(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CrearReglaRequest>,
) -> Result<(StatusCode, Json<ReglaRecordatorio>), AppError> {
    /* [169A-3] Reglas que envían WhatsApp solos: exige "recordatorios". */
    verificar_seccion(&state.pool, &auth, "recordatorios").await?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let regla = RecordatorioService::crear_regla(&state.pool, auth.user_id, req).await?;
    Ok((StatusCode::CREATED, Json(regla)))
}

/// Listar reglas de recordatorio
#[utoipa::path(
    get,
    path = "/api/recordatorios/reglas",
    tag = "Recordatorios",
    params(ReglasQuery),
    responses(
        (status = 200, description = "Lista paginada de reglas", body = ReglasPaginadas),
        (status = 401, description = "No autorizado")
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_reglas(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ReglasQuery>,
) -> Result<Json<ReglasPaginadas>, AppError> {
    let reglas =
        RecordatorioService::listar_reglas(&state.pool, auth.user_id, query.page, query.per_page)
            .await?;
    Ok(Json(reglas))
}

/// Obtener una regla de recordatorio por ID
#[utoipa::path(
    get,
    path = "/api/recordatorios/reglas/{id}",
    tag = "Recordatorios",
    params(("id" = Uuid, Path, description = "ID de la regla")),
    responses(
        (status = 200, description = "Regla encontrada", body = ReglaRecordatorio),
        (status = 404, description = "No encontrada")
    ),
    security(("bearer_auth" = []))
)]
pub async fn obtener_regla(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ReglaRecordatorio>, AppError> {
    let regla = RecordatorioService::obtener_regla(&state.pool, id, auth.user_id).await?;
    Ok(Json(regla))
}

/// Actualizar una regla de recordatorio
#[utoipa::path(
    put,
    path = "/api/recordatorios/reglas/{id}",
    tag = "Recordatorios",
    params(("id" = Uuid, Path, description = "ID de la regla")),
    request_body = ActualizarReglaRequest,
    responses(
        (status = 200, description = "Regla actualizada", body = ReglaRecordatorio),
        (status = 404, description = "No encontrada")
    ),
    security(("bearer_auth" = []))
)]
pub async fn actualizar_regla(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ActualizarReglaRequest>,
) -> Result<Json<ReglaRecordatorio>, AppError> {
    /* [169A-3] Ver crear_regla. */
    verificar_seccion(&state.pool, &auth, "recordatorios").await?;
    let regla = RecordatorioService::actualizar_regla(&state.pool, id, auth.user_id, req).await?;
    Ok(Json(regla))
}

/// Eliminar una regla de recordatorio
#[utoipa::path(
    delete,
    path = "/api/recordatorios/reglas/{id}",
    tag = "Recordatorios",
    params(("id" = Uuid, Path, description = "ID de la regla")),
    responses(
        (status = 204, description = "Regla eliminada"),
        (status = 404, description = "No encontrada")
    ),
    security(("bearer_auth" = []))
)]
pub async fn eliminar_regla(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    /* [169A-3] Ver crear_regla. */
    verificar_seccion(&state.pool, &auth, "recordatorios").await?;
    RecordatorioService::eliminar_regla(&state.pool, id, auth.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Historial de recordatorios enviados
#[utoipa::path(
    get,
    path = "/api/recordatorios/historial",
    tag = "Recordatorios",
    params(ReglasQuery),
    responses(
        (status = 200, description = "Historial paginado", body = HistorialRecordatorios),
        (status = 401, description = "No autorizado")
    ),
    security(("bearer_auth" = []))
)]
pub async fn historial_recordatorios(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ReglasQuery>,
) -> Result<Json<HistorialRecordatorios>, AppError> {
    let historial =
        RecordatorioService::historial(&state.pool, auth.user_id, query.page, query.per_page)
            .await?;
    Ok(Json(historial))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/recordatorios/reglas",
            get(listar_reglas).post(crear_regla),
        )
        .route(
            "/recordatorios/reglas/:id",
            get(obtener_regla)
                .put(actualizar_regla)
                .delete(eliminar_regla),
        )
        .route("/recordatorios/historial", get(historial_recordatorios))
}
