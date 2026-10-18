/* Tipos publicos de BDP WebLink: errores, respuestas y cliente con su cache de sesion. */

use serde::Deserialize;

use crate::models::ConfiguracionRestaurante;

#[derive(Debug, thiserror::Error)]
pub enum BdpWeblinkError {
    #[error("BDP no esta configurado")]
    NotConfigured,
    #[error("URL BDP invalida: {0}")]
    InvalidBaseUrl(String),
    #[error("Error HTTP BDP: {0}")]
    Http(String),
    #[error("BDP respondio HTTP {status}: {body}")]
    Api { status: u16, body: String },
    #[error("BDP devolvio error: {0}")]
    Remote(String),
    #[error("Escritura BDP bloqueada: destino no incluido en BDP_WRITE_ALLOWED_ORIGINS: {0}")]
    WriteTargetDenied(String),
    #[error("BDP throttled: {0}")]
    Throttled(String),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpHealthResponse {
    pub is_alive: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpAuthSession {
    pub token: String,
    #[serde(rename = "ExpiresIn_InSecconds", alias = "ExpiresIN_InSecconds")]
    pub expires_in_seconds: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpLoginResponse {
    #[serde(default)]
    pub error_message: String,
    pub auth_session: Option<BdpAuthSession>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpVersionResponse {
    #[serde(default)]
    pub version: i32,
    #[serde(default, alias = "Subversion")]
    pub sub_version: i32,
    #[serde(default)]
    pub revision: String,
    #[serde(default)]
    pub application: String,
    #[serde(default)]
    pub application_description: String,
    #[serde(default)]
    pub error_message: String,
}

pub struct BdpWeblinkClient<'a> {
    pub(super) config: &'a ConfiguracionRestaurante,
    /* [AUDIT-N4] Cache de sesión BDP para evitar login redundante.
     * Cada llamada a post_authenticated_json() hacia login() internamente.
     * Con caché, un handler que hace N llamadas BDP solo hace 1 login. */
    pub(super) cached_session: std::sync::Mutex<Option<(BdpAuthSession, std::time::Instant)>>,
}
