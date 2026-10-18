/* Contratos BDP de consulta: TPV, empleados, formas de pago, stock, salas y menus. */

use rust_decimal::Decimal;
use serde::Serialize;

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
