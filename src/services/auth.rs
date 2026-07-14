use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::{AuthResponse, LoginRequest, RegisterRequest, UserRole};
use crate::repositories::UserRepository;
use crate::services::email::EmailService;

/// Claims del JWT — `sub` es el `user_id`, `exp` la expiración Unix
/// [094A-3] `tid` opcional = `trabajador_id` (si el token es de un trabajador)
/// [cargo-fix] `role`/`effective_role`/`impersonator` para middleware AuthUser.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tid: Option<Uuid>,
    pub role: UserRole,
    pub effective_role: UserRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impersonator: Option<Uuid>,
}

pub struct AuthService;

impl AuthService {
    /// Registra un nuevo usuario: valida unicidad, hashea contraseña, genera JWT
    pub async fn register(
        pool: &PgPool,
        req: RegisterRequest,
        jwt_secret: &str,
    ) -> Result<AuthResponse, AppError> {
        if UserRepository::find_by_email(pool, &req.email)
            .await?
            .is_some()
        {
            return Err(AppError::Conflict("Email ya registrado".into()));
        }

        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(req.password.as_bytes(), &salt)
            .map_err(|e| AppError::Internal(format!("Error al hashear contraseña: {e}")))?
            .to_string();

        let user = UserRepository::create(pool, &req.email, &password_hash).await?;
        let token = Self::generate_token(user.id, jwt_secret)?;

        Ok(AuthResponse {
            token,
            user_id: user.id,
        })
    }

    /// Inicia sesión: verifica credenciales y genera JWT
    pub async fn login(
        pool: &PgPool,
        req: LoginRequest,
        jwt_secret: &str,
    ) -> Result<AuthResponse, AppError> {
        let user = UserRepository::find_by_email(pool, &req.email)
            .await?
            .ok_or(AppError::Unauthorized)?;

        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|e| AppError::Internal(format!("Hash almacenado inválido: {e}")))?;

        Argon2::default()
            .verify_password(req.password.as_bytes(), &parsed_hash)
            .map_err(|_| AppError::Unauthorized)?;

        let token = Self::generate_token(user.id, jwt_secret)?;

        Ok(AuthResponse {
            token,
            user_id: user.id,
        })
    }

    /// Genera un JWT con expiración de 24 horas
    /// [094A-3] Acepta `trabajador_id` para tokens de staff
    pub fn generate_token(user_id: Uuid, secret: &str) -> Result<String, AppError> {
        Self::generate_token_with_tid(user_id, None, secret)
    }

    pub fn generate_token_with_tid(
        user_id: Uuid,
        trabajador_id: Option<Uuid>,
        secret: &str,
    ) -> Result<String, AppError> {
        let timestamp = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(24))
            .ok_or_else(|| AppError::Internal("Error calculando expiración del token".into()))?
            .timestamp();
        let exp = usize::try_from(timestamp)
            .map_err(|_| AppError::Internal("Timestamp fuera de rango".into()))?;

        /* [cargo-fix] role/effective_role derivados de trabajador_id.
         * impersonator = None por ahora (impersonación se implementa aparte). */
        let (role, effective_role) = if trabajador_id.is_some() {
            (UserRole::Trabajador, UserRole::Trabajador)
        } else {
            (UserRole::Admin, UserRole::Admin)
        };

        let claims = Claims {
            sub: user_id,
            exp,
            tid: trabajador_id,
            role,
            effective_role,
            impersonator: None,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .map_err(|e| AppError::Internal(format!("Error generando token: {e}")))
    }

    /// Verifica un JWT y retorna los claims
    pub fn verify_token(token: &str, secret: &str) -> Result<Claims, AppError> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|_| AppError::Unauthorized)
    }

    /* [263A-15] Forgot/reset password */

    /// Genera token de reset, lo guarda en BD y envía email.
    /// Siempre retorna Ok para no revelar si el email existe.
    pub async fn forgot_password(
        pool: &PgPool,
        email: &str,
        config: &AppConfig,
    ) -> Result<(), AppError> {
        let user = UserRepository::find_by_email(pool, email).await?;
        if user.is_none() {
            /* No revelamos si el email existe — retornamos Ok silenciosamente */
            return Ok(());
        }

        let token = Self::generate_reset_token();
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);

        UserRepository::set_reset_token(pool, email, &token, expires_at).await?;

        let enlace = format!("{}/reset-password?token={}", config.app_url, token);
        EmailService::enviar_reset_password(config.smtp.as_ref(), email, &enlace)
            .await
            .map_err(|e| AppError::Internal(format!("Error enviando email: {e}")))?;

        Ok(())
    }

    /// Valida el token de reset y actualiza la contraseña
    pub async fn reset_password(
        pool: &PgPool,
        token: &str,
        new_password: &str,
    ) -> Result<(), AppError> {
        let user = UserRepository::find_by_reset_token(pool, token)
            .await?
            .ok_or(AppError::BadRequest("Token inválido o expirado".into()))?;

        let salt = SaltString::generate(&mut OsRng);
        let new_hash = Argon2::default()
            .hash_password(new_password.as_bytes(), &salt)
            .map_err(|e| AppError::Internal(format!("Error al hashear contraseña: {e}")))?
            .to_string();

        UserRepository::update_password(pool, user.id, &new_hash).await?;

        Ok(())
    }

    /// Genera un token hexadecimal seguro de 32 bytes (64 chars)
    fn generate_reset_token() -> String {
        use rand::Rng;
        use std::fmt::Write;
        let bytes: [u8; 32] = rand::thread_rng().gen();
        let mut hex = String::with_capacity(64);
        for b in bytes {
            let _ = write!(hex, "{b:02x}");
        }
        hex
    }
}
