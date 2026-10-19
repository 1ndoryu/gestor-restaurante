/* Cliente BDP: escrituras de stock, puntos, propinas y altas. */

use serde_json::Value;

use crate::services::bdp_weblink_catalog::{
    BdpAddOrderTipRequest, BdpAddPointsRequest, BdpCallWaiterRequest, BdpCreateArticlesRequest,
    BdpCreateDepartmentProfilesRequest, BdpCreateDepartmentRequest, BdpCreateFamilyRequest,
    BdpCreateSubfamilyRequest, BdpGetPointsRequest, BdpMassiveStockRequest,
    BdpModifyArticleRequest, BdpModifyPricesRequest, BdpRegularizationRequest, BdpTransferRequest,
    BdpUpdateStockRequest, BDP_PATH_ADD_ORDER_TIP, BDP_PATH_ADD_POINTS, BDP_PATH_CALL_WAITER,
    BDP_PATH_CREATE_ARTICLES, BDP_PATH_CREATE_DEPARTMENT, BDP_PATH_CREATE_DEPARTMENT_PROFILES,
    BDP_PATH_CREATE_FAMILY, BDP_PATH_CREATE_SUBFAMILY, BDP_PATH_GET_POINTS,
    BDP_PATH_MODIFY_ARTICLE, BDP_PATH_MODIFY_PRICES, BDP_PATH_REGULARIZATIONS, BDP_PATH_TRANSFERS,
    BDP_PATH_UPDATE_MASSIVE_INVENTORY, BDP_PATH_UPDATE_MASSIVE_STOCK, BDP_PATH_UPDATE_STOCK,
};

use super::tipos::{BdpWeblinkClient, BdpWeblinkError};
use super::transporte::ensure_embedded_errors_empty;

impl BdpWeblinkClient<'_> {
    pub async fn get_points(
        &self,
        request: &BdpGetPointsRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.post_authenticated_json(BDP_PATH_GET_POINTS, request)
            .await
    }

    /* [198A-1/F3] Escrituras de artículos.
     * [049A-1/H-W] El BDP responde HTTP 200 con `ErrorMessage` vacío pero con
     * errores embebidos en `ListaErroresArticulo` (p. ej. DeptCode/TAVCode
     * inexistentes). Sin validar esa lista, un alta rechazada se marcaría como
     * `sincronizado` (falso positivo). Se valida SIEMPRE tras escribir. */
    pub async fn create_articles_and_update_profiles(
        &self,
        request: &BdpCreateArticlesRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.ensure_write_target_allowed()?;
        let response = self
            .post_authenticated_json(BDP_PATH_CREATE_ARTICLES, request)
            .await?;
        ensure_embedded_errors_empty(&response)?;
        Ok(response)
    }

    pub async fn modify_article_and_update_profile(
        &self,
        request: &BdpModifyArticleRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.ensure_write_target_allowed()?;
        let response = self
            .post_authenticated_json(BDP_PATH_MODIFY_ARTICLE, request)
            .await?;
        ensure_embedded_errors_empty(&response)?;
        Ok(response)
    }

    pub async fn modify_prices_articles(
        &self,
        request: &BdpModifyPricesRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.ensure_write_target_allowed()?;
        self.post_authenticated_json(BDP_PATH_MODIFY_PRICES, request)
            .await
    }

    /* [198A-1/F5] Escrituras de departamentos.
     * [W-Q2.3] El BDP real devuelve HTTP 200 con `ErrorMessage` vacío y el
     * rechazo embebido en `ListaErroresArticulo` (p. ej. "EL DEPARTAMENTO 104
     * YA EXISTE"); sin `ensure_embedded_errors_empty` se marcaba como `exito`
     * (falso positivo). Igual que los escritores de artículos (H-W). */
    pub async fn create_department(
        &self,
        request: &BdpCreateDepartmentRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.ensure_write_target_allowed()?;
        let response = self
            .post_authenticated_json(BDP_PATH_CREATE_DEPARTMENT, request)
            .await?;
        ensure_embedded_errors_empty(&response)?;
        Ok(response)
    }

    pub async fn create_department_and_update_profiles(
        &self,
        request: &BdpCreateDepartmentProfilesRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.ensure_write_target_allowed()?;
        let response = self
            .post_authenticated_json(BDP_PATH_CREATE_DEPARTMENT_PROFILES, request)
            .await?;
        ensure_embedded_errors_empty(&response)?;
        Ok(response)
    }

    /* [198A-1/F6] Escrituras de comandas y plano de sala. */
    pub async fn add_order_tip(
        &self,
        request: &BdpAddOrderTipRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.ensure_write_target_allowed()?;
        self.post_authenticated_json(BDP_PATH_ADD_ORDER_TIP, request)
            .await
    }

    pub async fn call_waiter(
        &self,
        request: &BdpCallWaiterRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.ensure_write_target_allowed()?;
        self.post_authenticated_json(BDP_PATH_CALL_WAITER, request)
            .await
    }

    /* [198A-1/F7] Escritura de fidelización. */
    pub async fn add_points(
        &self,
        request: &BdpAddPointsRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.ensure_write_target_allowed()?;
        self.post_authenticated_json(BDP_PATH_ADD_POINTS, request)
            .await
    }

    /* [198A-1/F4] Escrituras de almacén/stock. */
    pub async fn create_family(
        &self,
        request: &BdpCreateFamilyRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.ensure_write_target_allowed()?;
        self.post_authenticated_json(BDP_PATH_CREATE_FAMILY, request)
            .await
    }

    pub async fn create_subfamily(
        &self,
        request: &BdpCreateSubfamilyRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.ensure_write_target_allowed()?;
        self.post_authenticated_json(BDP_PATH_CREATE_SUBFAMILY, request)
            .await
    }

    pub async fn regularize_stock(
        &self,
        request: &BdpRegularizationRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.ensure_write_target_allowed()?;
        self.post_authenticated_json(BDP_PATH_REGULARIZATIONS, request)
            .await
    }

    pub async fn transfer_stock(
        &self,
        request: &BdpTransferRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.ensure_write_target_allowed()?;
        self.post_authenticated_json(BDP_PATH_TRANSFERS, request)
            .await
    }

    pub async fn update_massive_stock(
        &self,
        request: &BdpMassiveStockRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.ensure_write_target_allowed()?;
        self.post_authenticated_json(BDP_PATH_UPDATE_MASSIVE_STOCK, request)
            .await
    }

    pub async fn update_stock(
        &self,
        request: &BdpUpdateStockRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.ensure_write_target_allowed()?;
        self.post_authenticated_json(BDP_PATH_UPDATE_STOCK, request)
            .await
    }

    pub async fn update_massive_inventory(
        &self,
        request: &BdpMassiveStockRequest,
    ) -> Result<Value, BdpWeblinkError> {
        self.ensure_write_target_allowed()?;
        self.post_authenticated_json(BDP_PATH_UPDATE_MASSIVE_INVENTORY, request)
            .await
    }
}
