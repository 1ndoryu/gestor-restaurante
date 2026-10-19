/* Transporte HTTP de BDP: cliente compartido, control de destino y lectura de respuestas. */

use std::sync::LazyLock;
use std::time::Duration;

use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use tracing::warn;

use super::tipos::{BdpWeblinkClient, BdpWeblinkError};

static HTTP_CLIENT: LazyLock<Result<Client, String>> = LazyLock::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(20))
        /* [207A-1] S6-H1: Deshabilitar redirects automáticos para que
         * ensure_target_allowed() realmente controle el destino. Sin esto,
         * un 302 a un host arbitrario bypassa la allowlist. */
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| error.to_string())
});

/* Cliente HTTP compartido de BDP. `build()` solo falla si el backend TLS no puede
 * inicializarse (fallo de entorno, no de configuracion). Se propaga como error de
 * transporte en lugar de entrar en panico, y NO se degrada al cliente por defecto de
 * reqwest porque este perderia `redirect::Policy::none()` y con ello el control de
 * allowlist de `ensure_target_allowed()` ([207A-1] S6-H1). */
fn http_client() -> Result<&'static Client, BdpWeblinkError> {
    HTTP_CLIENT.as_ref().map_err(|detalle| {
        BdpWeblinkError::Http(format!("cliente HTTP BDP no disponible: {detalle}"))
    })
}

impl BdpWeblinkClient<'_> {
    pub(super) async fn post_authenticated_json<P>(
        &self,
        path: &str,
        payload: &P,
    ) -> Result<Value, BdpWeblinkError>
    where
        P: Serialize + ?Sized,
    {
        let session = self.login().await?;
        let response: Value = self
            .post_authenticated(path, payload, &session.token)
            .await?;
        if let Some(message) = response_error_message(&response) {
            return Err(BdpWeblinkError::Remote(message));
        }
        Ok(response)
    }

    pub async fn post_authenticated<T, P>(
        &self,
        path: &str,
        payload: &P,
        token: &str,
    ) -> Result<T, BdpWeblinkError>
    where
        T: DeserializeOwned,
        P: Serialize + ?Sized,
    {
        self.ensure_configured()?;
        let base_url = self.config.bdp_base_url.as_str();
        let _throttle_guard = crate::services::bdp_throttle::BDP_THROTTLE
            .acquire(base_url)
            .map_err(|reason| BdpWeblinkError::Throttled(format!("{reason} ({path})")))?;
        let url = self.build_url(path)?;
        let response = http_client()?
            .post(url)
            .bearer_auth(token)
            .json(payload)
            .send()
            .await
            .map_err(|error| BdpWeblinkError::Http(error.to_string()))?;
        decode_response(response.status(), response.text().await)
    }

    pub(super) async fn post_public<T, P>(
        &self,
        path: &str,
        payload: &P,
    ) -> Result<T, BdpWeblinkError>
    where
        T: DeserializeOwned,
        P: Serialize + ?Sized,
    {
        self.ensure_base_url()?;
        let base_url = self.config.bdp_base_url.as_str();
        let _throttle_guard = crate::services::bdp_throttle::BDP_THROTTLE
            .acquire(base_url)
            .map_err(|reason| BdpWeblinkError::Throttled(format!("{reason} ({path})")))?;
        let url = self.build_url(path)?;
        let response = http_client()?
            .post(url)
            .json(payload)
            .send()
            .await
            .map_err(|error| BdpWeblinkError::Http(error.to_string()))?;
        decode_response(response.status(), response.text().await)
    }

    pub(super) fn ensure_configured(&self) -> Result<(), BdpWeblinkError> {
        self.ensure_base_url()?;
        if self.config.bdp_login.trim().is_empty()
            || self.config.bdp_password.trim().is_empty()
            || self.config.bdp_integrator_code.trim().is_empty()
        {
            return Err(BdpWeblinkError::NotConfigured);
        }
        Ok(())
    }

    fn ensure_base_url(&self) -> Result<(), BdpWeblinkError> {
        if self.config.bdp_base_url.trim().is_empty() {
            return Err(BdpWeblinkError::NotConfigured);
        }
        Ok(())
    }

    /// Kill switch de destino para toda escritura. Loopback se permite para el
    /// simulador local; cualquier host externo debe estar autorizado de forma
    /// exacta y deliberada mediante una variable del proceso.
    pub(crate) fn ensure_write_target_allowed(&self) -> Result<(), BdpWeblinkError> {
        self.ensure_target_allowed("BDP_WRITE_ALLOWED_ORIGINS")
    }

    pub(super) fn ensure_target_allowed(&self, allowlist_env: &str) -> Result<(), BdpWeblinkError> {
        self.ensure_base_url()?;
        let base = self.config.bdp_base_url.trim().trim_end_matches('/');
        let parsed = reqwest::Url::parse(base)
            .map_err(|_| BdpWeblinkError::InvalidBaseUrl(base.to_string()))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(BdpWeblinkError::WriteTargetDenied(base.to_string()));
        }
        if !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || parsed.path() != "/"
        {
            return Err(BdpWeblinkError::WriteTargetDenied(base.to_string()));
        }
        if parsed.host_str().is_some_and(|host| {
            host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1"
        }) {
            return Ok(());
        }

        let allowed = std::env::var(allowlist_env).unwrap_or_default();
        if allowed
            .split(',')
            .map(str::trim)
            .map(|entry| entry.trim_end_matches('/'))
            .any(|entry| !entry.is_empty() && entry == base)
        {
            return Ok(());
        }
        Err(BdpWeblinkError::WriteTargetDenied(base.to_string()))
    }

    fn build_url(&self, path: &str) -> Result<String, BdpWeblinkError> {
        let base = self.config.bdp_base_url.trim().trim_end_matches('/');
        let endpoint = path.trim_start_matches('/');
        let url = format!("{base}/{endpoint}");
        reqwest::Url::parse(&url).map_err(|_| BdpWeblinkError::InvalidBaseUrl(url.clone()))?;
        Ok(url)
    }
}

