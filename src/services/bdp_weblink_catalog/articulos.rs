/* Articulos BDP: exportacion, parseo tolerante de listas y campos del catalogo. */

use rust_decimal::Decimal;
use serde::Serialize;
use serde_json::Value;

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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpArticlePriceEntry {
    /// Stock dentro de una entrada de la tabla de precios.
    #[serde(default, alias = "CurrentStock", alias = "Stock")]
    pub current_stock: Option<Decimal>,
}

/// Un artículo individual de una colección BDP de artículos.
/// Endpoints que la producen: `ExportArticles` (clave `Articles`) y
/// `GetPOSList` por perfil (claves `ArticleListData`/`ArticlesListData`,
/// con `ArtCode`/`ArtDescription`/`TAVPer` en vez de `Code`/`Description`/`Tax1`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpExportArticleItem {
    /// Código del artículo (puede venir como string o número en BDP).
    /// `ArtCode` es la clave real de `GetPOSList`; `Code` la de `ExportArticles`.
    #[serde(
        default,
        alias = "ArtCode",
        deserialize_with = "deserialize_optional_string"
    )]
    pub code: Option<String>,
    /// Fallback: algunos endpoints usan `ItemCode` en vez de `Code`
    #[serde(
        default,
        alias = "ItemCode",
        deserialize_with = "deserialize_optional_string"
    )]
    pub item_code: Option<String>,
    /// Descripción / nombre del artículo (`ArtDescription` en `GetPOSList`)
    #[serde(default, alias = "ArtDescription", alias = "Description")]
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
    /// Porcentaje IVA 1 (venta). En `GetPOSList` el IVA llega como `TAVPer`.
    #[serde(default, alias = "TAVPer")]
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

/* [039A-1/H-Q1-03] Claves de colección de artículos según el endpoint:
 * ExportArticles → `Articles`; GetPOSList por perfil → `ArticleListData`
 * (alias observados `ArticlesListData`/`ArticleList`). Se prueban en orden
 * para parsear cualquier respuesta de artículos BDP. */
pub const BDP_ARTICLE_LIST_KEYS: &[&str] = &[
    "ArticlesListData",
    "ArticleListData",
    "Articles",
    "ArticleList",
];

/// Parsea el array de artículos de una respuesta cruda de BDP hacia
/// `BdpExportArticleItem`, probando las claves de colección conocidas.
/// Devuelve vacío (nunca errora) si la respuesta no trae colección.
/// Por ítem se intenta primero el parseo estricto (riqueza completa de
/// `ExportArticles`); si un campo desconocido con tipo raro lo tumbaría, se
/// cae a un mapeo laxo de los campos esenciales para que un ítem legítimo
/// nunca desaparezca silenciosamente del import.
#[must_use]
pub fn parse_articles_value(raw: &Value) -> Vec<BdpExportArticleItem> {
    let Some(items) = BDP_ARTICLE_LIST_KEYS
        .iter()
        .find_map(|key| raw.get(*key).and_then(Value::as_array))
    else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            serde_json::from_value::<BdpExportArticleItem>(item.clone())
                .ok()
                .or_else(|| map_article_essential(item))
        })
        .collect()
}

/// Devuelve cuántos ítems crudos trae una respuesta de artículos (para que el
/// llamador detecte parseos perdidos comparándolo con el parseo real).
#[must_use]
pub fn raw_article_items_len(raw: &Value) -> usize {
    BDP_ARTICLE_LIST_KEYS
        .iter()
        .find_map(|key| raw.get(*key).and_then(Value::as_array))
        .map_or(0, std::vec::Vec::len)
}

fn field_text(item: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| match item.get(*key) {
        Some(Value::String(value)) => {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        Some(Value::Number(value)) => Some(value.to_string()),
        _ => None,
    })
}

fn field_decimal(item: &Value, keys: &[&str]) -> Option<Decimal> {
    keys.iter().find_map(|key| match item.get(*key) {
        Some(Value::Number(value)) => value.as_f64().and_then(Decimal::from_f64_retain),
        Some(Value::String(value)) => value.parse::<Decimal>().ok(),
        _ => None,
    })
}

fn field_int(item: &Value, keys: &[&str]) -> Option<i32> {
    keys.iter().find_map(|key| match item.get(*key) {
        Some(Value::Number(value)) => value.as_i64().and_then(|n| i32::try_from(n).ok()),
        Some(Value::String(value)) => value.parse::<i32>().ok(),
        _ => None,
    })
}

fn field_bool(item: &Value, keys: &[&str]) -> Option<bool> {
    keys.iter()
        .find_map(|key| item.get(*key).and_then(Value::as_bool))
}

/* [039A-1/H-Q1-03] Mapeo laxo de los campos esenciales del import cuando el
 * parseo estricto falla por un campo con tipo inesperado. Claves reales de
 * GetPOSList: ArtCode/ArtDescription/Price1/TAVPer. */
fn map_article_essential(item: &Value) -> Option<BdpExportArticleItem> {
    let code = field_text(item, &["Code", "ArtCode", "ItemCode"])?;
    Some(BdpExportArticleItem {
        code: Some(code),
        item_code: None,
        name: field_text(item, &["Description", "ArtDescription", "Name"]),
        family: field_int(item, &["Family"]),
        subfamily: field_int(item, &["Subfamily"]),
        department: field_int(item, &["Department", "DeptCode"]),
        tax1: field_decimal(item, &["Tax1", "TAVPer"]),
        tax2: field_decimal(item, &["Tax2"]),
        price1: field_decimal(item, &["Price1", "Price"]),
        price2: field_decimal(item, &["Price2"]),
        price3: field_decimal(item, &["Price3"]),
        price4: field_decimal(item, &["Price4"]),
        price5: field_decimal(item, &["Price5"]),
        discount: field_decimal(item, &["Discount"]),
        bar_code: field_text(item, &["BarCode", "Barcode"]),
        active: field_bool(item, &["Active"]).unwrap_or(true),
        current_stock: field_decimal(item, &["CurrentStock", "Stock"]),
        prices_table_data: Vec::new(),
    })
}

pub(super) fn deserialize_optional_string<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
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
