/* [26AA-2] Lectura programada de formas de pago y empleados.
 *
 *   GET /api/bdp/lecturas-programadas          — cadencia, última lectura y error
 *   PUT /api/bdp/lecturas-programadas/:maestro — cambia la cadencia (0 = apagada)
 *
 * Solo configura cuándo lee el servidor; no escribe nada en BDP. */

use axum::extract::{Path, State};
use axum::routing::{get, put};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::services::bdp_pagos_empleados_auto::{self as auto, Maestro};
use crate::AppState;

use super::bdp_guard::exigir_modo_bdp;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/bdp/lecturas-programadas", get(listar))
        .route("/bdp/lecturas-programadas/:maestro", put(cambiar))
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct LecturaProgramada {
    pub maestro: String,
    pub segundos: i32,
    pub ultima_lectura: Option<DateTime<Utc>>,
    pub ultima_ok: Option<DateTime<Utc>>,
    pub ultimo_error: Option<String>,
    pub resumen: Option<String>,
    pub activos: i64,
    pub inactivos: i64,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CambiarLectura {
    pub segundos: i32,
}

/// Un registro por maestro aunque la cuenta no haya configurado nada todavía.
async fn lectura_de(
    state: &AppState,
    user_id: uuid::Uuid,
    maestro: Maestro,
) -> Result<LecturaProgramada, AppError> {
    let e = auto::estado(&state.pool, user_id, maestro)
        .await
        .map_err(|e| AppError::Internal(format!("Error leyendo la cadencia: {e}")))?;
    Ok(LecturaProgramada {
        maestro: maestro.clave().to_string(),
        segundos: e.segundos,
        ultima_lectura: e.ultima_lectura,
        ultima_ok: e.ultima_ok,
        ultimo_error: e.ultimo_error,
        resumen: e.resumen,
        activos: e.activos,
        inactivos: e.inactivos,
    })
}

/// Cadencia y estado de la lectura de formas de pago y empleados.
#[utoipa::path(
    get,
    path = "/api/bdp/lecturas-programadas",
    tag = "BDP Backup",
    responses((status = 200, body = Vec<LecturaProgramada>)),
    security(("bearer_auth" = []))
)]
pub async fn listar(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<LecturaProgramada>>, AppError> {
    let mut lecturas = Vec::new();
    for maestro in Maestro::TODOS {
        lecturas.push(lectura_de(&state, auth.user_id, maestro).await?);
    }
    Ok(Json(lecturas))
}

/// Enciende, cambia o apaga la lectura de un maestro.
#[utoipa::path(
    put,
    path = "/api/bdp/lecturas-programadas/{maestro}",
    tag = "BDP Backup",
    request_body = CambiarLectura,
    responses((status = 200, body = LecturaProgramada), (status = 400, description = "Valor no válido")),
    security(("bearer_auth" = []))
)]
pub async fn cambiar(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(clave): Path<String>,
    Json(req): Json<CambiarLectura>,
) -> Result<Json<LecturaProgramada>, AppError> {
    exigir_modo_bdp(&state, auth.user_id).await?;
    let maestro = Maestro::desde_clave(&clave)
        .ok_or_else(|| AppError::Validation("Maestro desconocido.".into()))?;
    /* Un valor negativo se leería como «vencido siempre» y martillearía al TPV. */
    if req.segundos < 0 {
        return Err(AppError::Validation(
            "La cadencia no puede ser negativa.".into(),
        ));
    }
    auto::fijar_cadencia(&state.pool, auth.user_id, maestro, req.segundos)
        .await
        .map_err(|e| AppError::Internal(format!("Error guardando la cadencia: {e}")))?;
    Ok(Json(lectura_de(&state, auth.user_id, maestro).await?))
}
