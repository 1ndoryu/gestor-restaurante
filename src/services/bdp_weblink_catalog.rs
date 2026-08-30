/* [065A-3] Inventario BDP/WebLink extraido del manual.
 * Se codifican rutas y payloads minimos antes de tener acceso real al PC del
 * restaurante. Las respuestas complejas quedan como JSON hasta contrastarlas
 * contra datos reales de BDP-NET para no inventar contratos incompletos. */

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
 * solo-lectura vs escritura) y para el kill-switch de destino por endpoint.
 * La autoridad real de enforcement sigue siendo
 * `BdpWeblinkClient::ensure_write_target_allowed`. */
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpEmptyRequest {}

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

/* [157A-7] F9.1: Struct tipado para parsear la respuesta de ExportArticles.
 * BDP devuelve {"Articles": [{Code, Name, Family, Subfamily, Department,
 * Tax1, Tax2, Price1..Price5, Discount, BarCode, Active}]}
 * Usado por BdpSyncService::sync_catalog(). */

/// Entrada de precios donde BDP puede anidar el stock (`PricesTableDataType`).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpArticlePriceEntry {
    /// Stock dentro de una entrada de la tabla de precios.
    #[serde(default, alias = "CurrentStock", alias = "Stock")]
    pub current_stock: Option<Decimal>,
}

/// Un artículo individual del array `Articles` en la respuesta de `ExportArticles`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpExportArticleItem {
    /// Código del artículo (puede venir como string o número en BDP)
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub code: Option<String>,
    /// Fallback: algunos endpoints usan `ItemCode` en vez de `Code`
    #[serde(
        default,
        alias = "ItemCode",
        deserialize_with = "deserialize_optional_string"
    )]
    pub item_code: Option<String>,
    /// Descripción / nombre del artículo
    #[serde(default, alias = "Description")]
    pub name: Option<String>,
    /// Código de familia
    #[serde(default)]
    pub family: Option<i32>,
    /// Código de subfamilia
    #[serde(default)]
    pub subfamily: Option<i32>,
    /// Código de departamento
    #[serde(default)]
    pub department: Option<i32>,
    /// Porcentaje IVA 1 (venta)
    #[serde(default)]
    pub tax1: Option<Decimal>,
    /// Porcentaje IVA 2
    #[serde(default)]
    pub tax2: Option<Decimal>,
    /// Precio tarifa 1
    #[serde(default)]
    pub price1: Option<Decimal>,
    /// Precio tarifa 2
    #[serde(default)]
    pub price2: Option<Decimal>,
    /// Precio tarifa 3
    #[serde(default)]
    pub price3: Option<Decimal>,
    /// Precio tarifa 4
    #[serde(default)]
    pub price4: Option<Decimal>,
    /// Precio tarifa 5
    #[serde(default)]
    pub price5: Option<Decimal>,
    /// Porcentaje de descuento
    #[serde(default)]
    pub discount: Option<Decimal>,
    /// Código de barras
    #[serde(default)]
    pub bar_code: Option<String>,
    /// Artículo activo
    #[serde(default = "default_true")]
    pub active: bool,
    /* [237A-4] Stock actual del artículo. BDP puede devolverlo como CurrentStock
     * o Stock en la respuesta de ExportArticles (PricesTableDataType).
     * Si no viene (módulo de almacén no activo o perfil sin stock), será None
     * y el campo quedará en 0 al hacer sync-catalog.
     * ⚠️ Si el stock siempre aparece como 0 en la tabla, verificar que el
     * perfil de exportación de BDP incluya el campo CurrentStock. */
    #[serde(default, alias = "CurrentStock", alias = "Stock")]
    pub current_stock: Option<Decimal>,
    /// Tabla de precios/stock alternativa. BDP puede devolver `CurrentStock`
    /// anidado dentro de `PricesTableData` o `Prices` en lugar de a nivel raíz.
    #[serde(default, alias = "PricesTableData", alias = "Prices")]
    pub prices_table_data: Vec<BdpArticlePriceEntry>,
}

