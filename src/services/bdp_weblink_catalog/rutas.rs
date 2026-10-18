/* Rutas BDP: constantes de path de cada endpoint. */

pub const BDP_PATH_SERVICE_HEALTH: &str = "/Service/Health";
pub const BDP_PATH_SERVICE_GET_VERSION: &str = "/Service/GetVersion";
pub const BDP_PATH_AUTH_LOGIN: &str = "/Auth/Login";
pub const BDP_PATH_EXPORT_ARTICLES: &str = "/API/Articles/Export";
pub const BDP_PATH_GET_POS_ARTICLES: &str = "/API/Articles/GetPOSList";
/* [128A-1/F3] N6: stock por artículo/almacén (path especulativo — el manual
 * WEBLINK RESTAPI.md documenta /API/Warehouse/GetStock y GetListStock; se
 * marcan como especulativos hasta contrastar contra un BDP real). */
pub const BDP_PATH_GET_STOCK: &str = "/API/Warehouse/GetStock";
pub const BDP_PATH_GET_LIST_STOCK: &str = "/API/Warehouse/GetListStock";
pub const BDP_PATH_EXPORT_CUSTOMERS: &str = "/API/Customers/Export";
pub const BDP_PATH_CREATE_CUSTOMER: &str = "/API/Customers/Create";
pub const BDP_PATH_CREATE_ORDER: &str = "/API/Orders/Create";
pub const BDP_PATH_GET_ORDER: &str = "/API/Orders/Get";
pub const BDP_PATH_CANCEL_ORDER: &str = "/API/Orders/Cancel";
pub const BDP_PATH_ORDER_PAYMENT_ADD: &str = "/API/Orders/Payment/Add";
pub const BDP_PATH_INVOICE_ORDER: &str = "/API/Orders/Invoice";
pub const BDP_PATH_EXPORT_DEPARTMENTS: &str = "/API/Departments/Export";
pub const BDP_PATH_EXPORT_DEPARTMENTS_FROM_PROFILE: &str = "/API/Departments/ExportFromProfile";
pub const BDP_PATH_GET_POS: &str = "/API/POS/Get";
pub const BDP_PATH_GET_POSES: &str = "/API/POSes/Get";
pub const BDP_PATH_GET_EMPLOYEE: &str = "/API/Employee/Get";
pub const BDP_PATH_GET_EMPLOYEES: &str = "/API/Employees/Get";
pub const BDP_PATH_GET_POS_EMPLOYEES: &str = "/API/POS/Employees/Get";
pub const BDP_PATH_GET_TENDERS: &str = "/API/Tenders/GetList";
pub const BDP_PATH_GET_POS_TENDERS: &str = "/API/Tenders/GetPOSList";
/* [157A-9] F9.2: consulta individual de artículo */
pub const BDP_PATH_GET_ARTICLE: &str = "/API/Articles/Get";
/* [157A-9] F9.3: refresh de precios de artículo */
pub const BDP_PATH_GET_PRICES_ARTICLES: &str = "/API/Articles/GetPrices";
/* [157A-9] F9.4: mesas por salón / todos los salones */
pub const BDP_PATH_GET_ROOM_TABLES: &str = "/API/Room/GetTables";
pub const BDP_PATH_GET_ROOMS_TABLES: &str = "/API/Rooms/GetTables";
/* [157A-9] F9.5: definiciones de menús, fastfoods y packs */
pub const BDP_PATH_GET_MENU: &str = "/API/Menus/Get";
pub const BDP_PATH_GET_FASTFOOD: &str = "/API/FastFoods/Get";
pub const BDP_PATH_GET_PACK: &str = "/API/Packs/Get";
/* [247A-11] Fase 1 compras BDP: exportación de albaranes de compra. */
pub const BDP_PATH_EXPORT_PURCHASE_NOTES: &str = "/API/ExportProfiles/PurchaseNotes";
/* [198A-1/F2] Lecturas de soporte para escrituras BDP. */
pub const BDP_PATH_GET_APPLICATION_VERSION: &str = "/Service/GetApplicationVersion";
pub const BDP_PATH_PROFILES_CREATE_ARTICLE_LIST: &str = "/API/ProfilesLists/GetCreateArticleList";
pub const BDP_PATH_PROFILES_MODIFY_ARTICLE_LIST: &str = "/API/ProfilesLists/GetModifyArticleList";
pub const BDP_PATH_PROFILES_CREATE_DEPARTMENT_LIST: &str =
    "/API/ProfilesLists/GetCreateDepartmentList";
pub const BDP_PATH_GET_POINTS: &str = "/API/Loyalty/GetPoints";
/* [198A-1/F3-F7] Escrituras BDP nuevas (catálogo, stock, departamentos, comandas, plano, fidelización). */
pub const BDP_PATH_CREATE_ARTICLES: &str = "/API/Articles/CreateAndUpdateProfiles";
pub const BDP_PATH_MODIFY_PRICES: &str = "/API/Articles/ModifyPrices";
pub const BDP_PATH_MODIFY_ARTICLE: &str = "/API/Articles/ModifyAndUpdateProfiles";
pub const BDP_PATH_CREATE_DEPARTMENT: &str = "/API/Departments/Create";
pub const BDP_PATH_CREATE_DEPARTMENT_PROFILES: &str = "/API/Departments/CreateAndUpdateProfiles";
pub const BDP_PATH_ADD_ORDER_TIP: &str = "/API/Orders/Tip/Add";
pub const BDP_PATH_CALL_WAITER: &str = "/API/Waiters/Call";
pub const BDP_PATH_ADD_POINTS: &str = "/API/Loyalty/AddPoints";
pub const BDP_PATH_CREATE_FAMILY: &str = "/API/Warehouse/CreateFamily";
pub const BDP_PATH_CREATE_SUBFAMILY: &str = "/API/Warehouse/CreateSubfamily";
pub const BDP_PATH_REGULARIZATIONS: &str = "/API/Warehouse/Regularizations";
pub const BDP_PATH_TRANSFERS: &str = "/API/Warehouse/Transfers";
pub const BDP_PATH_UPDATE_MASSIVE_STOCK: &str = "/API/Warehouse/UpdateMassiveStock";
pub const BDP_PATH_UPDATE_STOCK: &str = "/API/Warehouse/UpdateStock";
pub const BDP_PATH_UPDATE_MASSIVE_INVENTORY: &str = "/API/Warehouse/UpdateMassiveInventory";
