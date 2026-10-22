/* Transporte HTTP de BDP: cliente compartido, control de destino y lectura de respuestas. */

use std::sync::LazyLock;
use std::time::Duration;

use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use tracing::warn;

use super::tipos::{BdpWeblinkClient, BdpWeblinkError};
use crate::services::bdp_weblink_catalog::{
    BDP_PATH_ADD_ORDER_TIP, BDP_PATH_ADD_POINTS, BDP_PATH_CANCEL_ORDER, BDP_PATH_CREATE_ORDER,
    BDP_PATH_INVOICE_ORDER, BDP_PATH_ORDER_PAYMENT_ADD,
};

/* [D15] Timeout largo solo para venta/pago/puntos. Con 20 s, el Orders/Create de
 * la prueba D15 terminó como «ambiguo» (error de transporte a los ~20 s); la
 * causa exacta en BDP no se ha confirmado, solo que el cliente cortó. */
const TIMEOUT_LECTURA: Duration = Duration::from_secs(20);
const TIMEOUT_ESCRITURA: Duration = Duration::from_secs(60);

static HTTP_CLIENT: LazyLock<Result<Client, String>> =
    LazyLock::new(|| construir_cliente(TIMEOUT_LECTURA));

static HTTP_CLIENT_ESCRITURA: LazyLock<Result<Client, String>> =
    LazyLock::new(|| construir_cliente(TIMEOUT_ESCRITURA));

/* [10AA-3] `reqwest` solo dice «error sending request for url (…)»; el motivo de verdad
 * —conexión rechazada, cortada a media respuesta, timeout— va en los `source` encadenados.
 * Sin recorrerlos, un fallo del BDP en producción no dice nada y no se puede diagnosticar
 * (así se quedó el de D15 sin causa confirmada). */
fn causa_completa(error: &(dyn std::error::Error + 'static)) -> String {
    let mut texto = error.to_string();
    let mut causa = error.source();
    while let Some(actual) = causa {
        texto.push_str(": ");
        texto.push_str(&actual.to_string());
        causa = actual.source();
    }
    texto
}

fn construir_cliente(timeout: Duration) -> Result<Client, String> {
    let builder = Client::builder()
        .timeout(timeout)
        /* [207A-1] Sin redirects automáticos: un 302 desviaría una escritura a
         * un host que no es el configurado, sin dejar rastro en la auditoría. */
        .redirect(reqwest::redirect::Policy::none());
    /* [10AA-3] Las pruebas comparten este cliente (es un `LazyLock` de proceso), pero cada
     * `#[tokio::test]` trae su propio runtime. Una conexión ociosa que creó un runtime ya
     * cerrado rompe la petición del siguiente test con «dispatch task is gone: runtime
     * dropped the dispatch task»: una de cada ocho corridas fallaba en un test distinto.
     * Sin reutilizar conexiones ociosas ese cruce no puede ocurrir. Fuera de las pruebas
     * no aplica: producción tiene un solo runtime y ahí sí conviene el keep-alive. */
    #[cfg(test)]
    let builder = builder.pool_max_idle_per_host(0);
    builder.build().map_err(|error| error.to_string())
}

/* Cliente HTTP compartido de BDP. `build()` solo falla si el backend TLS no puede
 * inicializarse (fallo de entorno, no de configuracion). Se propaga como error de
 * transporte en lugar de entrar en panico, y NO se degrada al cliente por defecto de
 * reqwest porque este perderia `redirect::Policy::none()` ([207A-1] S6-H1). */
fn http_client(path: &str) -> Result<&'static Client, BdpWeblinkError> {
    let cliente = if es_escritura_venta(path) {
        &HTTP_CLIENT_ESCRITURA
    } else {
        &HTTP_CLIENT
    };
    cliente.as_ref().map_err(|detalle| {
        BdpWeblinkError::Http(format!("cliente HTTP BDP no disponible: {detalle}"))
    })
}