impl BdpExportArticleItem {
    /// Devuelve el stock efectivo del artículo: primero el campo raíz, luego
    /// cualquier entrada anidada en la tabla de precios.
    #[must_use]
    pub fn effective_stock(&self) -> Option<Decimal> {
        self.current_stock
            .or_else(|| self.prices_table_data.iter().find_map(|p| p.current_stock))
    }
}

fn default_true() -> bool {
    true
}

impl BdpExportArticleItem {
    /// Devuelve el código del artículo (Code o `ItemCode`)
    #[must_use]
    pub fn art_code(&self) -> Option<&str> {
        self.code
            .as_deref()
            .or(self.item_code.as_deref())
            .filter(|s| !s.is_empty())
    }

    /// Devuelve la descripción del artículo
    #[must_use]
    pub fn description(&self) -> &str {
        self.name.as_deref().unwrap_or("")
    }
}

/// Respuesta tipada de `POST /API/Articles/Export`.
/// El array viene dentro de la clave `Articles`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpExportArticlesResponse {
    #[serde(default)]
    pub articles: Vec<BdpExportArticleItem>,
}

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<serde_json::Value> = serde::Deserialize::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(value)) => Ok(Some(value)),
        Some(serde_json::Value::Number(value)) => Ok(Some(value.to_string())),
        Some(other) => Err(serde::de::Error::custom(format!(
            "código BDP debe ser string o número, recibido {other}"
        ))),
    }
}

/// Resultado del sync de catálogo BDP → Glory (F9.1).
/// [128A-1/F2] Contadores de reglas locales del import (M6/M7): artículos con
/// ediciones locales que el import no sobrescribe y artículos desactivados
/// localmente que el import no reactiva. Visibles en la UI del reporte.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct BdpCatalogSyncResult {
    pub creados: u32,
    pub actualizados: u32,
    pub sin_cambios: u32,
    /* [128A-1/F2] M6: filas con ediciones locales no sobrescritas por el import */
    pub omitidos_ediciones_locales: u32,
    /* [128A-1/F2] M7: artículos desactivados localmente que el import no reactiva */
    pub desactivados_localmente: u32,
    pub errores: u32,
    pub total_bdp: usize,
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

