/* [065A-3] Inventario BDP/WebLink extraido del manual.
 * Se codifican rutas y payloads minimos antes de tener acceso real al PC del
 * restaurante. Las respuestas complejas quedan como JSON hasta contrastarlas
 * contra datos reales de BDP-NET para no inventar contratos incompletos. */

mod articulos;
mod clientes_pedidos;
mod consultas;
mod endpoints;
mod operaciones;
mod rutas;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_article_list_parser;

pub use articulos::{
    parse_articles_value, raw_article_items_len, BdpArticlePriceEntry, BdpCatalogSyncResult,
    BdpEmptyRequest, BdpExportArticleItem, BdpExportArticlesRequest, BdpExportArticlesResponse,
    BDP_ARTICLE_LIST_KEYS,
};
pub use clientes_pedidos::{
    BdpAddOrderPaymentRequest, BdpCancelOrderRequest, BdpCreateCustomerRequest,
    BdpCreateOrderRequest, BdpDepartmentsExportFromProfileRequest, BdpExportCustomersRequest,
    BdpExportDepartmentsRequest, BdpGetOrderRequest, BdpGetPosArticlesRequest, BdpGetPosRequest,
    BdpInvoiceOrderRequest, BdpOrderIdentifier, BdpOrderPayment,
};
pub use consultas::{
    BdpGetArticleRequest, BdpGetEmployeeRequest, BdpGetEmployeesRequest, BdpGetFastfoodRequest,
    BdpGetListStockRequest, BdpGetListStockResponse, BdpGetMenuRequest, BdpGetPackRequest,
    BdpGetPosEmployeesRequest, BdpGetPosTendersRequest, BdpGetPricesArticlesRequest,
    BdpGetPricesArticlesResponse, BdpGetRoomTablesRequest, BdpGetRoomTablesResponse,
    BdpGetRoomsTablesRequest, BdpGetRoomsTablesResponse, BdpGetStockRequest, BdpGetStockResponse,
    BdpListStockItemRequest, BdpListStockItemResponse, BdpRoomData,
};
pub use endpoints::{
    es_escritura_bdp, BdpEndpointArea, BdpEndpointSpec, BDP_ENDPOINTS, BDP_WRITE_PATHS,
};
pub use operaciones::{
    BdpAddOrderTipRequest, BdpAddPointsRequest, BdpArticleData, BdpCallWaiterRequest,
    BdpCreateArticlesRequest, BdpCreateDepartmentProfilesRequest, BdpCreateDepartmentRequest,
    BdpCreateFamilyRequest, BdpCreateSubfamilyRequest, BdpExportDocumentsRequest,
    BdpExportPurchaseNotesRequest, BdpExportPurchaseNotesResponse, BdpGetApplicationVersionRequest,
    BdpGetPointsRequest, BdpMassiveStockRequest, BdpModifyArticleRequest, BdpModifyPricesRequest,
    BdpPurchaseNoteData, BdpRegularizationRequest, BdpStockInfoEntry, BdpTransferRequest,
    BdpUpdateStockRequest, APLICACION_LICENCIA_TERMINAL,
};
pub use rutas::{
    BDP_PATH_ADD_ORDER_TIP, BDP_PATH_ADD_POINTS, BDP_PATH_AUTH_LOGIN, BDP_PATH_CALL_WAITER,
    BDP_PATH_CANCEL_ORDER, BDP_PATH_CREATE_ARTICLES, BDP_PATH_CREATE_CUSTOMER,
    BDP_PATH_CREATE_DEPARTMENT, BDP_PATH_CREATE_DEPARTMENT_PROFILES, BDP_PATH_CREATE_FAMILY,
    BDP_PATH_CREATE_ORDER, BDP_PATH_CREATE_SUBFAMILY, BDP_PATH_EXPORT_ARTICLES,
    BDP_PATH_EXPORT_CUSTOMERS, BDP_PATH_EXPORT_DEPARTMENTS,
    BDP_PATH_EXPORT_DEPARTMENTS_FROM_PROFILE, BDP_PATH_EXPORT_DOCUMENTS,
    BDP_PATH_EXPORT_PURCHASE_NOTES, BDP_PATH_GET_APPLICATION_VERSION, BDP_PATH_GET_ARTICLE,
    BDP_PATH_GET_EMPLOYEE, BDP_PATH_GET_EMPLOYEES, BDP_PATH_GET_FASTFOOD, BDP_PATH_GET_LIST_STOCK,
    BDP_PATH_GET_MENU, BDP_PATH_GET_ORDER, BDP_PATH_GET_PACK, BDP_PATH_GET_POINTS,
    BDP_PATH_GET_POS, BDP_PATH_GET_POSES, BDP_PATH_GET_POS_ARTICLES, BDP_PATH_GET_POS_EMPLOYEES,
    BDP_PATH_GET_POS_TENDERS, BDP_PATH_GET_PRICES_ARTICLES, BDP_PATH_GET_ROOMS_TABLES,
    BDP_PATH_GET_ROOM_TABLES, BDP_PATH_GET_STOCK, BDP_PATH_GET_TENDERS, BDP_PATH_INVOICE_ORDER,
    BDP_PATH_MODIFY_ARTICLE, BDP_PATH_MODIFY_PRICES, BDP_PATH_ORDER_PAYMENT_ADD,
    BDP_PATH_PROFILES_CREATE_ARTICLE_LIST, BDP_PATH_PROFILES_CREATE_DEPARTMENT_LIST,
    BDP_PATH_PROFILES_MODIFY_ARTICLE_LIST, BDP_PATH_REGULARIZATIONS, BDP_PATH_SERVICE_GET_VERSION,
    BDP_PATH_SERVICE_HEALTH, BDP_PATH_TRANSFERS, BDP_PATH_UPDATE_MASSIVE_INVENTORY,
    BDP_PATH_UPDATE_MASSIVE_STOCK, BDP_PATH_UPDATE_STOCK,
};