/// Endpoints de escritura de venta, pago y puntos: los que reciben el timeout largo.
fn es_escritura_venta(path: &str) -> bool {
    [
        BDP_PATH_CREATE_ORDER,
        BDP_PATH_ORDER_PAYMENT_ADD,
        BDP_PATH_INVOICE_ORDER,
        BDP_PATH_CANCEL_ORDER,
        BDP_PATH_ADD_ORDER_TIP,
        BDP_PATH_ADD_POINTS,
    ]
    .contains(&path)
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
        let response = http_client(path)?
            .post(url)
            .bearer_auth(token)
            .json(payload)
            .send()
            .await
            .map_err(|error| BdpWeblinkError::Http(causa_completa(&error)))?;
        decode_escritura(response.status(), response.text().await)
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
        let response = http_client(path)?
            .post(url)
            .json(payload)
            .send()
            .await
            .map_err(|error| BdpWeblinkError::Http(causa_completa(&error)))?;
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

    /// La URL debe ser un origen limpio: esquema http(s), sin credenciales, sin
    /// query, sin fragmento y sin ruta. Cualquier otro valor rompería `build_url`
    /// o desviaría la petición; el destino en sí no se restringe.
    fn ensure_base_url(&self) -> Result<(), BdpWeblinkError> {
        let base = self.config.bdp_base_url.trim().trim_end_matches('/');
        if base.is_empty() {
            return Err(BdpWeblinkError::NotConfigured);
        }
        let parsed = reqwest::Url::parse(base)
            .map_err(|_| BdpWeblinkError::InvalidBaseUrl(base.to_string()))?;
        let limpia = matches!(parsed.scheme(), "http" | "https")
            && parsed.username().is_empty()
            && parsed.password().is_none()
            && parsed.query().is_none()
            && parsed.fragment().is_none()
            && parsed.path() == "/";
        if limpia {
            Ok(())
        } else {
            Err(BdpWeblinkError::InvalidBaseUrl(base.to_string()))
        }
    }

    fn build_url(&self, path: &str) -> Result<String, BdpWeblinkError> {
        let base = self.config.bdp_base_url.trim().trim_end_matches('/');
        let endpoint = path.trim_start_matches('/');
        let url = format!("{base}/{endpoint}");
        reqwest::Url::parse(&url).map_err(|_| BdpWeblinkError::InvalidBaseUrl(url.clone()))?;
        Ok(url)
    }
}

/* [D26g] Toda escritura autenticada pasa por aquí. Si la respuesta llegó con un
 * estado pero su cuerpo no se pudo leer (corte a media respuesta), la escritura
 * puede estar aplicada: `RespuestaPerdida` lo dice, y el libro de stock decide
 * con una lectura posterior. Un rechazo de negocio llega con cuerpo y sigue
 * leyéndose como `Remote`. */
fn decode_escritura<T>(
    status: StatusCode,
    body: Result<String, reqwest::Error>,
) -> Result<T, BdpWeblinkError>
where
    T: DeserializeOwned,
{
    let body = match body {
        Ok(body) => body,
        Err(error) => {
            let motivo = causa_completa(&error);
            if status.is_success() {
                return Err(BdpWeblinkError::RespuestaPerdida(motivo));
            }
            return Err(BdpWeblinkError::Api {
                status: status.as_u16(),
                body: motivo,
            });
        }
    };
    decode_response(status, Ok(body))
}

fn decode_response<T>(
    status: StatusCode,
    body: Result<String, reqwest::Error>,
) -> Result<T, BdpWeblinkError>
where
    T: DeserializeOwned,
{
    let body = body.map_err(|error| BdpWeblinkError::Http(causa_completa(&error)))?;
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

#[cfg(test)]
mod tests_timeout {
    use super::*;

    #[test]
    fn solo_venta_pago_y_puntos_usan_timeout_largo() {
        assert!(es_escritura_venta(BDP_PATH_CREATE_ORDER));
        assert!(es_escritura_venta(BDP_PATH_ORDER_PAYMENT_ADD));
        assert!(es_escritura_venta(BDP_PATH_ADD_POINTS));
        assert!(!es_escritura_venta("/API/Orders/Get"));
        assert!(!es_escritura_venta("/API/Loyalty/GetPoints"));
    }
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
/// `ErrorList` (stock masivo) es una lista de objetos con `Article` y
/// `ErrorMessage`: BDP deja la entrada aunque el artículo sí se aplicó, así que
/// cuenta como reclamo solo si el `ErrorMessage` trae texto.
const BDP_EMBEDDED_ERROR_LISTS: &[&str] = &["ListaErroresArticulo", "ErrorList"];

/// Texto del error de una entrada de las listas embebidas: cadena suelta o
/// objeto `{Article, ErrorMessage}` del stock masivo.
fn mensaje_de_entrada(entrada: &Value) -> Option<String> {
    let texto = match entrada {
        Value::String(texto) => texto.as_str(),
        Value::Object(_) => entrada.get("ErrorMessage").and_then(Value::as_str)?,
        _ => return None,
    };
    let texto = texto.trim();
    if texto.is_empty() {
        return None;
    }
    /* Con artículo identificado, el mensaje solo («NO ES DEL TIPO WEB») no dice
     * de qué línea es: sin el código, el operador no puede corregir el lote. */
    match entrada.get("Article").and_then(Value::as_i64) {
        Some(article) => Some(format!("artículo {article}: {texto}")),
        None => Some(texto.to_string()),
    }
}

/// [049A-1/H-W] Detecta errores embebidos del BDP en listas tipo
/// `ListaErroresArticulo` (HTTP 200 + `ErrorMessage` vacío). Devuelve `Remote`
/// con el primer error legible cuando la lista no está vacía.
pub fn ensure_embedded_errors_empty(value: &Value) -> Result<(), BdpWeblinkError> {
    for key in BDP_EMBEDDED_ERROR_LISTS {
        if let Some(list) = value.get(key).and_then(Value::as_array) {
            let mensajes: Vec<String> = list.iter().filter_map(mensaje_de_entrada).collect();
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
