use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::UserRole;
use crate::repositories::UserRepository;
use crate::services::AuthService;
use crate::AppState;

/* [044A-38] AuthUser extendido con role y effective_role del JWT.
 * effective_role determina qué panel/permisos tiene el usuario en la sesión actual.
 * Para admins, puede ser diferente de role si usan "cambiar rol".
 * [084A-1] impersonator: si Some, es UUID del admin que inició impersonación.
 * En ese caso user_id es el usuario impersonado y role es su rol real.
 * [cargo-fix] trabajador_id wired from claims.tid for trabajadores handler. */
pub struct AuthUser {
    pub user_id: Uuid,
    pub role: UserRole,
    pub effective_role: UserRole,
    pub impersonator: Option<Uuid>,
    pub trabajador_id: Option<Uuid>,
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

        /* [2026-07-20] Verificar que el usuario del JWT existe en la DB.
         * Sin esto, un JWT válido de un usuario eliminado o de otro entorno
         * generaría errores FK en obtener_o_crear u otros endpoints. */
        let exists = UserRepository::find_by_id(&state.pool, claims.sub)
            .await
            .map_err(|e| AppError::Internal(format!("Error verificando usuario: {e}")))?
            .is_some();

        if !exists {
            return Err(AppError::Unauthorized);
        }

        Ok(Self {
            user_id: claims.sub,
            role: claims.role,
            effective_role: claims.effective_role,
            impersonator: claims.impersonator,
            trabajador_id: claims.tid,
        })
    }
}