/* [048A-8] Contrato completo de CreateCustomer según la documentación
 * oficial de BDP (WebLink REST API). El BDP real con módulo de gestión
 * devuelve NullReferenceException si faltan los campos de gestión
 * (PaymentMode, Representative, AreaCode, TAVCode, RateCode), por lo que
 * el payload debe incluir todos los campos del JSON de solicitud, incluso
 * con valores vacíos. La clave del e-mail en la solicitud es `Email`
 * (no `EMail`, que es la clave de la respuesta de ExportCustomers). */
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpCreateCustomerRequest {
    pub code: i32,
    /// Nombre fiscal (máx. 40 caracteres; BDP recorta a 40).
    pub fiscal_name: String,
    /// Nombre comercial (máx. 40 caracteres; BDP recorta a 40).
    pub commercial_name: String,
    /// Dirección (máx. 40 caracteres; BDP recorta a 40).
    pub address: String,
    /// Código postal (máx. 10 caracteres; BDP recorta a 10).
    pub post_code: String,
    /// Población (máx. 40 caracteres; BDP recorta a 40).
    pub town: String,
    /// Provincia (máx. 40 caracteres; BDP recorta a 40).
    pub province: String,
    /// Teléfono fijo (máx. 15 caracteres; BDP recorta a 15).
    pub land_line: String,
    /// Teléfono móvil (máx. 15 caracteres; BDP recorta a 15).
    pub mobile_phone: String,
    /// Identificador fiscal (máx. 15 caracteres; BDP recorta a 15).
    pub fin: String,
    /// Tipo de documento de identificación:
    /// 1=N.I.F., 2=N.I.F. Extranjero, 3=Pasaporte, 4=ID País de Residencia,
    /// 5=Certificado Residencia, 6=Otro Documento.
    /// El BDP real exige un entero (desecha "1.0" con "not a valid integer").
    pub fin_type: i32,
    /// Correo electrónico (máx. 60 caracteres; BDP recorta a 10).
    #[serde(rename = "Email")]
    pub email: String,
    /// Porcentaje de descuento (0,00 a 99,99; más de 2 decimales se redondea).
    pub per_discount: f64,
    /// Código de forma de pago (1-99). En apps con módulo de gestión NO puede ser 0.
    pub payment_mode: i32,
    /// Código de representante (1-9999). En módulo de gestión NO puede ser 0.
    pub representative: i32,
    /// Código de zona (1-999). En módulo de gestión NO puede ser 0.
    pub area_code: i32,
    /// Código de TAV (1-99). En módulo de gestión NO puede ser 0.
    pub tav_code: i32,
    /// Código de tarifa (1-99). En módulo de gestión NO puede ser 0.
    pub rate_code: i32,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

/* [157A-9] F9.2: GetArticle — consulta individual de artículo por código.
 * Path: POST /API/Articles/Get
 * Input: { "ArtCode": 1001 }
 * Output: ArticleData con campos extensos (DeptCode, TAVPer, Price1..5, etc.) */
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetArticleRequest {
    pub art_code: i64,
}

/* [157A-9] F9.3: GetPricesArticles — precios de venta de un artículo.
 * Path: POST /API/Articles/GetPrices
 * Input: { "ArtCode": 1001 }
 * Output: { "Prices": [1.05,...], "Discounts": [25.0,...], "ErrorMessage": "" } */
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetPricesArticlesRequest {
    pub art_code: i64,
}

/// Respuesta tipada de `POST /API/Articles/GetPrices`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetPricesArticlesResponse {
    #[serde(default)]
    pub prices: Vec<Decimal>,
    #[serde(default, alias = "Disconts")]
    pub discounts: Vec<Decimal>,
    #[serde(default)]
    pub error_message: String,
}

/* [128A-1/F3] N6: GetStock — stock de un artículo en un almacén.
 * Path: POST /API/Warehouse/GetStock (especulativo, manual WEBLINK).
 * Input: { "Article": 1001, "Altern": 0, "Store": 1 }
 * Output: { "Stock": 0.0, "ErrorMessage": "" } */
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetStockRequest {
    pub article: i64,
    pub altern: i32,
    pub store: i32,
}

/// Respuesta tipada de `POST /API/Warehouse/GetStock`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetStockResponse {
    #[serde(default)]
    pub stock: Decimal,
    #[serde(default)]
    pub error_message: String,
}

/* [128A-1/F3] N6: GetListStock — stock de varios artículos en un almacén.
 * Path: POST /API/Warehouse/GetListStock (especulativo, manual WEBLINK).
 * Input: { "Store": 1, "Articles": [{ "Article": 1001, "Altern": 0 }, ...] }
 * Output: { "Stock": [{ "Article": 1001, "Altern": 0, "Units": 0.0,
 *           "ErrorMessage": "" }, ...], "ErrorMessage": "" } */
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetListStockRequest {
    pub store: i32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub articles: Vec<BdpListStockItemRequest>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpListStockItemRequest {
    pub article: i64,
    pub altern: i32,
}

/// Un artículo en la respuesta de `GetListStock`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpListStockItemResponse {
    pub article: i64,
    #[serde(default)]
    pub altern: i32,
    #[serde(default)]
    pub units: Decimal,
    #[serde(default)]
    pub error_message: String,
}

/// Respuesta tipada de `POST /API/Warehouse/GetListStock`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetListStockResponse {
    #[serde(default)]
    pub stock: Vec<BdpListStockItemResponse>,
    #[serde(default)]
    pub error_message: String,
}

/* [157A-9] F9.4: GetRoomTables — mesas de un salón.
 * Path: POST /API/Room/GetTables
 * Input: { "Id": 1 }
 * Output: { "Tables": [1,3,5,...], "ErrorMessage": "" } */
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetRoomTablesRequest {
    pub id: i32,
}

