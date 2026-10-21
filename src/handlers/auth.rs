use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    AuthResponse, ForgotPasswordRequest, LoginRequest, MessageResponse, RegisterRequest,
    ResetPasswordRequest, UserRole,
};
use crate::repositories::UserRepository;
use crate::services::AuthService;
use crate::AppState;

use super::rate_limit::{limitar, Limitador};

/// Registrar nuevo usuario
#[utoipa::path(
    post,
    path = "/api/auth/register",
    tag = "Auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "Usuario registrado", body = AuthResponse),
        (status = 409, description = "Email ya registrado", body = ErrorResponse),
        (status = 422, description = "Error de validación", body = ErrorResponse)
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let response = AuthService::register(&state.pool, req, &state.jwt_secret).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// Iniciar sesión
#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "Auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login exitoso", body = AuthResponse),
        (status = 401, description = "Credenciales inválidas", body = ErrorResponse)
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let response = AuthService::login(&state.pool, req, &state.jwt_secret).await?;
    Ok(Json(response))
}

/* [por que] Estas rutas aceptan credenciales o disparan emails: sin tope, un bucle
 * de POST prueba contraseñas o inunda el buzón de un usuario. 12 por minuto sostenidos
 * por IP, con ráfaga de 10 para que un usuario que se equivoca no quede bloqueado. */
const AUTH_PETICIONES_POR_SEGUNDO: f64 = 0.2;
const AUTH_RAFAGA: f64 = 10.0;

pub fn routes() -> Router<AppState> {
    let limitador = Limitador::nuevo(AUTH_PETICIONES_POR_SEGUNDO, AUTH_RAFAGA);
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/intentos", get(listar_intentos))
        .route("/auth/password", axum::routing::put(cambiar_password))
        .route("/auth/forgot-password", post(forgot_password))
        .route("/auth/reset-password", post(reset_password))
        .layer(axum::middleware::from_fn_with_state(limitador, limitar))
}

/* [D02] Intentos de acceso: evidencia de «la contraseña vieja no entra, la nueva
 * sí, con registro de intentos». Solo el propietario los consulta, y nunca
 * devuelve contraseñas ni hashes. */

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct IntentoAcceso {
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub email: String,
    pub exito: bool,
    pub motivo: String,
}

/// Lista los intentos de acceso más recientes.
#[utoipa::path(
    get,
    path = "/api/auth/intentos",
    tag = "Auth",
    params(("limite" = Option<i64>, Query, description = "Máximo de intentos (1-500, por defecto 50)")),
    responses(
        (status = 200, description = "Intentos recientes", body = Vec<IntentoAcceso>),
        (status = 403, description = "Solo el propietario", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_intentos(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<IntentoAcceso>>, AppError> {
    auth.require_role(&[UserRole::Admin])?;
    let limite = params
        .get("limite")
        .and_then(|valor| valor.parse::<i64>().ok())
        .unwrap_or(50)
        .clamp(1, 500);
    let intentos = UserRepository::listar_intentos_acceso(&state.pool, limite)
        .await
        .map_err(|error| {
            AppError::Internal(format!("No se pudieron leer los intentos: {error}"))
        })?;
    Ok(Json(
        intentos
            .into_iter()
            .map(|fila| IntentoAcceso {
                created_at: fila.created_at,
                email: fila.email,
                exito: fila.exito,
                motivo: fila.motivo,
            })
            .collect(),
    ))
}

/* [D02] Rotación de la contraseña del terminal. */

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct CambiarPasswordRequest {
    pub actual: String,
    pub nueva: String,
}

/// Cambia la contraseña del usuario en sesión.
#[utoipa::path(
    put,
    path = "/api/auth/password",
    tag = "Auth",
    request_body = CambiarPasswordRequest,
    responses(
        (status = 200, description = "Contraseña cambiada", body = MessageResponse),
        (status = 401, description = "La contraseña actual no es correcta", body = ErrorResponse),
        (status = 422, description = "La nueva no cumple los requisitos", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn cambiar_password(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CambiarPasswordRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    /* Misma regla que el registro y la recuperación: mínimo 8 caracteres. */
    if req.nueva.chars().count() < 8 {
        return Err(AppError::Validation(
            "La contraseña nueva debe tener al menos 8 caracteres.".into(),
        ));
    }
    if req.nueva == req.actual {
        return Err(AppError::Validation(
            "La contraseña nueva debe ser distinta de la actual.".into(),
        ));
    }
    AuthService::cambiar_password(&state.pool, auth.user_id, &req.actual, &req.nueva).await?;
    Ok(Json(MessageResponse {
        message: "Contraseña actualizada correctamente.".into(),
    }))
}

/* [263A-15] Endpoints de recuperación de contraseña */

/// Solicitar enlace de recuperación por email
#[utoipa::path(
    post,
    path = "/api/auth/forgot-password",
    tag = "Auth",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 200, description = "Si el email existe se enviará un enlace", body = MessageResponse),
        (status = 422, description = "Error de validación", body = ErrorResponse)
    )
)]
pub async fn forgot_password(
    State(state): State<AppState>,
    Json(req): Json<ForgotPasswordRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    AuthService::forgot_password(&state.pool, &req.email, &state.config).await?;

    Ok(Json(MessageResponse {
        message: "Si el email existe, recibirás un enlace para restablecer tu contraseña.".into(),
    }))
}

/// Restablecer contraseña con token
#[utoipa::path(
    post,
    path = "/api/auth/reset-password",
    tag = "Auth",
    request_body = ResetPasswordRequest,
    responses(
        (status = 200, description = "Contraseña actualizada", body = MessageResponse),
        (status = 400, description = "Token inválido o expirado", body = ErrorResponse),
        (status = 422, description = "Error de validación", body = ErrorResponse)
    )
)]
pub async fn reset_password(
    State(state): State<AppState>,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    AuthService::reset_password(&state.pool, &req.token, &req.new_password).await?;

    Ok(Json(MessageResponse {
        message: "Contraseña actualizada correctamente.".into(),
    }))
}
