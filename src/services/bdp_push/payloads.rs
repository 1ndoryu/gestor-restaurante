use rust_decimal::Decimal;
use serde_json::Value;

use crate::models::{BdpArticleMap, ConfiguracionRestaurante};
use crate::services::bdp_weblink_catalog::{
    BdpAddOrderTipRequest, BdpAddPointsRequest, BdpArticleData, BdpCancelOrderRequest,
    BdpCreateArticlesRequest, BdpCreateDepartmentProfilesRequest, BdpCreateFamilyRequest,
    BdpMassiveStockRequest, BdpModifyArticleRequest, BdpOrderIdentifier, BdpRegularizationRequest,
    BdpStockInfoEntry,
};

/* ===== [198A-1/F1] Payloads de push (construidos por los handlers locales) ===== */

/// Construye el payload de `articulo/modificar` (artículo ya en BDP, editado
/// localmente). Usa el código BDP ya mapeado; si el artículo es local puro
/// (sin código BDP numérico) devuelve Err para que el handler lo deje en F3.
pub fn payload_modificar_articulo(
    config: &ConfiguracionRestaurante,
    map: &BdpArticleMap,
) -> Result<Value, String> {
    let article_data = article_data_desde_map(config, map)?;
    let req = BdpModifyArticleRequest {
        article_data: serde_json::to_value(&article_data)
            .map_err(|error| format!("No se pudo serializar artículo: {error}"))?,
        profiles_list: None,
        all_profiles: Some(true),
    };
    serde_json::to_value(&req).map_err(|error| format!("No se pudo serializar push: {error}"))
}

/// Construye el payload de `articulo/crear` (artículo local nuevo). Requiere un
/// código numérico explícito (D3); el caller ya lo resolvió y lo guardó en
/// `articulo_bdp_codigo`.
pub fn payload_crear_articulo(
    config: &ConfiguracionRestaurante,
    map: &BdpArticleMap,
) -> Result<Value, String> {
    let article_data = article_data_desde_map(config, map)?;
    let req = BdpCreateArticlesRequest {
        automatic_code: false,
        article_data: serde_json::to_value(&article_data)
            .map_err(|error| format!("No se pudo serializar artículo: {error}"))?,
        profiles_list: None,
        all_profiles: Some(true),
    };
    serde_json::to_value(&req).map_err(|error| format!("No se pudo serializar push: {error}"))
}

/// Construye el payload de `stock/inventario` (conteo físico por lotes, D6).
/// `articulos` son las líneas ya resueltas a código BDP + unidades contadas.
pub fn payload_inventario(
    config: &ConfiguracionRestaurante,
    articulos: Vec<BdpStockInfoEntry>,
) -> Result<Value, String> {
    let req = BdpMassiveStockRequest {
        cod_reg: config.bdp_codreg_default,
        store: config.bdp_almacen_default,
        date_reg: fecha_hoy(),
        articles_list: articulos,
    };
    serde_json::to_value(&req).map_err(|error| format!("No se pudo serializar push: {error}"))
}

/// Construye el payload de `stock/regularizar` (ajuste manual de stock).
/// `almacen` es el almacén BDP del ajuste (no el por defecto de la configuración).
pub fn payload_regularizacion(
    config: &ConfiguracionRestaurante,
    bdp_articulo_codigo: i64,
    delta: Decimal,
    almacen: i32,
) -> Result<Value, String> {
    let req = BdpRegularizationRequest {
        article: bdp_articulo_codigo,
        sd1: String::new(),
        sd2: String::new(),
        sd3: String::new(),
        units: delta,
        cod_reg: config.bdp_codreg_default,
        store: almacen,
        date_reg: fecha_hoy(),
    };
    serde_json::to_value(&req).map_err(|error| format!("No se pudo serializar push: {error}"))
}

pub(super) fn article_data_desde_map(
    config: &ConfiguracionRestaurante,
    map: &BdpArticleMap,
) -> Result<BdpArticleData, String> {
    let art_code = map.articulo_bdp_codigo.trim().parse::<i64>().map_err(|_| {
        format!(
            "Artículo sin código BDP numérico: {}",
            map.articulo_glory_codigo
        )
    })?;
    Ok(BdpArticleData {
        art_code,
        art_description: if map.descripcion.trim().is_empty() {
            map.articulo_bdp_nombre.clone()
        } else {
            map.descripcion.clone()
        },
        dept_code: (map.departamento > 0).then_some(map.departamento),
        dept_description: None,
        tav_code: lookup_tav(config, map.iva_pct),
        tav_per: Some(map.iva_pct),
        price1: Some(map.precio_tarifa1),
        price2: None,
        price3: None,
        price4: None,
        price5: None,
        /* [149A-2/W-Q2.2] `activo=false` despublica el artículo en BDP
         * (`WebArticle:false`) en lugar de dejarlo visible. Es la vía de
         * neutralización de altas de prueba: BDP no expone borrado. */
        web_article: Some(map.activo),
        is_inventoriable: Some(true),
        modifiable_price: None,
        menu_dish: None,
        extra: serde_json::Map::new(),
    })
}

