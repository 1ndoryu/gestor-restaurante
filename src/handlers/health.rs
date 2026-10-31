use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use utoipa::ToSchema;

use crate::AppState;

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Endpoint de health check — siempre público
#[utoipa::path(
    get,
    path = "/api/health",
    tag = "Health",
    responses(
        (status = 200, description = "Servicio funcionando", body = HealthResponse)
    )
)]
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[derive(Serialize, ToSchema)]
pub struct EntornoResponse {
    /// Texto del aviso de entorno de pruebas; `null` = producción, sin aviso.
    pub etiqueta: Option<String>,
}

/// Etiqueta de entorno de pruebas: vacía o en blanco equivale a «apagado».
fn etiqueta_entorno(valor: Option<String>) -> Option<String> {
    valor
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Dice si esta instancia es un entorno de pruebas (público: también lo lee el login).
#[utoipa::path(
    get,
    path = "/api/entorno",
    tag = "Health",
    responses(
        (status = 200, description = "Etiqueta del entorno de pruebas, si la hay", body = EntornoResponse)
    )
)]
pub async fn entorno() -> Json<EntornoResponse> {
    Json(EntornoResponse {
        etiqueta: etiqueta_entorno(std::env::var("ENTORNO_PRUEBA_ETIQUETA").ok()),
    })
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .route("/entorno", get(entorno))
}

#[cfg(test)]
mod tests {
    use super::etiqueta_entorno;

    #[test]
    fn sin_variable_o_en_blanco_esta_apagado() {
        assert_eq!(etiqueta_entorno(None), None);
        assert_eq!(etiqueta_entorno(Some(String::new())), None);
        assert_eq!(etiqueta_entorno(Some("   ".into())), None);
    }

    #[test]
    fn con_texto_devuelve_la_etiqueta_recortada() {
        assert_eq!(
            etiqueta_entorno(Some("  PRUEBAS · BDP demo ".into())),
            Some("PRUEBAS · BDP demo".to_string())
        );
    }
}
