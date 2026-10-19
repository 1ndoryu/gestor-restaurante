/* Cliente BDP: sesion (health, login, version) y consultas y pedidos. */

use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use tracing::warn;

use crate::models::ConfiguracionRestaurante;
use crate::services::bdp_weblink_catalog::{
    parse_articles_value, BdpAddOrderPaymentRequest, BdpCancelOrderRequest,
    BdpCreateCustomerRequest, BdpCreateOrderRequest, BdpDepartmentsExportFromProfileRequest,
    BdpEmptyRequest, BdpExportArticleItem, BdpExportArticlesRequest, BdpExportCustomersRequest,
    BdpExportDepartmentsRequest, BdpExportPurchaseNotesRequest, BdpGetApplicationVersionRequest,
    BdpGetArticleRequest, BdpGetEmployeeRequest, BdpGetEmployeesRequest, BdpGetFastfoodRequest,
    BdpGetListStockRequest, BdpGetMenuRequest, BdpGetOrderRequest, BdpGetPackRequest,
    BdpGetPosArticlesRequest, BdpGetPosEmployeesRequest, BdpGetPosRequest, BdpGetPosTendersRequest,
    BdpGetPricesArticlesRequest, BdpGetRoomTablesRequest, BdpGetRoomsTablesRequest,
    BdpGetStockRequest, BdpInvoiceOrderRequest, BDP_PATH_CANCEL_ORDER, BDP_PATH_CREATE_CUSTOMER,
    BDP_PATH_CREATE_ORDER, BDP_PATH_EXPORT_ARTICLES, BDP_PATH_EXPORT_CUSTOMERS,
    BDP_PATH_EXPORT_DEPARTMENTS, BDP_PATH_EXPORT_DEPARTMENTS_FROM_PROFILE,
    BDP_PATH_EXPORT_PURCHASE_NOTES, BDP_PATH_GET_APPLICATION_VERSION, BDP_PATH_GET_ARTICLE,
    BDP_PATH_GET_EMPLOYEE, BDP_PATH_GET_EMPLOYEES, BDP_PATH_GET_FASTFOOD, BDP_PATH_GET_LIST_STOCK,
    BDP_PATH_GET_MENU, BDP_PATH_GET_ORDER, BDP_PATH_GET_PACK, BDP_PATH_GET_POS, BDP_PATH_GET_POSES,
    BDP_PATH_GET_POS_ARTICLES, BDP_PATH_GET_POS_EMPLOYEES, BDP_PATH_GET_POS_TENDERS,
    BDP_PATH_GET_PRICES_ARTICLES, BDP_PATH_GET_ROOMS_TABLES, BDP_PATH_GET_ROOM_TABLES,
    BDP_PATH_GET_STOCK, BDP_PATH_GET_TENDERS, BDP_PATH_INVOICE_ORDER, BDP_PATH_ORDER_PAYMENT_ADD,
    BDP_PATH_PROFILES_CREATE_ARTICLE_LIST, BDP_PATH_PROFILES_CREATE_DEPARTMENT_LIST,
    BDP_PATH_PROFILES_MODIFY_ARTICLE_LIST,
};

use super::tipos::{
    BdpAuthSession, BdpHealthResponse, BdpLoginResponse, BdpVersionResponse, BdpWeblinkClient,
    BdpWeblinkError,
};
use super::transporte::{ensure_no_remote_error, response_error_message};

const BDP_SESSION_MINUTES: u8 = 59;

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct BdpLoginRequest<'a> {
    login: &'a str,
    password: &'a str,
    tiempo_session: u8,
    codigo_integrador: &'a str,
}

impl<'a> BdpWeblinkClient<'a> {
    #[must_use]
    pub fn new(config: &'a ConfiguracionRestaurante) -> Self {
        Self {
            config,
            cached_session: std::sync::Mutex::new(None),
        }
    }

    pub async fn health(&self) -> Result<BdpHealthResponse, BdpWeblinkError> {
        self.post_public("/Service/Health", &serde_json::json!({}))
            .await
    }