fn decode_response<T>(
    status: StatusCode,
    body: Result<String, reqwest::Error>,
) -> Result<T, BdpWeblinkError>
where
    T: DeserializeOwned,
{
    let body = body.map_err(|error| BdpWeblinkError::Http(error.to_string()))?;
    if !status.is_success() {
        return Err(BdpWeblinkError::Api {
            status: status.as_u16(),
            body: sanitize_body(&body),
        });
    }

    serde_json::from_str::<T>(&body).map_err(|error| {
        warn!(
            "Respuesta BDP no parseable: {error}; body={}",
            sanitize_body(&body)
        );
        BdpWeblinkError::Http(format!("respuesta JSON invalida: {error}"))
    })
}

pub(super) fn ensure_no_remote_error(error_message: &str) -> Result<(), BdpWeblinkError> {
    let trimmed = error_message.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    Err(BdpWeblinkError::Remote(trimmed.to_string()))
}

fn sanitize_body(body: &str) -> String {
    /* [287A-4] Las respuestas BDP pueden reflejar credenciales, clientes o
     * payloads comerciales. Para errores HTTP basta conservar tamaño y no el
     * contenido; el diagnóstico detallado se hace con status y endpoint. */
    format!("cuerpo BDP omitido ({} bytes)", body.len())
}

pub fn response_error_message(value: &Value) -> Option<String> {
    value
        .get("ErrorMessage")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(ToOwned::to_owned)
}

/// Listas de errores embebidos que el BDP devuelve con HTTP 200 y
/// `ErrorMessage` vacío. Si alguna tiene entradas, la operación NO se aplicó
/// (o se aplicó parcialmente) y debe tratarse como rechazo, no como éxito.
const BDP_EMBEDDED_ERROR_LISTS: &[&str] = &["ListaErroresArticulo"];

/// [049A-1/H-W] Detecta errores embebidos del BDP en listas tipo
/// `ListaErroresArticulo` (HTTP 200 + `ErrorMessage` vacío). Devuelve `Remote`
/// con el primer error legible cuando la lista no está vacía.
pub fn ensure_embedded_errors_empty(value: &Value) -> Result<(), BdpWeblinkError> {
    for key in BDP_EMBEDDED_ERROR_LISTS {
        if let Some(list) = value.get(key).and_then(Value::as_array) {
            let mensajes: Vec<String> = list
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|message| !message.is_empty())
                .map(ToOwned::to_owned)
                .collect();
            if let Some(primero) = mensajes.first() {
                let resumen = if mensajes.len() > 1 {
                    format!("{primero} (+{} más)", mensajes.len() - 1)
                } else {
                    primero.clone()
                };
                return Err(BdpWeblinkError::Remote(resumen));
            }
        }
    }
    Ok(())
}
