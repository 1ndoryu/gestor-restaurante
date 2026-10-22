/* Inventario de endpoints BDP y rutas de escritura, para introspeccion y kill-switch. */

use super::rutas::{
    BDP_PATH_ADD_ORDER_TIP, BDP_PATH_ADD_POINTS, BDP_PATH_AUTH_LOGIN, BDP_PATH_CALL_WAITER,
    BDP_PATH_CANCEL_ORDER, BDP_PATH_CREATE_ARTICLES, BDP_PATH_CREATE_CUSTOMER,
    BDP_PATH_CREATE_DEPARTMENT, BDP_PATH_CREATE_DEPARTMENT_PROFILES, BDP_PATH_CREATE_FAMILY,
    BDP_PATH_CREATE_ORDER, BDP_PATH_CREATE_SUBFAMILY, BDP_PATH_EXPORT_ARTICLES,
    BDP_PATH_EXPORT_CUSTOMERS, BDP_PATH_EXPORT_DEPARTMENTS,
    BDP_PATH_EXPORT_DEPARTMENTS_FROM_PROFILE, BDP_PATH_EXPORT_PURCHASE_NOTES,
    BDP_PATH_GET_APPLICATION_VERSION, BDP_PATH_GET_ARTICLE, BDP_PATH_GET_EMPLOYEE,
    BDP_PATH_GET_EMPLOYEES, BDP_PATH_GET_FASTFOOD, BDP_PATH_GET_LIST_STOCK, BDP_PATH_GET_MENU,
    BDP_PATH_GET_ORDER, BDP_PATH_GET_PACK, BDP_PATH_GET_POINTS, BDP_PATH_GET_POS,
    BDP_PATH_GET_POSES, BDP_PATH_GET_POS_ARTICLES, BDP_PATH_GET_POS_EMPLOYEES,
    BDP_PATH_GET_POS_TENDERS, BDP_PATH_GET_PRICES_ARTICLES, BDP_PATH_GET_ROOMS_TABLES,
    BDP_PATH_GET_ROOM_TABLES, BDP_PATH_GET_STOCK, BDP_PATH_GET_TENDERS, BDP_PATH_INVOICE_ORDER,
    BDP_PATH_MODIFY_ARTICLE, BDP_PATH_MODIFY_PRICES, BDP_PATH_ORDER_PAYMENT_ADD,
    BDP_PATH_PROFILES_CREATE_ARTICLE_LIST, BDP_PATH_PROFILES_CREATE_DEPARTMENT_LIST,
    BDP_PATH_PROFILES_MODIFY_ARTICLE_LIST, BDP_PATH_REGULARIZATIONS, BDP_PATH_SERVICE_GET_VERSION,
    BDP_PATH_SERVICE_HEALTH, BDP_PATH_TRANSFERS, BDP_PATH_UPDATE_MASSIVE_INVENTORY,
    BDP_PATH_UPDATE_MASSIVE_STOCK, BDP_PATH_UPDATE_STOCK,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BdpEndpointArea {
    Servicios,
    Articulos,
    Clientes,
    Comandas,
    Departamentos,
    Terminales,
    Empleados,
    Pagos,
    Salones,
    Menus,
    Compras,
    /* [198A-1] Áreas nuevas para las escrituras BDP. */
    Perfiles,
    Fidelizacion,
    Almacen,
}

#[derive(Debug, Clone, Copy)]
pub struct BdpEndpointSpec {
    pub name: &'static str,
    pub area: BdpEndpointArea,
    pub path: &'static str,
    pub purpose: &'static str,
}

pub const BDP_ENDPOINTS: &[BdpEndpointSpec] = &[
    BdpEndpointSpec {
        name: "ServiceHealth",
        area: BdpEndpointArea::Servicios,
        path: BDP_PATH_SERVICE_HEALTH,
        purpose: "health remoto",
    },
    BdpEndpointSpec {
        name: "Login",
        area: BdpEndpointArea::Servicios,
        path: BDP_PATH_AUTH_LOGIN,
        purpose: "sesion autenticada",
    },
    BdpEndpointSpec {
        name: "GetVersion",
        area: BdpEndpointArea::Servicios,
        path: BDP_PATH_SERVICE_GET_VERSION,
        purpose: "version BDP-NET",
    },
    BdpEndpointSpec {
        name: "ExportArticles",
        area: BdpEndpointArea::Articulos,
        path: BDP_PATH_EXPORT_ARTICLES,
        purpose: "catalogo web de articulos",
    },
    BdpEndpointSpec {
        name: "GetPOSArticlesList",
        area: BdpEndpointArea::Articulos,
        path: BDP_PATH_GET_POS_ARTICLES,
        purpose: "articulos por perfil TPV",
    },
    BdpEndpointSpec {
        name: "ExportCustomers",
        area: BdpEndpointArea::Clientes,
        path: BDP_PATH_EXPORT_CUSTOMERS,
        purpose: "exportacion de clientes",
    },
    BdpEndpointSpec {
        name: "CreateCustomer",
        area: BdpEndpointArea::Clientes,
        path: BDP_PATH_CREATE_CUSTOMER,
        purpose: "alta o sobrescritura de cliente",
    },
    BdpEndpointSpec {
        name: "CreateOrder",
        area: BdpEndpointArea::Comandas,
        path: BDP_PATH_CREATE_ORDER,
        purpose: "crear comanda",
    },
    BdpEndpointSpec {
        name: "GetOrder",
        area: BdpEndpointArea::Comandas,
        path: BDP_PATH_GET_ORDER,
        purpose: "consultar comanda",
    },
    BdpEndpointSpec {
        name: "CancelOrder",
        area: BdpEndpointArea::Comandas,
        path: BDP_PATH_CANCEL_ORDER,
        purpose: "cancelar comanda",
    },
    BdpEndpointSpec {
        name: "AddOrderPayment",
        area: BdpEndpointArea::Pagos,
        path: BDP_PATH_ORDER_PAYMENT_ADD,
        purpose: "agregar pago",
    },
    BdpEndpointSpec {
        name: "InvoiceOrder",
        area: BdpEndpointArea::Pagos,
        path: BDP_PATH_INVOICE_ORDER,
        purpose: "facturar comanda",
    },
    BdpEndpointSpec {
        name: "ExportDepartments",
        area: BdpEndpointArea::Departamentos,
        path: BDP_PATH_EXPORT_DEPARTMENTS,
        purpose: "departamentos por rango",
    },
    BdpEndpointSpec {
        name: "DepartmentsExportFromProfile",
        area: BdpEndpointArea::Departamentos,
        path: BDP_PATH_EXPORT_DEPARTMENTS_FROM_PROFILE,
        purpose: "departamentos por perfil",
    },
    BdpEndpointSpec {
        name: "GetPOS",
        area: BdpEndpointArea::Terminales,
        path: BDP_PATH_GET_POS,
        purpose: "terminal concreto",
    },
    BdpEndpointSpec {
        name: "GetPOSes",
        area: BdpEndpointArea::Terminales,
        path: BDP_PATH_GET_POSES,
        purpose: "terminales disponibles",
    },
    BdpEndpointSpec {
        name: "GetEmployee",
        area: BdpEndpointArea::Empleados,
        path: BDP_PATH_GET_EMPLOYEE,
        purpose: "empleado concreto",
    },
    BdpEndpointSpec {
        name: "GetEmployees",
        area: BdpEndpointArea::Empleados,
        path: BDP_PATH_GET_EMPLOYEES,
        purpose: "empleados disponibles",
    },
    BdpEndpointSpec {
        name: "GetPOSEmployees",
        area: BdpEndpointArea::Empleados,
        path: BDP_PATH_GET_POS_EMPLOYEES,
        purpose: "empleados de un terminal",
    },
    BdpEndpointSpec {
        name: "GetTenderList",
        area: BdpEndpointArea::Pagos,
        path: BDP_PATH_GET_TENDERS,
        purpose: "formas de pago",
    },
    BdpEndpointSpec {
        name: "GetPOSTenderList",
        area: BdpEndpointArea::Pagos,
        path: BDP_PATH_GET_POS_TENDERS,
        purpose: "formas de pago por terminal",
    },
    /* [157A-9] F9.2-F9.5: nuevos endpoints catálogo/salones/menús */
    BdpEndpointSpec {
        name: "GetArticle",
        area: BdpEndpointArea::Articulos,
        path: BDP_PATH_GET_ARTICLE,
        purpose: "consulta individual de articulo",
    },
    BdpEndpointSpec {
        name: "GetPricesArticles",
        area: BdpEndpointArea::Articulos,
        path: BDP_PATH_GET_PRICES_ARTICLES,
        purpose: "precios de venta de un articulo",
    },
    /* [128A-1/F3] N6: stock por artículo/almacén. Paths especulativos
     * (documentados en WEBLINK RESTAPI.md) — sin UI ni bloqueo standalone. */
    BdpEndpointSpec {
        name: "GetStock",
        area: BdpEndpointArea::Articulos,
        path: BDP_PATH_GET_STOCK,
        purpose: "stock de un articulo en un almacen",
    },
    BdpEndpointSpec {
        name: "GetListStock",
        area: BdpEndpointArea::Articulos,
        path: BDP_PATH_GET_LIST_STOCK,
        purpose: "stock de varios articulos en un almacen",
    },
    BdpEndpointSpec {
        name: "GetRoomTables",
        area: BdpEndpointArea::Salones,
        path: BDP_PATH_GET_ROOM_TABLES,
        purpose: "mesas de un salon",
    },
    BdpEndpointSpec {
        name: "GetRoomsTables",
        area: BdpEndpointArea::Salones,
        path: BDP_PATH_GET_ROOMS_TABLES,
        purpose: "salones con mesas",
    },
    BdpEndpointSpec {
        name: "GetMenuDefinition",
        area: BdpEndpointArea::Menus,
        path: BDP_PATH_GET_MENU,
        purpose: "definicion de menu",
    },
    BdpEndpointSpec {
        name: "GetFastfoodDefinition",
        area: BdpEndpointArea::Menus,
        path: BDP_PATH_GET_FASTFOOD,
        purpose: "definicion de fastfood",
    },
    BdpEndpointSpec {
        name: "GetPackDefinition",
        area: BdpEndpointArea::Menus,
        path: BDP_PATH_GET_PACK,
        purpose: "definicion de pack",
    },
    /* [247A-11] Fase 1 compras BDP: exportación de albaranes de compra. */
    BdpEndpointSpec {
        name: "ExportPurchaseNotes",
        area: BdpEndpointArea::Compras,
        path: BDP_PATH_EXPORT_PURCHASE_NOTES,
        purpose: "albaranes de compra",
    },
    /* [198A-1/F2] Lecturas de soporte. */
    BdpEndpointSpec {
        name: "GetApplicationVersion",
        area: BdpEndpointArea::Servicios,
        path: BDP_PATH_GET_APPLICATION_VERSION,
        purpose: "estado de suscripcion extendida",
    },
    BdpEndpointSpec {
        name: "GetProfilesListCreateArticleList",
        area: BdpEndpointArea::Perfiles,
        path: BDP_PATH_PROFILES_CREATE_ARTICLE_LIST,
        purpose: "perfiles para crear articulos",
    },
    BdpEndpointSpec {
        name: "GetProfileListModifyArticleList",
        area: BdpEndpointArea::Perfiles,
        path: BDP_PATH_PROFILES_MODIFY_ARTICLE_LIST,
        purpose: "perfiles para modificar articulos",
    },
    BdpEndpointSpec {
        name: "GetProfilesListCreateDepartmentList",
        area: BdpEndpointArea::Perfiles,
        path: BDP_PATH_PROFILES_CREATE_DEPARTMENT_LIST,
        purpose: "perfiles para crear departamentos",
    },
    BdpEndpointSpec {
        name: "GetPoints",
        area: BdpEndpointArea::Fidelizacion,
        path: BDP_PATH_GET_POINTS,
        purpose: "puntos de un cliente",
    },
    /* [198A-1/F3-F7] Escrituras BDP nuevas. */
    BdpEndpointSpec {
        name: "CreateArticlesAndUpdateProfiles",
        area: BdpEndpointArea::Articulos,
        path: BDP_PATH_CREATE_ARTICLES,
        purpose: "crear articulo y perfiles",
    },
    BdpEndpointSpec {
        name: "ModifyArticleAndUpdateProfile",
        area: BdpEndpointArea::Articulos,
        path: BDP_PATH_MODIFY_ARTICLE,
        purpose: "modificar articulo y perfiles",
    },
    BdpEndpointSpec {
        name: "ModifyPricesArticles",
        area: BdpEndpointArea::Articulos,
        path: BDP_PATH_MODIFY_PRICES,
        purpose: "modificar precios de articulos",
    },
    BdpEndpointSpec {
        name: "CreateDepartment",
        area: BdpEndpointArea::Departamentos,
        path: BDP_PATH_CREATE_DEPARTMENT,
        purpose: "crear departamento",
    },
    BdpEndpointSpec {
        name: "CreateDepartmentAndupdateProfiles",
        area: BdpEndpointArea::Departamentos,
        path: BDP_PATH_CREATE_DEPARTMENT_PROFILES,
        purpose: "crear departamento y perfiles",
    },
    BdpEndpointSpec {
        name: "AddOrderTip",
        area: BdpEndpointArea::Comandas,
        path: BDP_PATH_ADD_ORDER_TIP,
        purpose: "propina de comanda",
    },
    BdpEndpointSpec {
        name: "CallWaiter",
        area: BdpEndpointArea::Salones,
        path: BDP_PATH_CALL_WAITER,
        purpose: "reclamar atencion de camarero",
    },
    BdpEndpointSpec {
        name: "AddPoints",
        area: BdpEndpointArea::Fidelizacion,
        path: BDP_PATH_ADD_POINTS,
        purpose: "sumar/restar puntos de cliente",
    },
    BdpEndpointSpec {
        name: "CreateFamily",
        area: BdpEndpointArea::Almacen,
        path: BDP_PATH_CREATE_FAMILY,
        purpose: "crear familia",
    },
    BdpEndpointSpec {
        name: "CreateSubfamily",
        area: BdpEndpointArea::Almacen,
        path: BDP_PATH_CREATE_SUBFAMILY,
        purpose: "crear subfamilia",
    },
    BdpEndpointSpec {
        name: "Regularizations",
        area: BdpEndpointArea::Almacen,
        path: BDP_PATH_REGULARIZATIONS,
        purpose: "regularizacion de stock",
    },
    BdpEndpointSpec {
        name: "Transfers",
        area: BdpEndpointArea::Almacen,
        path: BDP_PATH_TRANSFERS,
        purpose: "traspaso entre almacenes",
    },
    BdpEndpointSpec {
        name: "UpdateMassiveStock",
        area: BdpEndpointArea::Almacen,
        path: BDP_PATH_UPDATE_MASSIVE_STOCK,
        purpose: "regularizacion masiva de stock",
    },
    BdpEndpointSpec {
        name: "UpdateStock",
        area: BdpEndpointArea::Almacen,
        path: BDP_PATH_UPDATE_STOCK,
        purpose: "actualizar stock de un articulo",
    },
    BdpEndpointSpec {
        name: "UpdateMassiveInventory",
        area: BdpEndpointArea::Almacen,
        path: BDP_PATH_UPDATE_MASSIVE_INVENTORY,
        purpose: "inventario masivo",
    },
];

/* [198A-1] Rutas de escritura BDP. Se usan para introspección/UI (badge de
 * solo-lectura vs escritura) y para separar el cliente HTTP con timeout de
 * escritura del de lectura. */
pub const BDP_WRITE_PATHS: &[&str] = &[
    BDP_PATH_AUTH_LOGIN,
    BDP_PATH_CREATE_CUSTOMER,
    BDP_PATH_CREATE_ORDER,
    BDP_PATH_CANCEL_ORDER,
    BDP_PATH_ORDER_PAYMENT_ADD,
    BDP_PATH_INVOICE_ORDER,
    BDP_PATH_CREATE_ARTICLES,
    BDP_PATH_MODIFY_PRICES,
    BDP_PATH_MODIFY_ARTICLE,
    BDP_PATH_CREATE_DEPARTMENT,
    BDP_PATH_CREATE_DEPARTMENT_PROFILES,
    BDP_PATH_ADD_ORDER_TIP,
    BDP_PATH_CALL_WAITER,
    BDP_PATH_ADD_POINTS,
    BDP_PATH_CREATE_FAMILY,
    BDP_PATH_CREATE_SUBFAMILY,
    BDP_PATH_REGULARIZATIONS,
    BDP_PATH_TRANSFERS,
    BDP_PATH_UPDATE_MASSIVE_STOCK,
    BDP_PATH_UPDATE_STOCK,
    BDP_PATH_UPDATE_MASSIVE_INVENTORY,
];

#[must_use]
pub fn es_escritura_bdp(path: &str) -> bool {
    BDP_WRITE_PATHS.contains(&path)
}