    pub async fn login(&self) -> Result<BdpAuthSession, BdpWeblinkError> {
        /* [AUDIT-N4] Verificar caché antes de hacer HTTP. El token BDP dura
         * BDP_SESSION_MINUTES (59 min). Usamos 55 min como margen seguro. */
        {
            let cache = self.cached_session.lock();
            let cache = match cache {
                Ok(c) => c,
                Err(poisoned) => {
                    warn!("[R8] cached_session mutex poisoned; recuperando lock.");
                    poisoned.into_inner()
                }
            };
            if let Some((ref session, cached_at)) = *cache {
                if cached_at.elapsed() < Duration::from_mins(55) {
                    return Ok(BdpAuthSession {
                        token: session.token.clone(),
                        expires_in_seconds: session.expires_in_seconds,
                    });
                }
            }
        }

        self.ensure_configured()?;

        let payload = BdpLoginRequest {
            login: &self.config.bdp_login,
            password: &self.config.bdp_password,
            tiempo_session: BDP_SESSION_MINUTES,
            codigo_integrador: &self.config.bdp_integrator_code,
        };

        let response: BdpLoginResponse = self.post_public("/Auth/Login", &payload).await?;
        ensure_no_remote_error(&response.error_message)?;
        let session = response.auth_session.ok_or_else(|| {
            BdpWeblinkError::Remote("BDP no devolvio AuthSession en Login".to_string())
        })?;

        /* Almacenar en caché */
        {
            let cache = self.cached_session.lock();
            let mut cache = match cache {
                Ok(c) => c,
                Err(poisoned) => {
                    warn!("[R8] cached_session mutex poisoned; recuperando lock.");
                    poisoned.into_inner()
                }
            };
            *cache = Some((
                BdpAuthSession {
                    token: session.token.clone(),
                    expires_in_seconds: session.expires_in_seconds,
                },
                std::time::Instant::now(),
            ));
        }

        Ok(session)
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

    /* [039A-1/H-Q1-03] Lista completa de artículos del perfil de items vía
     * GetPOSList, paginando hasta página corta. Es exactamente el universo
     * que CreateOrder/items_profile_id puede referenciar: ExportArticles
     * devuelve vacío en BDP reales sin artículos web contratados, así que
     * esta lectura es el fallback honesto del import de catálogo. */
    pub async fn list_profile_articles(
        &self,
    ) -> Result<Vec<BdpExportArticleItem>, BdpWeblinkError> {
        /* Mismo tamaño de página que el preflight (BDP_DRY_RUN_PAGE_SIZE=10):
         * es la llamada GetPOSList probada contra el BDP real. Página mayor
         * devolvió vacío en la primera validación, así que se replica el
         * contrato conocido. */
        const PAGE_SIZE: i32 = 10;
        const MAX_PAGES: i32 = 1000;
        let mut items: Vec<BdpExportArticleItem> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for page in 1..=MAX_PAGES {
            let mut request =
                BdpGetPosArticlesRequest::first_page(self.config.bdp_items_profile_id, PAGE_SIZE);
            request.actual_page = page;
            let raw = self.get_pos_articles(&request).await?;
            let page_items = parse_articles_value(&raw);
            let total_en_pagina = page_items.len();
            let mut nuevos = 0usize;
            for item in page_items {
                let Some(code) = item.art_code() else {
                    continue;
                };
                if seen.insert(code.to_string()) {
                    nuevos += 1;
                    items.push(item);
                }
            }
            /* Página corta (o sin ítems nuevos: fin del catálogo). */
            if nuevos == 0 || total_en_pagina < PAGE_SIZE as usize {
                break;
            }
        }
        Ok(items)
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
        self.ensure_write_target_allowed()?;
        self.post_authenticated_json(BDP_PATH_CREATE_CUSTOMER, request)
            .await
    }

    pub async fn create_order(
        &self,
        request: &BdpCreateOrderRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.ensure_write_target_allowed()?;
        self.post_authenticated_json(BDP_PATH_CREATE_ORDER, request)
            .await
    }

    pub async fn check_order(
        &self,
        request: &BdpCreateOrderRequest,
    ) -> Result<Value, BdpWeblinkError> {
        /* [187A-1] OnlyCheck comparte el endpoint de creación. Hasta que la
         * versión real demuestre que no persiste, queda denegado externamente
         * por defecto y usa una allowlist independiente de las escrituras. */
        self.ensure_target_allowed("BDP_CHECK_ORDER_ALLOWED_ORIGINS")?;
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
        self.ensure_write_target_allowed()?;
        self.post_authenticated_json(BDP_PATH_CANCEL_ORDER, request)
            .await
    }

    pub async fn add_order_payment(
        &self,
        request: &BdpAddOrderPaymentRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.ensure_write_target_allowed()?;
        self.post_authenticated_json(BDP_PATH_ORDER_PAYMENT_ADD, request)
            .await
    }

    pub async fn invoice_order(
        &self,
        request: &BdpInvoiceOrderRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.ensure_write_target_allowed()?;
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
        self.post_authenticated_json(BDP_PATH_GET_POSES, &BdpEmptyRequest {})
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
        self.post_authenticated_json(BDP_PATH_GET_TENDERS, &BdpEmptyRequest {})
            .await
    }

    pub async fn get_pos_tenders(
        &self,
        request: &BdpGetPosTendersRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_GET_POS_TENDERS, request)
            .await
    }

