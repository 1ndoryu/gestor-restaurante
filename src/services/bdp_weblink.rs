/* [065A-2] Cliente base para BDP WebLink REST API.
 * Centraliza login, headers y manejo de ErrorMessage para que la integracion
 * posterior de articulos/clientes/comandas no replique detalles de transporte.
 * Gotcha: el manual no explicita el header del token; se encapsula aqui para
 * ajustar una sola pieza durante la prueba remota si BDP usa otro nombre. */

use std::sync::LazyLock;
use std::time::Duration;

use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::warn;

use crate::models::ConfiguracionRestaurante;
use crate::services::bdp_weblink_catalog::{
    BdpAddOrderPaymentRequest, BdpCancelOrderRequest, BdpCreateCustomerRequest,
    BdpCreateOrderRequest, BdpDepartmentsExportFromProfileRequest, BdpEmptyRequest,
    BdpExportArticlesRequest, BdpExportCustomersRequest, BdpExportDepartmentsRequest,
    BdpGetEmployeeRequest, BdpGetEmployeesRequest, BdpGetOrderRequest, BdpGetPosArticlesRequest,
    BdpGetPosEmployeesRequest, BdpGetPosRequest, BdpGetPosTendersRequest, BdpInvoiceOrderRequest,
    BDP_PATH_CANCEL_ORDER, BDP_PATH_CREATE_CUSTOMER, BDP_PATH_CREATE_ORDER,
    BDP_PATH_EXPORT_ARTICLES, BDP_PATH_EXPORT_CUSTOMERS, BDP_PATH_EXPORT_DEPARTMENTS,
    BDP_PATH_EXPORT_DEPARTMENTS_FROM_PROFILE, BDP_PATH_GET_EMPLOYEE, BDP_PATH_GET_EMPLOYEES,
    BDP_PATH_GET_ORDER, BDP_PATH_GET_POS, BDP_PATH_GET_POSES, BDP_PATH_GET_POS_ARTICLES,
    BDP_PATH_GET_POS_EMPLOYEES, BDP_PATH_GET_POS_TENDERS, BDP_PATH_GET_TENDERS,
    BDP_PATH_INVOICE_ORDER, BDP_PATH_ORDER_PAYMENT_ADD,
};

const BDP_SESSION_MINUTES: u8 = 59;

static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .expect("BDP HTTP client must be buildable")
});

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

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct BdpLoginRequest<'a> {
    login: &'a str,
    password: &'a str,
    tiempo_session: u8,
    codigo_integrador: &'a str,
}

pub struct BdpWeblinkClient<'a> {
    config: &'a ConfiguracionRestaurante,
}

impl<'a> BdpWeblinkClient<'a> {
    #[must_use]
    pub const fn new(config: &'a ConfiguracionRestaurante) -> Self {
        Self { config }
    }

    pub async fn health(&self) -> Result<BdpHealthResponse, BdpWeblinkError> {
        self.post_public("/Service/Health", &serde_json::json!({}))
            .await
    }

    pub async fn login(&self) -> Result<BdpAuthSession, BdpWeblinkError> {
        self.ensure_configured()?;

        let payload = BdpLoginRequest {
            login: &self.config.bdp_login,
            password: &self.config.bdp_password,
            tiempo_session: BDP_SESSION_MINUTES,
            codigo_integrador: &self.config.bdp_integrator_code,
        };

        let response: BdpLoginResponse = self.post_public("/Auth/Login", &payload).await?;
        ensure_no_remote_error(&response.error_message)?;
        response.auth_session.ok_or_else(|| {
            BdpWeblinkError::Remote("BDP no devolvio AuthSession en Login".to_string())
        })
    }

    pub async fn get_version(&self) -> Result<BdpVersionResponse, BdpWeblinkError> {
        let session = self.login().await?;
        let response: BdpVersionResponse = self
            .post_authenticated(
                "/Service/GetVersion",
                &serde_json::json!({}),
                &session.token,
            )
            .await?;
        ensure_no_remote_error(&response.error_message)?;
        Ok(response)
    }

