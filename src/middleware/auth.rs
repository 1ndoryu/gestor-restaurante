use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use uuid::Uuid;

use crate::errors::AppError;
use crate::repositories::UserRepository;
use crate::services::AuthService;
use crate::AppState;

/// Extractor que valida el JWT del header Authorization y extrae el `user_id`.
/// [094A-3] También extrae `trabajador_id` si el token es de un trabajador.
/// [085A-1] Verifica que el `user_id` del token existe en BD → evita FK violations (500)
///          cuando el usuario fue eliminado o la BD fue reseteada. Devuelve 401 limpio.
/// Usar como parámetro en handlers que requieren autenticación.
pub struct AuthUser {
    pub user_id: Uuid,
    /* [094A-3] None = propietario, Some = trabajador con permisos restringidos */
    pub trabajador_id: Option<Uuid>,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|value| value.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?;

        let claims = AuthService::verify_token(token, &state.jwt_secret)?;

        /* Verificar que el usuario aún existe en BD.
         * Sin esto, un token válido con user_id inexistente pasa auth pero falla
         * en FK de otras tablas devolviendo 500 en vez de 401. */
        UserRepository::find_by_id(&state.pool, claims.sub)
            .await
            .map_err(|e| AppError::Internal(format!("Error verificando usuario: {e}")))?
            .ok_or(AppError::Unauthorized)?;

        Ok(Self {
            user_id: claims.sub,
            trabajador_id: claims.tid,
        })
    }
}
