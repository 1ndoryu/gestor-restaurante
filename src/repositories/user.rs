use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::User;

pub struct UserRepository;

impl UserRepository {
    /// Crea un usuario y retorna el registro completo
    pub async fn create(
        pool: &PgPool,
        email: &str,
        password_hash: &str,
    ) -> Result<User, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            User,
            "INSERT INTO users (id, email, password_hash) \
             VALUES ($1, $2, $3) \
             RETURNING id, email, password_hash, created_at",
            id,
            email,
            password_hash
        )
        .fetch_one(pool)
        .await
    }

    /// Busca un usuario por email
    pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            "SELECT id, email, password_hash, created_at FROM users WHERE email = $1",
            email
        )
        .fetch_optional(pool)
        .await
    }

    /// Busca un usuario por ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            "SELECT id, email, password_hash, created_at FROM users WHERE id = $1",
            id
        )
        .fetch_optional(pool)
        .await
    }

    /* [263A-15] Métodos para recuperación de contraseña */

    /// Guarda un token de reset con expiración
    pub async fn set_reset_token(
        pool: &PgPool,
        email: &str,
        token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            "UPDATE users SET reset_token = $1, reset_token_expires_at = $2 WHERE email = $3",
            token,
            expires_at,
            email
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Busca un usuario por token de reset válido (no expirado)
    pub async fn find_by_reset_token(
        pool: &PgPool,
        token: &str,
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            "SELECT id, email, password_hash, created_at FROM users \
             WHERE reset_token = $1 AND reset_token_expires_at > NOW()",
            token
        )
        .fetch_optional(pool)
        .await
    }

    /// Actualiza la contraseña y limpia el token de reset
    pub async fn update_password(
        pool: &PgPool,
        id: Uuid,
        new_hash: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            "UPDATE users SET password_hash = $1, reset_token = NULL, reset_token_expires_at = NULL WHERE id = $2",
            new_hash,
            id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /* [D02] Rastro de accesos al terminal: sin contraseña ni hash, solo quién,
     * cuándo, si entró y por qué no. */

    /// Anota un intento de acceso (o una rotación de contraseña).
    pub async fn registrar_intento_acceso(
        pool: &PgPool,
        user_id: Option<Uuid>,
        email: &str,
        exito: bool,
        motivo: &str,
    ) -> Result<(), sqlx::Error> {
        /* [D02] Consulta en runtime contra la tabla nueva: la macro exige la
         * cache offline `.sqlx`, que necesita `cargo sqlx prepare` contra una
         * base migrada. El proyecto ya usa runtime queries para tablas recién
         * añadidas (ver Agente/documentacion/api/bdp-integration-status). */
        // sentinel-disable-next-line sqlx-query-sin-macro: tabla nueva `auth_intentos`; la macro exigiría regenerar la cache offline y rompería el build de Docker.
        sqlx::query(
            "INSERT INTO auth_intentos (user_id, email, exito, motivo)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(user_id)
        .bind(email)
        .bind(exito)
        .bind(motivo)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Los intentos más recientes, del más nuevo al más viejo.
    pub async fn listar_intentos_acceso(
        pool: &PgPool,
        limite: i64,
    ) -> Result<Vec<IntentoAcceso>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro: misma tabla nueva `auth_intentos`; con la cache offline sin regenerar, el build de Docker (SQLX_OFFLINE=true) no compila.
        sqlx::query_as::<_, IntentoAcceso>(
            "SELECT created_at, email, exito, motivo
             FROM auth_intentos
             ORDER BY created_at DESC
             LIMIT $1",
        )
        .bind(limite)
        .fetch_all(pool)
        .await
    }
}

/// Una fila del registro de intentos de acceso.
#[derive(Debug, sqlx::FromRow)]
pub struct IntentoAcceso {
    pub created_at: DateTime<Utc>,
    pub email: String,
    pub exito: bool,
    pub motivo: String,
}
