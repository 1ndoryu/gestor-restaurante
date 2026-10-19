use sqlx::PgPool;

use super::campos::{summarize_value, value_count};
use super::{
    BdpSyncDryRunCheck, BdpSyncDryRunResponse, BdpSyncPreflightService, BDP_DRY_RUN_PAGE_SIZE,
};
use crate::models::ConfiguracionRestaurante;
use crate::services::bdp_weblink::{BdpWeblinkClient, BdpWeblinkError};
use crate::services::bdp_weblink_catalog::{
    BdpDepartmentsExportFromProfileRequest, BdpGetEmployeeRequest, BdpGetPosArticlesRequest,
    BdpGetPosEmployeesRequest, BdpGetPosRequest, BdpGetPosTendersRequest,
};
use serde_json::Value;

impl BdpSyncPreflightService {
    /* [169A-2] `con_pago_tender`: P1 silencioso. Con `Some(tender)` el
     * OnlyCheck final valida el payload exacto de pago-en-creación
     * (Payments + Invoice, EndType=1) sin crear nada; con `None` el dry-run
     * anterior sin pago. El tender se valida contra la lista del POS. */
    pub async fn execute(
        pool: &PgPool,
        user_id: uuid::Uuid,
        config: &ConfiguracionRestaurante,
        con_pago_tender: Option<i32>,
    ) -> BdpSyncDryRunResponse {
        let mut response = BdpSyncDryRunResponse::new(config.bdp_sync_enabled);
        if !super::bdp_configurado(config) {
            response.mensaje = "BDP no esta configurado".to_string();
            return response;
        }
        response.configurado = true;

        /* [F2.7] Check de mapeo de artículos — verificar que hay mapeos en bdp_article_map */
        Self::check_article_mapping(pool, user_id, config, &mut response).await;

        let client = BdpWeblinkClient::new(config);
        Self::check_health(&client, &mut response).await;
        Self::check_version(&client, &mut response).await;

        let pos = Self::capture(
            &mut response,
            "Terminal POS",
            "/API/POS/Get",
            client.get_pos(&BdpGetPosRequest {
                id: config.bdp_pos_id,
            }),
            &["POS"],
        )
        .await;

        let employee = Self::capture(
            &mut response,
            "Empleado BDP",
            "/API/Employee/Get",
            client.get_employee(&BdpGetEmployeeRequest {
                id: config.bdp_employee_id,
            }),
            &["Employee"],
        )
        .await;

        let pos_employees = Self::capture(
            &mut response,
            "Empleados del POS",
            "/API/POS/Employees/Get",
            client.get_pos_employees(&BdpGetPosEmployeesRequest {
                pos_id: config.bdp_pos_id,
            }),
            &["Employees"],
        )
        .await;
        Self::check_employee_is_allowed(config, pos_employees.as_ref(), &mut response);

        let tenders = Self::capture(
            &mut response,
            "Formas de pago del POS",
            "/API/Tenders/GetPOSList",
            client.get_pos_tenders(&BdpGetPosTendersRequest {
                pos_id: config.bdp_pos_id,
            }),
            &["TenderList", "Tenders"],
        )
        .await;

        /* [F3.4] Validar que los tenders configurados existen en el POS */
        Self::check_tender_mapping(config, tenders.as_ref(), &mut response);

        /* [F3.5] Validar que los order types configurados son válidos */
        Self::check_order_type_mapping(config, &mut response);

        let departments = Self::capture(
            &mut response,
            "Departamentos del perfil",
            "/API/Departments/ExportFromProfile",
            client.export_departments_from_profile(&BdpDepartmentsExportFromProfileRequest {
                profile_id: config.bdp_items_profile_id,
            }),
            &["Departamentos", "Departments"],
        )
        .await;

        let articles = Self::capture(
            &mut response,
            "Articulos del perfil",
            "/API/Articles/GetPOSList",
            client.get_pos_articles(&BdpGetPosArticlesRequest::first_page(
                config.bdp_items_profile_id,
                BDP_DRY_RUN_PAGE_SIZE,
            )),
            &[
                "ArticlesListData",
                "ArticleListData",
                "Articles",
                "ArticleList",
            ],
        )
        .await;

        if pos.is_some() && employee.is_some() && departments.is_some() {
            Self::check_order_only(
                config,
                &client,
                articles.as_ref(),
                tenders.as_ref(),
                con_pago_tender,
                &mut response,
            )
            .await;
        }

        response.listo_para_sincronizar = response.checks.iter().all(|check| check.ok);
        response.mensaje = if response.listo_para_sincronizar {
            "El simulador aceptó el contrato local; esto no demuestra el comportamiento de un BDP real"
                .to_string()
        } else {
            "BDP aun tiene checks pendientes antes de activar escrituras reales".to_string()
        };
        response
    }

    async fn check_health(client: &BdpWeblinkClient<'_>, response: &mut BdpSyncDryRunResponse) {
        let check = match client.health().await {
            Ok(health) if health.is_alive => BdpSyncDryRunCheck::ok(
                "Health",
                "/Service/Health",
                "BDP responde IsAlive=true",
                None,
                None,
            ),
            Ok(_) => BdpSyncDryRunCheck::error(
                "Health",
                "/Service/Health",
                "BDP respondio IsAlive=false",
            ),
            Err(error) => BdpSyncDryRunCheck::error(
                "Health",
                "/Service/Health",
                format!("No se pudo contactar BDP: {error}"),
            ),
        };
        response.checks.push(check);
    }

    async fn check_version(client: &BdpWeblinkClient<'_>, response: &mut BdpSyncDryRunResponse) {
        let check = match client.get_version().await {
            Ok(version) => BdpSyncDryRunCheck::ok(
                "Sesion y version",
                "/Auth/Login + /Service/GetVersion",
                "Login y version correctos",
                None,
                Some(format!(
                    "{}.{} {}",
                    version.version, version.sub_version, version.application_description
                )),
            ),
            Err(error) => BdpSyncDryRunCheck::error(
                "Sesion y version",
                "/Auth/Login + /Service/GetVersion",
                format!("Login o GetVersion fallo: {error}"),
            ),
        };
        response.checks.push(check);
    }

    async fn capture<F>(
        response: &mut BdpSyncDryRunResponse,
        name: &str,
        endpoint: &str,
        future: F,
        array_keys: &[&str],
    ) -> Option<Value>
    where
        F: std::future::Future<Output = Result<Value, BdpWeblinkError>>,
    {
        match future.await {
            Ok(value) => {
                response.checks.push(BdpSyncDryRunCheck::ok(
                    name,
                    endpoint,
                    "Endpoint BDP respondio correctamente",
                    value_count(&value, array_keys),
                    summarize_value(&value, array_keys),
                ));
                Some(value)
            }
            Err(error) => {
                response
                    .checks
                    .push(BdpSyncDryRunCheck::error(name, endpoint, error.to_string()));
                None
            }
        }
    }
}
