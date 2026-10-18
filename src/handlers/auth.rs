use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use validator::Validate;

use crate::errors::AppError;
use crate::models::{
    AuthResponse, ForgotPasswordRequest, LoginRequest, MessageResponse, RegisterRequest,
    ResetPasswordRequest,
};
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
        .route("/auth/forgot-password", post(forgot_password))
        .route("/auth/reset-password", post(reset_password))
        .layer(axum::middleware::from_fn_with_state(limitador, limitar))
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
