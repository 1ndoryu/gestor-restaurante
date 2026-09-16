/* [094A-4] Handlers de reseñas.
 * Endpoints autenticados (panel del propietario) + endpoints públicos (cliente responde).
 * El flujo: propietario crea solicitud → se envía enlace al cliente → cliente puntúa
 * → si 4-5★ se redirige a Google Business. */

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    ResenaPublicaResponse, ResenasPaginadas, ResenasQuery, ResponderResenaRequest,
    ResponderResenaResponse,
};
use crate::repositories::{ConfiguracionRepository, ResenaRepository};
use crate::services::verificar_seccion;
use crate::AppState;

/* ========== Panel del propietario (autenticado) ========== */

#[utoipa::path(
    get,
    path = "/api/resenas",
    tag = "resenas",
    params(
        ("page" = Option<i64>, Query, description = "Página"),
        ("per_page" = Option<i64>, Query, description = "Ítems por página"),
        ("min_puntuacion" = Option<i16>, Query, description = "Filtro mínimo"),
        ("max_puntuacion" = Option<i16>, Query, description = "Filtro máximo"),
        ("solo_respondidas" = Option<bool>, Query, description = "Solo reseñas respondidas")
    ),
    responses(
        (status = 200, description = "Lista de reseñas", body = ResenasPaginadas),
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_resenas(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ResenasQuery>,
) -> Result<Json<ResenasPaginadas>, AppError> {
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(20).clamp(1, 100);
    let (data, total) = ResenaRepository::list(
        &state.pool,
        auth.user_id,
        page,
        per_page,
        q.min_puntuacion,
        q.max_puntuacion,
        q.solo_respondidas.unwrap_or(false),
    )
    .await?;
    Ok(Json(ResenasPaginadas { data, total }))
}

/* Crear solicitud de reseña (genera token) */
#[utoipa::path(
    post,
    path = "/api/resenas/solicitar",
    tag = "resenas",
    params(
        ("reserva_id" = Option<Uuid>, Query, description = "ID de la reserva asociada"),
        ("cliente_id" = Option<Uuid>, Query, description = "ID del cliente")
    ),
    responses(
        (status = 201, description = "Solicitud creada con token y URL"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn solicitar_resena(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<SolicitarParams>,
) -> Result<(StatusCode, Json<SolicitarResponse>), AppError> {
    /* Si ya existe una reseña para esta reserva, no duplicar */
    /* [169A-3] Mensaje que sale al cliente: exige sección "resenas". */
    verificar_seccion(&state.pool, &auth, "resenas").await?;
    if let Some(reserva_id) = params.reserva_id {
        if ResenaRepository::existe_para_reserva(&state.pool, reserva_id).await? {
            return Err(AppError::Conflict(
                "Ya se solicitó reseña para esta reserva".into(),
            ));
        }
    }

    /* Generar token aleatorio seguro (32 bytes → 64 hex) */
    let token: String = {
        use rand::RngCore;
        use std::fmt::Write;
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        bytes.iter().fold(String::with_capacity(64), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
    };

    let resena = ResenaRepository::create(
        &state.pool,
        auth.user_id,
        params.reserva_id,
        params.cliente_id,
        &token,
    )
    .await?;

    /* Construir URL pública */
    let base_url = std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000".into());
    let url = format!("{base_url}/resena/{}", resena.token);

    Ok((
        StatusCode::CREATED,
        Json(SolicitarResponse {
            id: resena.id,
            token: resena.token,
            url,
        }),
    ))
}

/* ========== Endpoints públicos (sin auth) ========== */

/* Obtener estado de una reseña por token (para la landing) */
#[utoipa::path(
    get,
    path = "/api/public/resenas/{token}",
    tag = "resenas",
    params(("token" = String, Path, description = "Token único de la reseña")),
    responses(
        (status = 200, description = "Info de la reseña", body = ResenaPublicaResponse),
        (status = 404, description = "Token inválido")
    )
)]
pub async fn obtener_resena_publica(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<ResenaPublicaResponse>, AppError> {
    let resena = ResenaRepository::find_by_token(&state.pool, &token)
        .await?
        .ok_or_else(|| AppError::NotFound("Reseña no encontrada".into()))?;

    /* Obtener nombre del restaurante para la landing */
    let config = ConfiguracionRepository::obtener_o_crear(&state.pool, resena.user_id).await?;

    Ok(Json(ResenaPublicaResponse {
        token: resena.token,
        respondida: resena.respondida_at.is_some(),
        nombre_restaurante: Some(config.nombre_restaurante),
    }))
}

/* Responder reseña (cliente envía puntuación) */
#[utoipa::path(
    post,
    path = "/api/public/resenas/{token}",
    tag = "resenas",
    params(("token" = String, Path, description = "Token único")),
    request_body = ResponderResenaRequest,
    responses(
        (status = 200, description = "Gracias + redirect si aplica", body = ResponderResenaResponse),
        (status = 404, description = "Token inválido"),
        (status = 409, description = "Ya respondida")
    )
)]
pub async fn responder_resena(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(req): Json<ResponderResenaRequest>,
) -> Result<Json<ResponderResenaResponse>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let resena = ResenaRepository::find_by_token(&state.pool, &token)
        .await?
        .ok_or_else(|| AppError::NotFound("Reseña no encontrada".into()))?;

    if resena.respondida_at.is_some() {
        return Err(AppError::Conflict("Esta reseña ya fue respondida".into()));
    }

    /* Si puntuación >= 4, buscar google_review_url */
    let redirect_url = if req.puntuacion >= 4 {
        let config = ConfiguracionRepository::obtener_o_crear(&state.pool, resena.user_id).await?;
        if config.google_review_url.is_empty() {
            None
        } else {
            Some(config.google_review_url)
        }
    } else {
        None
    };

    ResenaRepository::responder(
        &state.pool,
        &token,
        req.puntuacion,
        req.comentario.as_deref().unwrap_or(""),
        redirect_url.is_some(),
    )
    .await?;

    let gracias = if redirect_url.is_some() {
        "¡Gracias por tu valoración! Te redirigimos a Google para dejar tu reseña.".into()
    } else {
        "¡Gracias por tu feedback! Lo tendremos en cuenta para mejorar.".into()
    };

    Ok(Json(ResponderResenaResponse {
        gracias,
        redirect_url,
    }))
}

/* ========== Tipos auxiliares ========== */

#[derive(Debug, serde::Deserialize)]
pub struct SolicitarParams {
    pub reserva_id: Option<Uuid>,
    pub cliente_id: Option<Uuid>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct SolicitarResponse {
    pub id: Uuid,
    pub token: String,
    pub url: String,
}

/* ========== Router ========== */

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/resenas", get(listar_resenas))
        .route("/resenas/solicitar", post(solicitar_resena))
}

/* Rutas públicas (sin auth) — se montan por separado */
pub fn public_routes() -> Router<AppState> {
    Router::new().route(
        "/public/resenas/:token",
        get(obtener_resena_publica).post(responder_resena),
    )
}