/// Respuesta tipada de `POST /API/Room/GetTables`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetRoomTablesResponse {
    #[serde(default)]
    pub tables: Vec<i32>,
    #[serde(default)]
    pub error_message: String,
}

/* [157A-9] F9.4: GetRoomsTables — todos los salones con mesas.
 * Path: POST /API/Rooms/GetTables
 * Input: { "Ids": [1,2] } o {}
 * Output: { "Rooms": [{ "Id": 1, "Name": "Comedor", "Tables": [...] }], "ErrorMessage": "" } */
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetRoomsTablesRequest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<i32>,
}

/// Un salón con sus mesas en la respuesta de `GetRoomsTables`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpRoomData {
    pub id: i32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub tables: Vec<i32>,
}

/// Respuesta tipada de `POST /API/Rooms/GetTables`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetRoomsTablesResponse {
    #[serde(default)]
    pub rooms: Vec<BdpRoomData>,
    #[serde(default)]
    pub error_message: String,
}

/* [157A-9] F9.5: GetMenuDefinition, GetFastfoodDefinition, GetPackDefinition.
 * Endpoints informativos que exponen JSON raw de BDP. */

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetMenuRequest {
    pub menu_id: i32,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetFastfoodRequest {
    pub fastfood_id: i32,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetPackRequest {
    pub pack_id: i32,
}

/* [247A-11/287A-4] Fase 1 compras BDP: petición de ExportPurchaseNotes.
 * Los filtros son opcionales en Glory, pero el BDP real exige un rango de
 * proveedores; el handler lo completa antes de construir este contrato. */
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpExportPurchaseNotesRequest {
    pub export_profile_code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_supplier: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_supplier: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_serial: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_serial: Option<String>,
}

/// Albarán de compra individual devuelto por `ExportPurchaseNotes`.
/// Solo se mapean los campos de cabecera confirmados; el resto del JSON se
/// conserva en `extra` y se persiste en `datos_bdp`. La estructura exacta de
/// las líneas de albarán aún no está verificada con datos reales de BDP.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpPurchaseNoteData {
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub serie_albaran: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub num_albaran: Option<String>,
    #[serde(default)]
    pub fecha_albaran: Option<String>,
    #[serde(default)]
    pub cod_proveedor: Option<serde_json::Value>,
    #[serde(default)]
    pub nom_proveedor: Option<String>,
    #[serde(default)]
    pub total_albaran: Option<Decimal>,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// Respuesta tipada de `POST /API/ExportProfiles/PurchaseNotes`.
/// BDP devuelve `DocumentsLists` como array de albaranes.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpExportPurchaseNotesResponse {
    #[serde(default, alias = "DocumentsLists")]
    pub documents_lists: Vec<BdpPurchaseNoteData>,
    #[serde(default)]
    pub error_message: String,
}

/* ===== [198A-1/F2] Lecturas de soporte ===== */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetApplicationVersionRequest {
    pub application: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetPointsRequest {
    pub customer: i64,
}

/* ===== [198A-1] Escrituras: comandas, plano, fidelización ===== */

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpAddOrderTipRequest {
    pub order_identifier: BdpOrderIdentifier,
    pub amount: Decimal,
    pub add_tip: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpCallWaiterRequest {
    pub table: i32,
    pub room: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpAddPointsRequest {
    pub customer: i64,
    pub points_added: Decimal,
    pub reason: String,
}

/* ===== [198A-1] Escrituras: departamentos ===== */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpCreateDepartmentRequest {
    pub code: i32,
    pub description: String,
    pub short_description: String,
    pub graph_description1: String,
    pub graph_description2: String,
    pub graph_description3: String,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpCreateDepartmentProfilesRequest {
    pub code: i32,
    pub description: String,
    pub short_description: String,
    pub graph_description1: String,
    pub graph_description2: String,
    pub graph_description3: String,
    pub overwrite: bool,
    pub all_profiles: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_list: Option<Value>,
}

/* ===== [198A-1] Escrituras: artículos ===== */

/// Un artículo en el cuerpo de `CreateArticlesAndUpdateProfiles` y
/// `ModifyArticleAndUpdateProfile` (subconjunto de `ArticleListDataType`).
/// Los campos no mapeados pueden añadirse en `extra` sin romper el contrato.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpArticleData {
    pub art_code: i64,
    pub art_description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dept_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dept_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tav_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tav_per: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price1: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price2: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price3: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price4: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price5: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_article: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_inventoriable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifiable_price: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu_dish: Option<bool>,
    /// Campos adicionales de `ArticleListDataType` (combinados, impresoras...).
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpCreateArticlesRequest {
    pub automatic_code: bool,
    pub article_data: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profiles_list: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_profiles: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpModifyArticleRequest {
    pub article_data: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profiles_list: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_profiles: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpModifyPricesRequest {
    pub articles_data_list: Value,
}

/* ===== [198A-1] Escrituras: almacén/stock ===== */

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpCreateFamilyRequest {
    pub code: i32,
    pub description: String,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpCreateSubfamilyRequest {
    pub code: i32,
    pub description: String,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpUpdateStockRequest {
    pub article: i64,
    pub altern: i32,
    pub units: Decimal,
    pub cod_reg: i32,
    pub store: i32,
    pub date_reg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpRegularizationRequest {
    pub article: i64,
    #[serde(rename = "sD1")]
    pub sd1: String,
    #[serde(rename = "sD2")]
    pub sd2: String,
    #[serde(rename = "sD3")]
    pub sd3: String,
    pub units: Decimal,
    pub cod_reg: i32,
    pub store: i32,
    pub date_reg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpTransferRequest {
    pub article: i64,
    #[serde(rename = "sD1")]
    pub sd1: String,
    #[serde(rename = "sD2")]
    pub sd2: String,
    #[serde(rename = "sD3")]
    pub sd3: String,
    pub units: Decimal,
    pub cod_transfer: i32,
    pub store_from: i32,
    pub store_to: i32,
    pub date_transfer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpStockInfoEntry {
    pub article: i64,
    pub units: Decimal,
}

/// Cuerpo compartido de `UpdateMassiveStock` y `UpdateMassiveInventory`
/// (`ArticlesList` de `InfoStock`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpMassiveStockRequest {
    pub cod_reg: i32,
    pub store: i32,
    pub date_reg: String,
    pub articles_list: Vec<BdpStockInfoEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

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
            BdpEndpointArea::Salones,
            BdpEndpointArea::Menus,
            BdpEndpointArea::Compras,
            BdpEndpointArea::Perfiles,
            BdpEndpointArea::Fidelizacion,
            BdpEndpointArea::Almacen,
        ] {
            assert!(BDP_ENDPOINTS.iter().any(|endpoint| endpoint.area == area));
        }
    }

    /* [198A-1] Inventario de escrituras BDP nuevas. */
    #[test]
    fn new_write_endpoints_are_registered_and_marked_as_writes() {
        for name in [
            "CreateArticlesAndUpdateProfiles",
            "ModifyArticleAndUpdateProfile",
            "ModifyPricesArticles",
            "CreateDepartment",
            "CreateDepartmentAndupdateProfiles",
            "AddOrderTip",
            "CallWaiter",
            "AddPoints",
            "CreateFamily",
            "CreateSubfamily",
            "Regularizations",
            "Transfers",
            "UpdateMassiveStock",
            "UpdateStock",
            "UpdateMassiveInventory",
        ] {
            let endpoint = BDP_ENDPOINTS
                .iter()
                .find(|endpoint| endpoint.name == name)
                .unwrap_or_else(|| panic!("falta endpoint {name} en el catálogo"));
            assert!(
                es_escritura_bdp(endpoint.path),
                "{name} debe estar marcado como escritura"
            );
        }
        for name in [
            "GetApplicationVersion",
            "GetProfilesListCreateArticleList",
            "GetProfileListModifyArticleList",
            "GetProfilesListCreateDepartmentList",
            "GetPoints",
        ] {
            let endpoint = BDP_ENDPOINTS
                .iter()
                .find(|endpoint| endpoint.name == name)
                .unwrap_or_else(|| panic!("falta endpoint {name} en el catálogo"));
            assert!(!es_escritura_bdp(endpoint.path), "{name} es una lectura");
        }
    }

    /* [198A-1] Serialización PascalCase de los contratos nuevos (M6: fechas ISO). */
    #[test]
    fn new_write_requests_serialize_pascal_case() {
        let tip = serde_json::to_value(BdpAddOrderTipRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(123),
            amount: Decimal::from_str("2.5").unwrap(),
            add_tip: true,
        })
        .unwrap();
        assert_eq!(tip["OrderIdentifier"]["OrderId"], 123);
        /* El proyecto serializa Decimal como string (serde-with-str), igual que
         * BdpOrderPayment.amount. El número vs string en BDP real queda como
         * verificación diferida (048A-11). */
        assert_eq!(tip["Amount"], "2.5");
        assert_eq!(tip["AddTip"], true);

        let reg = serde_json::to_value(BdpRegularizationRequest {
            article: 1001,
            sd1: String::new(),
            sd2: String::new(),
            sd3: String::new(),
            units: Decimal::from_str("-2.0").unwrap(),
            cod_reg: 1,
            store: 1,
            date_reg: "2026-08-19T10:00:00".into(),
        })
        .unwrap();
        assert_eq!(reg["sD1"], "");
        assert_eq!(reg["CodReg"], 1);
        assert_eq!(reg["DateReg"], "2026-08-19T10:00:00");

        let waiter = serde_json::to_value(BdpCallWaiterRequest { table: 2, room: 1 }).unwrap();
        assert_eq!(waiter["Table"], 2);
        assert_eq!(waiter["Room"], 1);

        let stock = serde_json::to_value(BdpUpdateStockRequest {
            article: 1001,
            altern: 0,
            units: Decimal::from_str("5.0").unwrap(),
            cod_reg: 1,
            store: 1,
            date_reg: "2026-08-19T10:00:00".into(),
        })
        .unwrap();
        assert_eq!(stock["Article"], 1001);
        assert_eq!(stock["Altern"], 0);
        assert_eq!(stock["Store"], 1);
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

    #[test]
    fn export_articles_parses_pascal_case_and_numeric_codes() {
        let parsed: BdpExportArticlesResponse = serde_json::from_value(serde_json::json!({
            "Articles": [{"Code": 1001, "Name": "Café", "Price1": 2.5}]
        }))
        .unwrap();
        assert_eq!(parsed.articles.len(), 1);
        assert_eq!(parsed.articles[0].art_code(), Some("1001"));
    }

    /* [247A-10/S1] Tests defensivos de parsing de stock. */
    #[test]
    fn effective_stock_reads_root_current_stock() {
        let parsed: BdpExportArticlesResponse = serde_json::from_value(serde_json::json!({
            "Articles": [{"Code": "1001", "CurrentStock": 12.34}]
        }))
        .unwrap();
        assert_eq!(
            parsed.articles[0].effective_stock(),
            Some(rust_decimal::Decimal::from_str("12.34").unwrap())
        );
    }

    #[test]
    fn effective_stock_falls_back_to_prices_table_data() {
        let parsed: BdpExportArticlesResponse = serde_json::from_value(serde_json::json!({
            "Articles": [{"Code": "1002", "PricesTableData": [{"CurrentStock": 5.0}]}]
        }))
        .unwrap();
        assert_eq!(
            parsed.articles[0].effective_stock(),
            Some(rust_decimal::Decimal::from_f64_retain(5.0).unwrap())
        );
    }

    #[test]
    fn effective_stock_returns_none_when_stock_module_inactive() {
        let parsed: BdpExportArticlesResponse = serde_json::from_value(serde_json::json!({
            "Articles": [{"Code": "1003", "Price1": 2.5}]
        }))
        .unwrap();
        assert!(parsed.articles[0].effective_stock().is_none());
    }
}
