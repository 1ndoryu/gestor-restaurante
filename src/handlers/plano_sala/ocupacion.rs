/* [263A-14] Plano de sala: ocupación.
 * Parte de plano_sala (ver mod.rs). */

use axum::extract::{Query, State};
use axum::Json;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{PlanoOcupacion, PlanoOcupacionQuery};
use crate::services::PlanoSalaService;
use crate::AppState;

/* ========== Ocupación (263A-16) ========== */

/* [014A-4] turno_a_rango ahora acepta la config de turnos.
 * Si no se tiene config disponible, usa defaults. */
/* Hora literal del turno por defecto. `from_hms_opt` solo rechaza combinaciones que no
 * forman una hora real; los literales de este modulo si lo son, asi que se devuelve el
 * `Option` tal cual en lugar de entrar en panico si alguno se rompiera. */
fn hora_literal(hora: u32, minuto: u32, segundo: u32) -> Option<chrono::NaiveTime> {
    chrono::NaiveTime::from_hms_opt(hora, minuto, segundo)
}

fn turno_a_rango_config(
    turno: &str,
    config: Option<&crate::models::ConfiguracionRestaurante>,
) -> (Option<chrono::NaiveTime>, Option<chrono::NaiveTime>) {
    match (turno, config) {
        ("desayuno", Some(c)) => (Some(c.hora_desayuno_inicio), Some(c.hora_desayuno_fin)),
        ("comida", Some(c)) => (Some(c.hora_comida_inicio), Some(c.hora_comida_fin)),
        ("cena", Some(c)) => (Some(c.hora_cena_inicio), Some(c.hora_cena_fin)),
        ("desayuno", None) => (hora_literal(0, 0, 0), hora_literal(12, 0, 0)),
        ("comida", None) => (hora_literal(12, 0, 0), hora_literal(18, 0, 0)),
        ("cena", None) => (hora_literal(18, 0, 0), hora_literal(23, 59, 59)),
        _ => (None, None),
    }
}

#[utoipa::path(
    get,
    path = "/api/plano-sala/ocupacion",
    tag = "PlanoSala",
    params(PlanoOcupacionQuery),
    responses(
        (status = 200, description = "Plano con ocupación de mesas", body = PlanoOcupacion),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn obtener_ocupacion(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<PlanoOcupacionQuery>,
) -> Result<Json<PlanoOcupacion>, AppError> {
    /* [014A-4] Obtener config de turnos para rangos horarios configurables */
    let config =
        crate::repositories::ConfiguracionRepository::obtener_o_crear(&state.pool, auth.user_id)
            .await
            .ok();

    let (hora_desde, hora_hasta) = query
        .turno
        .as_deref()
        .map_or((None, None), |t| turno_a_rango_config(t, config.as_ref()));

    let plano = PlanoSalaService::plano_ocupacion(
        &state.pool,
        auth.user_id,
        query.fecha,
        hora_desde,
        hora_hasta,
    )
    .await?;
    Ok(Json(plano))
}
