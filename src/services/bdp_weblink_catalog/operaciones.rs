/* Contratos BDP de operaciones: compras, puntos, propinas, altas y stock. */

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::articulos::deserialize_optional_string;
use super::clientes_pedidos::BdpOrderIdentifier;

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

/* [L8/D28] Petición de ExportProfiles/Documents. Los campos de documento no
 * están documentados en el manual: la respuesta se lee de forma tolerante. */
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpExportDocumentsRequest {
    pub export_profile_code: i32,
    pub initial_date: String,
    pub final_date: String,
    pub only_last_movements: bool,
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
    pub printer_level: i32,
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
    pub printer_level: i32,
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