/* [M13] Mapeo IVA local (%) -> TAVCode BDP. Best-effort: se lee del mapa de
 * configuración; el auto-aprendizaje del mapa queda en F3.
 *
 * [321A-4] La clave del mapa se busca normalizada: `iva_pct` llega de
 * `numeric(6,2)` y `Decimal::to_string()` produce "10.00" (escala 2), pero la
 * documentación (M13) y la UI configuran el mapa con claves canónicas sin
 * decimales ("10" -> 1, "21" -> 2). Sin normalizar, el lookup fallaba siempre
 * para IVAs con escala distinta de la clave configurada. `normalize()` quita
 * los ceros finales conservando decimales significativos (10.00 -> "10",
 * 10.50 -> "10.5"); se prueba primero la clave exacta para no romper mapas
 * que ya usasen "10.00". */
pub(super) fn lookup_tav(config: &ConfiguracionRestaurante, iva_pct: Decimal) -> Option<i32> {
    let clave_exacta = iva_pct.to_string();
    let clave_normalizada = iva_pct.normalize().to_string();
    let mapa = &config.bdp_tav_map;
    let resolver = |clave: &str| {
        mapa.get(clave)
            .and_then(Value::as_i64)
            .and_then(|code| i32::try_from(code).ok())
    };
    resolver(&clave_exacta).or_else(|| resolver(&clave_normalizada))
}

fn fecha_hoy() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// Construye el payload de `departamento/crear` (D7) con `AllProfiles=true` (D4).
/// `ShortDescription` se trunca a 10 caracteres y `PrinterLevel=1`: el manual
/// `WebLink` exige abreviada no vacía de máx 10 y nivel de impresión 1-9; sin
/// ellos el BDP real devuelve avisos en `ListaErroresArticulo` (W-Q2.3).
pub fn payload_crear_departamento(code: i32, nombre: &str) -> Result<Value, String> {
    let short_description: String = nombre.chars().take(10).collect();
    let req = BdpCreateDepartmentProfilesRequest {
        code,
        description: nombre.to_string(),
        short_description,
        graph_description1: String::new(),
        graph_description2: String::new(),
        graph_description3: String::new(),
        printer_level: 1,
        overwrite: false,
        all_profiles: true,
        profile_list: None,
    };
    serde_json::to_value(&req).map_err(|error| format!("No se pudo serializar push: {error}"))
}

/// Construye el payload de `familia/crear` (D7).
pub fn payload_crear_familia(code: i32, nombre: &str) -> Result<Value, String> {
    let req = BdpCreateFamilyRequest {
        code,
        description: nombre.to_string(),
        overwrite: false,
    };
    serde_json::to_value(&req).map_err(|error| format!("No se pudo serializar push: {error}"))
}

/// Construye el payload de `propina/propina` (D8). `add_tip=true` suma,
/// `false` sustituye (decisión D8 resuelta: configurable por venta).
pub fn payload_propina(bdp_order_id: i64, amount: Decimal, add_tip: bool) -> Result<Value, String> {
    let req = BdpAddOrderTipRequest {
        order_identifier: BdpOrderIdentifier::by_order_id(bdp_order_id),
        amount,
        add_tip,
    };
    serde_json::to_value(&req).map_err(|error| format!("No se pudo serializar push: {error}"))
}

/// Construye el payload de `venta/cancelar` (`CancelOrder`, F6/D2). Usa
/// `OrderIdentifier { OrderId }` (M26: el local solo guarda `bdp_order_id`;
/// sin Room/Table/Market). Requiere `pos_id` de la configuración.
pub fn payload_cancelar(
    config: &ConfiguracionRestaurante,
    bdp_order_id: i64,
) -> Result<Value, String> {
    let req = BdpCancelOrderRequest {
        pos_id: config.bdp_pos_id,
        order_identifier: BdpOrderIdentifier::by_order_id(bdp_order_id),
    };
    serde_json::to_value(&req).map_err(|error| format!("No se pudo serializar push: {error}"))
}

/// Construye el payload de `cliente_puntos/puntos` (D9).
pub fn payload_puntos(
    bdp_customer_code: i64,
    points_added: Decimal,
    reason: &str,
) -> Result<Value, String> {
    let req = BdpAddPointsRequest {
        customer: bdp_customer_code,
        points_added,
        reason: reason.to_string(),
    };
    serde_json::to_value(&req).map_err(|error| format!("No se pudo serializar push: {error}"))
}
