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
/// [cargo-fix] `role`/`effective_role`/`impersonator` para middleware `AuthUser`.
/// [169A-3] `secs` = secciones de menú concedidas al trabajador (firmadas en
/// el token; la UI filtra el menú con ellas). Vacío en tokens de dueño y en
/// tokens antiguos (`default`, decodifican igual: compat hacia atrás).
/// Al cambiar permisos, el trabajador debe volver a entrar: el token nuevo
/// trae las secciones actualizadas.
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
    #[serde(default)]
    pub secs: Vec<String>,
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
        /* [D02] Cada intento queda registrado antes de devolver el resultado: sin
         * ese rastro, un intento fallido (o una contraseña rotada) no se puede
         * demostrar después. El registro no altera la respuesta. */
        let resultado = Self::verificar_credenciales(pool, &req, jwt_secret).await;
        let motivo = Self::motivo_intento(&resultado);
        let user_id = resultado.as_ref().ok().map(|respuesta| respuesta.user_id);
        Self::anotar_intento(pool, &req.email, user_id, motivo).await;
        resultado
    }

    async fn verificar_credenciales(
        pool: &PgPool,
        req: &LoginRequest,
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

    /// Motivo corto del intento para el registro: `ok` o el fallo observado.
    fn motivo_intento(resultado: &Result<AuthResponse, AppError>) -> &'static str {
        match resultado {
            Ok(_) => "ok",
            Err(AppError::Unauthorized) => "credenciales_invalidas",
            Err(AppError::Internal(_)) => "hash_invalido",
            Err(_) => "error",
        }
    }

    /// Anota el intento sin guardar nunca la contraseña ni su hash. Un fallo al
    /// anotar no puede tumbar el acceso: se avisa y se sigue.
    async fn anotar_intento(pool: &PgPool, email: &str, user_id: Option<Uuid>, motivo: &str) {
        let email = email.trim().to_lowercase();
        let email = email.chars().take(320).collect::<String>();
        /* `rotacion` no es un acceso, pero sí un intento que salió bien. */
        let exito = matches!(motivo, "ok" | "rotacion");
        let resultado =
            UserRepository::registrar_intento_acceso(pool, user_id, &email, exito, motivo).await;
        if let Err(error) = resultado {
            tracing::warn!("No se pudo registrar el intento de acceso: {error}");
        }
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
        Self::generate_token_with_secciones(user_id, trabajador_id, Vec::new(), secret)
    }

    /* [169A-3] Variante con secciones de menú firmadas (tokens de trabajador). */
    pub fn generate_token_with_secciones(
        user_id: Uuid,
        trabajador_id: Option<Uuid>,
        secciones: Vec<String>,
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
            secs: secciones,
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

    /// [D02] Rotación de la contraseña del terminal desde dentro de la sesión:
    /// exige la actual, de modo que un token robado no basta para cambiarla.
    pub async fn cambiar_password(
        pool: &PgPool,
        user_id: Uuid,
        actual: &str,
        nueva: &str,
    ) -> Result<(), AppError> {
        let user = UserRepository::find_by_id(pool, user_id)
            .await?
            .ok_or(AppError::Unauthorized)?;
        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|e| AppError::Internal(format!("Hash almacenado inválido: {e}")))?;
        Argon2::default()
            .verify_password(actual.as_bytes(), &parsed_hash)
            .map_err(|_| AppError::Unauthorized)?;

        let salt = SaltString::generate(&mut OsRng);
        let nuevo_hash = Argon2::default()
            .hash_password(nueva.as_bytes(), &salt)
            .map_err(|e| AppError::Internal(format!("Error al hashear contraseña: {e}")))?
            .to_string();
        UserRepository::update_password(pool, user.id, &nuevo_hash).await?;

        /* La rotación queda en el mismo rastro que los accesos: así la evidencia
         * de «la vieja no entra, la nueva sí» se puede leer de una sola tabla. */
        Self::anotar_intento(pool, &user.email, Some(user.id), "rotacion").await;
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

/* [11AA-1] Cubre el salto jsonwebtoken 9→10 (backend rust_crypto) y el
 * validator 0.18→0.19 de los DTOs de auth. Unitarios: no requieren Postgres. */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::RegisterRequest;
    use validator::Validate;

    #[test]
    fn token_round_trip_conserva_claims() {
        let user_id = Uuid::new_v4();
        let trabajador_id = Uuid::new_v4();
        let token = AuthService::generate_token_with_secciones(
            user_id,
            Some(trabajador_id),
            vec!["ventas".into()],
            "secreto-de-prueba",
        )
        .expect("generar token");

        let claims =
            AuthService::verify_token(&token, "secreto-de-prueba").expect("verificar token");

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.tid, Some(trabajador_id));
        assert_eq!(claims.secs, vec!["ventas".to_string()]);
    }

    #[test]
    fn token_con_otro_secreto_se_rechaza() {
        let token = AuthService::generate_token_with_tid(Uuid::new_v4(), None, "uno")
            .expect("generar token");

        assert!(AuthService::verify_token(&token, "otro").is_err());
    }

    #[test]
    fn validator_acepta_y_rechaza_registro() {
        let valido = RegisterRequest {
            email: "a@b.com".into(),
            password: "12345678".into(),
        };
        let email_malo = RegisterRequest {
            email: "no-es-email".into(),
            password: "12345678".into(),
        };

        assert!(valido.validate().is_ok());
        assert!(email_malo.validate().is_err());
    }
}
