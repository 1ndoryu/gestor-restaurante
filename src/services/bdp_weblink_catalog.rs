/* [065A-3] Inventario BDP/WebLink extraido del manual.
 * Se codifican rutas y payloads minimos antes de tener acceso real al PC del
 * restaurante. Las respuestas complejas quedan como JSON hasta contrastarlas
 * contra datos reales de BDP-NET para no inventar contratos incompletos. */

use rust_decimal::Decimal;
use serde::Serialize;
use serde_json::Value;

pub const BDP_PATH_SERVICE_HEALTH: &str = "/Service/Health";
pub const BDP_PATH_SERVICE_GET_VERSION: &str = "/Service/GetVersion";
pub const BDP_PATH_AUTH_LOGIN: &str = "/Auth/Login";
pub const BDP_PATH_EXPORT_ARTICLES: &str = "/API/Articles/Export";
pub const BDP_PATH_GET_POS_ARTICLES: &str = "/API/Articles/GetPOSList";
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
];

#[derive(Debug, Serialize)]
pub struct BdpEmptyRequest;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpExportArticlesRequest {
    pub dept1: i32,
    pub dept2: i32,
    pub art1: i64,
    pub art2: i64,
    pub modified: bool,
    pub type_price: i32,
    pub disc: i32,
}

impl BdpExportArticlesRequest {
    #[must_use]
    pub const fn all_web_articles(type_price: i32) -> Self {
        Self {
            dept1: 1,
            dept2: 999,
            art1: 1,
            art2: 9_999_999_999_999,
            modified: false,
            type_price,
            disc: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetPosArticlesRequest {
    pub art1: i64,
    pub art2: i64,
    pub dept1: i32,
    pub dept2: i32,
    pub description: String,
    pub description_query_type: i32,
    pub items_per_page: i32,
    pub actual_page: i32,
    #[serde(rename = "nField")]
    pub n_field: i32,
    #[serde(rename = "nOrder")]
    pub n_order: i32,
    pub profile_code: i32,
}

impl BdpGetPosArticlesRequest {
    #[must_use]
    pub fn first_page(profile_code: i32, items_per_page: i32) -> Self {
        Self {
            art1: 1,
            art2: 9_999_999_999_999,
            dept1: 1,
            dept2: 999,
            description: String::new(),
            description_query_type: 0,
            items_per_page,
            actual_page: 1,
            n_field: 1,
            n_order: 0,
            profile_code,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpExportCustomersRequest {
    pub customer1: i32,
    pub customer2: i32,
}

impl Default for BdpExportCustomersRequest {
    fn default() -> Self {
        Self {
            customer1: 1,
            customer2: 999_999,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpCreateCustomerRequest {
    pub code: i32,
    pub fiscal_name: String,
    pub commercial_name: String,
    pub mobile_phone: String,
    #[serde(rename = "EMail")]
    pub email: String,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpCreateOrderRequest {
    pub employee_id: i32,
    pub items_profile_id: i32,
    pub order_end_type: i32,
    pub order_operation_type: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice: Option<bool>,
    pub order: Value,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpOrderIdentifier {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marketplace_order_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_number: Option<i32>,
}

impl BdpOrderIdentifier {
    #[must_use]
    pub const fn by_order_id(order_id: i64) -> Self {
        Self {
            order_id: Some(order_id),
            market_id: None,
            marketplace_order_id: None,
            room_number: None,
            table_number: None,
        }
    }

    #[must_use]
    pub fn by_market(market_id: i32, marketplace_order_id: impl Into<String>) -> Self {
        Self {
            order_id: None,
            market_id: Some(market_id),
            marketplace_order_id: Some(marketplace_order_id.into()),
            room_number: None,
            table_number: None,
        }
    }

    #[must_use]
    pub const fn by_table(room_number: i32, table_number: i32) -> Self {
        Self {
            order_id: None,
            market_id: None,
            marketplace_order_id: None,
            room_number: Some(room_number),
            table_number: Some(table_number),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetOrderRequest {
    pub order_identifier: BdpOrderIdentifier,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpCancelOrderRequest {
    pub pos_id: i32,
    pub order_identifier: BdpOrderIdentifier,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpOrderPayment {
    pub tender_id: i32,
    pub amount: Decimal,
    pub payment_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpAddOrderPaymentRequest {
    pub order_identifier: BdpOrderIdentifier,
    pub payment: BdpOrderPayment,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub employee_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpInvoiceOrderRequest {
    pub pos_id: i32,
    pub employee_id: i32,
    pub order_identifier: BdpOrderIdentifier,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpExportDepartmentsRequest {
    pub dept1: i32,
    pub dept2: i32,
    pub description: String,
    pub description_query_type: i32,
    pub items_per_page: i32,
    pub actual_page: i32,
    #[serde(rename = "nField")]
    pub n_field: i32,
    #[serde(rename = "nOrder")]
    pub n_order: i32,
}

impl Default for BdpExportDepartmentsRequest {
    fn default() -> Self {
        Self {
            dept1: 1,
            dept2: 999,
            description: String::new(),
            description_query_type: 0,
            items_per_page: 0,
            actual_page: 1,
            n_field: 1,
            n_order: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpDepartmentsExportFromProfileRequest {
    pub profile_id: i32,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetPosRequest {
    pub id: i32,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct BdpGetPosEmployeesRequest {
    #[serde(rename = "POSId")]
    pub pos_id: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetEmployeeRequest {
    pub id: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetEmployeesRequest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only_salespeople: Option<bool>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct BdpGetPosTendersRequest {
    #[serde(rename = "POSId")]
    pub pos_id: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_inventory_covers_pending_bdp_domains() {
        for area in [
            BdpEndpointArea::Articulos,
            BdpEndpointArea::Servicios,
            BdpEndpointArea::Clientes,
            BdpEndpointArea::Comandas,
            BdpEndpointArea::Departamentos,
            BdpEndpointArea::Terminales,
            BdpEndpointArea::Empleados,
            BdpEndpointArea::Pagos,
        ] {
            assert!(BDP_ENDPOINTS.iter().any(|endpoint| endpoint.area == area));
        }
    }

    #[test]
    fn requests_match_bdp_pascal_case_examples() {
        let articles = serde_json::to_value(BdpExportArticlesRequest::all_web_articles(1)).unwrap();
        assert_eq!(articles["Dept1"], 1);
        assert_eq!(articles["Art2"], 9_999_999_999_999_i64);
        assert!(articles.get("type_price").is_none());

        let tenders = serde_json::to_value(BdpGetPosTendersRequest { pos_id: 1 }).unwrap();
        assert_eq!(tenders["POSId"], 1);
    }
}