    /* [157A-9] F9.2: consulta individual de artículo por código.
     * Devuelve ArticleData con campos extensos (precios, IVA, combinados, etc.). */
    pub async fn get_article(
        &self,
        request: &BdpGetArticleRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_GET_ARTICLE, request)
            .await
    }

    /* [157A-9] F9.3: precios de venta (1-5) y descuentos de un artículo. */
    pub async fn get_prices_articles(
        &self,
        request: &BdpGetPricesArticlesRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_GET_PRICES_ARTICLES, request)
            .await
    }

    /* [128A-1/F3] N6: stock de un artículo en un almacén concreto.
     * Path especulativo (/API/Warehouse/GetStock) — sin bloqueo standalone. */
    pub async fn get_stock(&self, request: &BdpGetStockRequest) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_GET_STOCK, request)
            .await
    }

    /* [128A-1/F3] N6: stock de varios artículos en un almacén concreto.
     * Path especulativo (/API/Warehouse/GetListStock) — sin bloqueo standalone. */
    pub async fn get_list_stock(
        &self,
        request: &BdpGetListStockRequest,
    ) -> Result<Value, BdpWeblinkError> {
        let session = self.login().await?;
        let response: Value = self
            .post_authenticated(BDP_PATH_GET_LIST_STOCK, request, &session.token)
            .await?;
        /* [D26b] Un artículo inexistente marca el ErrorMessage global (200204) aunque
         * el resto de líneas sean válidas. Solo se rechaza el lote si no llega ninguna
         * línea; los errores de cada línea se leen en BdpListStockItemResponse. */
        let con_lineas = response
            .get("Stock")
            .and_then(Value::as_array)
            .is_some_and(|lineas| !lineas.is_empty());
        if !con_lineas {
            if let Some(message) = response_error_message(&response) {
                return Err(BdpWeblinkError::Remote(message));
            }
        }
        Ok(response)
    }

    /* [157A-9] F9.4: mesas configuradas de un salón concreto. */
    pub async fn get_room_tables(
        &self,
        request: &BdpGetRoomTablesRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_GET_ROOM_TABLES, request)
            .await
    }

    /* [157A-9] F9.4: todos los salones con sus mesas. */
    pub async fn get_rooms_tables(
        &self,
        request: &BdpGetRoomsTablesRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_GET_ROOMS_TABLES, request)
            .await
    }

    /* [157A-9] F9.5: definición completa de un menú (grupos + platos + suplementos). */
    pub async fn get_menu_definition(
        &self,
        request: &BdpGetMenuRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_GET_MENU, request)
            .await
    }

    /* [157A-9] F9.5: definición completa de un fastfood (ingredientes + precios base). */
    pub async fn get_fastfood_definition(
        &self,
        request: &BdpGetFastfoodRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_GET_FASTFOOD, request)
            .await
    }

    /* [157A-9] F9.5: definición completa de un pack (grupos + elementos). */
    pub async fn get_pack_definition(
        &self,
        request: &BdpGetPackRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_GET_PACK, request)
            .await
    }

    /* [247A-11] Fase 1 compras BDP: exportación de albaranes de compra. */
    pub async fn export_purchase_notes(
        &self,
        request: &BdpExportPurchaseNotesRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_EXPORT_PURCHASE_NOTES, request)
            .await
    }

    /* [198A-1/F2] Lecturas de soporte para las escrituras BDP. */
    pub async fn get_application_version(
        &self,
        request: &BdpGetApplicationVersionRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_GET_APPLICATION_VERSION, request)
            .await
    }

    pub async fn get_profiles_create_article_list(&self) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_PROFILES_CREATE_ARTICLE_LIST, &BdpEmptyRequest {})
            .await
    }

    pub async fn get_profile_list_modify_article_list(&self) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_PROFILES_MODIFY_ARTICLE_LIST, &BdpEmptyRequest {})
            .await
    }

    pub async fn get_profiles_create_department_list(&self) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(
            BDP_PATH_PROFILES_CREATE_DEPARTMENT_LIST,
            &BdpEmptyRequest {},
        )
        .await
    }
}
