/* [263A-14] Plano de sala: paredes.
 * Parte de plano_sala (ver mod.rs). */

use super::*;

/* ========== Paredes — 094A-7 ========== */

#[utoipa::path(
    post,
    path = "/api/plano-sala/paredes",
    tag = "PlanoSala",
    request_body = CrearParedRequest,
    responses(
        (status = 201, description = "Pared creada", body = ParedSala),
        (status = 401, description = "No autorizado"),
        (status = 422, description = "Error de validación")
    ),
    security(("bearer_auth" = []))
)]
pub async fn crear_pared(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<CrearParedRequest>,
) -> Result<(StatusCode, Json<ParedSala>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let datos = NuevaPared {
        zona_id: req.zona_id,
        pos_x: req.pos_x.unwrap_or(0),
        pos_y: req.pos_y.unwrap_or(0),
        ancho: req.ancho.unwrap_or(100),
        alto: req.alto.unwrap_or(20),
        rotacion: req.rotacion.unwrap_or(0),
        color: req.color.as_deref().unwrap_or("#6b7280"),
    };
    let pared = crate::repositories::PlanoSalaRepository::crear_pared(&state.pool, datos).await?;

    Ok((StatusCode::CREATED, Json(pared)))
}

#[utoipa::path(
    patch,
    path = "/api/plano-sala/paredes/{id}",
    tag = "PlanoSala",
    request_body = ActualizarParedRequest,
    params(("id" = Uuid, Path, description = "ID de la pared")),
    responses(
        (status = 200, description = "Pared actualizada", body = ParedSala),
        (status = 401, description = "No autorizado")
    ),
    security(("bearer_auth" = []))
)]
pub async fn actualizar_pared(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ActualizarParedRequest>,
) -> Result<Json<ParedSala>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let cambios = CambiosPared {
        pos_x: req.pos_x,
        pos_y: req.pos_y,
        ancho: req.ancho,
        alto: req.alto,
        rotacion: req.rotacion,
        color: req.color.as_deref(),
    };
    let pared =
        crate::repositories::PlanoSalaRepository::actualizar_pared(&state.pool, id, cambios)
            .await?;

    Ok(Json(pared))
}

#[utoipa::path(
    delete,
    path = "/api/plano-sala/paredes/{id}",
    tag = "PlanoSala",
    params(("id" = Uuid, Path, description = "ID de la pared")),
    responses(
        (status = 204, description = "Pared eliminada"),
        (status = 404, description = "Pared no encontrada")
    ),
    security(("bearer_auth" = []))
)]
pub async fn eliminar_pared(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let deleted = crate::repositories::PlanoSalaRepository::eliminar_pared(&state.pool, id).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound("Pared no encontrada".into()))
    }
}

#[utoipa::path(
    patch,
    path = "/api/plano-sala/paredes/posiciones",
    tag = "PlanoSala",
    request_body = ActualizarPosicionesParedesRequest,
    responses(
        (status = 200, description = "Posiciones actualizadas"),
        (status = 401, description = "No autorizado")
    ),
    security(("bearer_auth" = []))
)]
pub async fn actualizar_posiciones_paredes(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<ActualizarPosicionesParedesRequest>,
) -> Result<StatusCode, AppError> {
    let datos: Vec<(Uuid, i32, i32)> = req
        .posiciones
        .iter()
        .map(|p| (p.id, p.pos_x, p.pos_y))
        .collect();
    crate::repositories::PlanoSalaRepository::actualizar_posiciones_paredes(&state.pool, &datos)
        .await?;
    Ok(StatusCode::OK)
}