    pub async fn export_articles(
        &self,
        request: &BdpExportArticlesRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_EXPORT_ARTICLES, request)
            .await
    }

    pub async fn get_pos_articles(
        &self,
        request: &BdpGetPosArticlesRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_GET_POS_ARTICLES, request)
            .await
    }

    pub async fn export_customers(
        &self,
        request: &BdpExportCustomersRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_EXPORT_CUSTOMERS, request)
            .await
    }

    pub async fn create_customer(
        &self,
        request: &BdpCreateCustomerRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_CREATE_CUSTOMER, request)
            .await
    }

    pub async fn create_order(
        &self,
        request: &BdpCreateOrderRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_CREATE_ORDER, request)
            .await
    }

    pub async fn check_order(
        &self,
        request: &BdpCreateOrderRequest,
    ) -> Result<Value, BdpWeblinkError> {
        let mut request = request.clone();
        request.order_operation_type = 1;
        self.post_authenticated_json(BDP_PATH_CREATE_ORDER, &request)
            .await
    }

    pub async fn get_order(&self, request: &BdpGetOrderRequest) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_GET_ORDER, request)
            .await
    }

    pub async fn cancel_order(
        &self,
        request: &BdpCancelOrderRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_CANCEL_ORDER, request)
            .await
    }

    pub async fn add_order_payment(
        &self,
        request: &BdpAddOrderPaymentRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_ORDER_PAYMENT_ADD, request)
            .await
    }

    pub async fn invoice_order(
        &self,
        request: &BdpInvoiceOrderRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_INVOICE_ORDER, request)
            .await
    }

    pub async fn export_departments(
        &self,
        request: &BdpExportDepartmentsRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_EXPORT_DEPARTMENTS, request)
            .await
    }

    pub async fn export_departments_from_profile(
        &self,
        request: &BdpDepartmentsExportFromProfileRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_EXPORT_DEPARTMENTS_FROM_PROFILE, request)
            .await
    }

    pub async fn get_pos(&self, request: &BdpGetPosRequest) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_GET_POS, request)
            .await
    }

    pub async fn get_poses(&self) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_GET_POSES, &BdpEmptyRequest)
            .await
    }

    pub async fn get_employee(
        &self,
        request: &BdpGetEmployeeRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_GET_EMPLOYEE, request)
            .await
    }

    pub async fn get_employees(
        &self,
        request: &BdpGetEmployeesRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_GET_EMPLOYEES, request)
            .await
    }

    pub async fn get_pos_employees(
        &self,
        request: &BdpGetPosEmployeesRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_GET_POS_EMPLOYEES, request)
            .await
    }

    pub async fn get_tenders(&self) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_GET_TENDERS, &BdpEmptyRequest)
            .await
    }

    pub async fn get_pos_tenders(
        &self,
        request: &BdpGetPosTendersRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_GET_POS_TENDERS, request)
            .await
    }

    async fn post_authenticated_json<P>(
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
        let url = self.build_url(path)?;
        let response = HTTP_CLIENT
            .post(url)
            .bearer_auth(token)
            .json(payload)
            .send()
            .await
            .map_err(|error| BdpWeblinkError::Http(error.to_string()))?;
        decode_response(response.status(), response.text().await)
    }

    async fn post_public<T, P>(&self, path: &str, payload: &P) -> Result<T, BdpWeblinkError>
    where
        T: DeserializeOwned,
        P: Serialize + ?Sized,
    {
        self.ensure_base_url()?;
        let url = self.build_url(path)?;
        let response = HTTP_CLIENT
            .post(url)
            .json(payload)
            .send()
            .await
            .map_err(|error| BdpWeblinkError::Http(error.to_string()))?;
        decode_response(response.status(), response.text().await)
    }

    fn ensure_configured(&self) -> Result<(), BdpWeblinkError> {
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

fn ensure_no_remote_error(error_message: &str) -> Result<(), BdpWeblinkError> {
    let trimmed = error_message.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    Err(BdpWeblinkError::Remote(trimmed.to_string()))
}

fn sanitize_body(body: &str) -> String {
    body.chars().take(500).collect()
}

pub fn response_error_message(value: &Value) -> Option<String> {
    value
        .get("ErrorMessage")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveTime, Utc};
    use rust_decimal::Decimal;
    use uuid::Uuid;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn config(base_url: String) -> ConfiguracionRestaurante {
        ConfiguracionRestaurante {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            reserva_email_obligatorio: false,
            reserva_telefono_obligatorio: true,
            reserva_nombre_obligatorio: true,
            reserva_apellidos_obligatorio: false,
            iva_por_defecto: Decimal::new(10, 0),
            nombre_restaurante: "Nakomi".to_string(),
            groq_api_key: None,
            auto_venta_reserva: true,
            hora_desayuno_inicio: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            hora_desayuno_fin: NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
            hora_comida_inicio: NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
            hora_comida_fin: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
            hora_cena_inicio: NaiveTime::from_hms_opt(20, 0, 0).unwrap(),
            hora_cena_fin: NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
            url_haddock: String::new(),
            haddock_api_token: String::new(),
            haddock_sync_enabled: false,
            bdp_base_url: base_url,
            bdp_login: "usuario".to_string(),
            bdp_password: "secreto".to_string(),
            bdp_integrator_code: "INTEGRADOR".to_string(),
            bdp_sync_enabled: true,
            bdp_pos_id: 1,
            bdp_employee_id: 1,
            bdp_items_profile_id: 1,
            bdp_default_article_code: String::new(),
            bdp_default_article_name: String::new(),
            bdp_tender_map: serde_json::json!({}),
            bdp_order_type_map: serde_json::json!({}),
            bdp_default_customer_code: String::new(),
            bdp_poll_interval_secs: 60,
            google_review_url: String::new(),
            telefono_restaurante: String::new(),
            url_reservas: String::new(),
            bdp_auto_sync_customers: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn health_posts_to_service_health() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/Service/Health"))
            .and(body_json(serde_json::json!({})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "IsAlive": true
            })))
            .mount(&server)
            .await;

        let config = config(server.uri());
        let client = BdpWeblinkClient::new(&config);
        let health = client.health().await.unwrap();

        assert!(health.is_alive);
    }

    #[tokio::test]
    async fn login_uses_pascal_case_payload() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/Auth/Login"))
            .and(body_json(serde_json::json!({
                "Login": "usuario",
                "Password": "secreto",
                "TiempoSession": 59,
                "CodigoIntegrador": "INTEGRADOR"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ErrorMessage": "",
                "AuthSession": {
                    "Token": "token-bdp",
                    "ExpiresIn_InSecconds": 3540
                }
            })))
            .mount(&server)
            .await;

        let config = config(server.uri());
        let client = BdpWeblinkClient::new(&config);
        let session = client.login().await.unwrap();

        assert_eq!(session.token, "token-bdp");
        assert_eq!(session.expires_in_seconds, 3540);
    }

    #[tokio::test]
    async fn authenticated_calls_use_bearer_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/API/Tenders/GetList"))
            .and(header("authorization", "Bearer token-bdp"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ErrorMessage": "",
                "TenderList": []
            })))
            .mount(&server)
            .await;

        let config = config(server.uri());
        let client = BdpWeblinkClient::new(&config);
        let response: Value = client
            .post_authenticated("/API/Tenders/GetList", &serde_json::json!({}), "token-bdp")
            .await
            .unwrap();

        assert_eq!(response_error_message(&response), None);
    }

    #[tokio::test]
    async fn export_articles_logs_in_and_posts_catalog_request() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/Auth/Login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ErrorMessage": "",
                "AuthSession": {
                    "Token": "token-bdp",
                    "ExpiresIn_InSecconds": 3540
                }
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/API/Articles/Export"))
            .and(header("authorization", "Bearer token-bdp"))
            .and(body_json(serde_json::json!({
                "Dept1": 1,
                "Dept2": 999,
                "Art1": 1,
                "Art2": 9999999999999_i64,
                "Modified": false,
                "TypePrice": 1,
                "Disc": 0
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ErrorMessage": "",
                "Articles": []
            })))
            .mount(&server)
            .await;

        let config = config(server.uri());
        let client = BdpWeblinkClient::new(&config);
        let response = client
            .export_articles(&BdpExportArticlesRequest::all_web_articles(1))
            .await
            .unwrap();

        assert!(response["Articles"].is_array());
    }

    #[tokio::test]
    async fn check_order_forces_only_check_mode() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/Auth/Login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ErrorMessage": "",
                "AuthSession": {
                    "Token": "token-bdp",
                    "ExpiresIn_InSecconds": 3540
                }
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/API/Orders/Create"))
            .and(header("authorization", "Bearer token-bdp"))
            .and(body_json(serde_json::json!({
                "EmployeeId": 1,
                "ItemsProfileId": 1,
                "OrderEndType": 0,
                "OrderOperationType": 1,
                "Order": { "Items": [] }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ErrorMessage": ""
            })))
            .mount(&server)
            .await;

        let config = config(server.uri());
        let client = BdpWeblinkClient::new(&config);
        let response = client
            .check_order(&BdpCreateOrderRequest {
                employee_id: 1,
                items_profile_id: 1,
                order_end_type: 0,
                order_operation_type: 0,
                invoice: None,
                order: serde_json::json!({ "Items": [] }),
            })
            .await
            .unwrap();

        assert_eq!(response_error_message(&response), None);
    }

    #[test]
    fn response_error_message_extracts_non_empty_bdp_errors() {
        let value = serde_json::json!({ "ErrorMessage": " [300041]-BDP error " });

        assert_eq!(
            response_error_message(&value),
            Some("[300041]-BDP error".to_string())
        );
    }
}
