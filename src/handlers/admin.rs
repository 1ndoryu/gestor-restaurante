/* [283A-39] Handlers de administración para datos de prueba.
 * POST /api/admin/seed  — ejecuta el seed (recarga todos los datos demo).
 * POST /api/admin/reset — elimina todos los datos del usuario (sin borrar cuenta).
 * [303A-2] Requiere DEMO_MODE=true env var para estar habilitado.
 * Ambos requieren autenticación. Solo para entornos de demo. */

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::Serialize;
use utoipa::ToSchema;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::repositories::AdminRepository;
use crate::AppState;

#[derive(Serialize, ToSchema)]
pub struct AdminResult {
    pub ok: bool,
    pub mensaje: String,
}

/// Verificar que `DEMO_MODE` está habilitado
fn verificar_demo_mode() -> Result<(), AppError> {
    let demo = std::env::var("DEMO_MODE").unwrap_or_default();
    if demo != "true" && demo != "1" {
        return Err(AppError::BadRequest(
            "Los endpoints de administración solo están disponibles en modo demo (DEMO_MODE=true)"
                .to_string(),
        ));
    }
    Ok(())
}

/// Ejecutar seed — recarga todos los datos de prueba del usuario demo
#[utoipa::path(
    post,
    path = "/api/admin/seed",
    tag = "Admin",
    responses(
        (status = 200, description = "Seed ejecutado", body = AdminResult),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn ejecutar_seed(
    State(_state): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<AdminResult>, AppError> {
    verificar_demo_mode()?;
    let output = std::process::Command::new("/app/seed")
        .output()
        .map_err(|e| AppError::Internal(format!("Error ejecutando seed: {e}")))?;

    if output.status.success() {
        Ok(Json(AdminResult {
            ok: true,
            mensaje: "Datos de prueba cargados exitosamente.".to_string(),
        }))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(AppError::Internal(format!("Seed falló: {stderr}")))
    }
}

/// Eliminar todos los datos del usuario (sin borrar la cuenta)
#[utoipa::path(
    post,
    path = "/api/admin/reset",
    tag = "Admin",
    responses(
        (status = 200, description = "Datos eliminados", body = AdminResult),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn eliminar_datos(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<AdminResult>, AppError> {
    verificar_demo_mode()?;
    /* [267A-7] El orden FK vive en el repositorio; aquí solo puerta demo + delegación. */
    AdminRepository::eliminar_datos_usuario(&state.pool, auth.user_id)
        .await
        .map_err(AppError::Database)?;
    Ok(Json(AdminResult {
        ok: true,
        mensaje: "Todos los datos de prueba han sido eliminados.".to_string(),
    }))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/seed", post(ejecutar_seed))
        .route("/admin/reset", post(eliminar_datos))
}
