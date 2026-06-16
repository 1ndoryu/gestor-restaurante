use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::UserRole;
use crate::services::AuthService;
use crate::AppState;

/* [044A-38] AuthUser extendido con role y effective_role del JWT.
 * effective_role determina qué panel/permisos tiene el usuario en la sesión actual.
 * Para admins, puede ser diferente de role si usan "cambiar rol".
 * [084A-1] impersonator: si Some, es UUID del admin que inició impersonación.
 * En ese caso user_id es el usuario impersonado y role es su rol real. */
pub struct AuthUser {
    pub user_id: Uuid,
    pub role: UserRole,
    pub effective_role: UserRole,
    pub impersonator: Option<Uuid>,
}

impl AuthUser {
    /// Verifica que el `effective_role` sea uno de los roles permitidos
    pub fn require_role(&self, allowed: &[UserRole]) -> Result<(), AppError> {
        if allowed.contains(&self.effective_role) {
            Ok(())
        } else {
            Err(AppError::Forbidden(
                "No tienes permisos para esta acción".into(),
            ))
        }
    }
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

        Ok(Self {
            user_id: claims.sub,
            role: claims.role,
            effective_role: claims.effective_role,
            impersonator: claims.impersonator,
        })
    }
}
