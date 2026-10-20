/* Tests de los contratos BDP y del inventario de endpoints. */

use rust_decimal::Decimal;

use super::articulos::{BdpExportArticlesRequest, BdpExportArticlesResponse};
use super::clientes_pedidos::BdpOrderIdentifier;
use super::consultas::BdpGetPosTendersRequest;
use super::endpoints::{es_escritura_bdp, BdpEndpointArea, BDP_ENDPOINTS};
use super::operaciones::{
    BdpAddOrderTipRequest, BdpCallWaiterRequest, BdpExportPurchaseNotesResponse,
    BdpRegularizationRequest, BdpUpdateStockRequest,
};

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

/* [D28] La lista de albaranes del BDP local llega en `ExportedData.Cabecera`,
 * no en `DocumentsLists`: el contrato anterior devolvía cero albaranes sin
 * error. Las dos formas se aceptan. */
#[test]
fn purchase_notes_lee_cabecera_dentro_de_exported_data() {
    let parsed: BdpExportPurchaseNotesResponse = serde_json::from_value(serde_json::json!({
        "ExportedData": {"Cabecera": [
            {"SerieAlbaran": "00001AC", "NumAlbaran": 7, "NomProveedor": "PROVEEDOR UNO",
             "TotalAlbaran": 12.5, "CampoDesconocido": "se conserva en extra"},
        ]},
        "ErrorMessage": ""
    }))
    .unwrap();
    assert_eq!(parsed.documents_lists.len(), 1);
    let albaran = &parsed.documents_lists[0];
    assert_eq!(albaran.serie_albaran.as_deref(), Some("00001AC"));
    assert_eq!(albaran.num_albaran.as_deref(), Some("7"));
    assert_eq!(albaran.nom_proveedor.as_deref(), Some("PROVEEDOR UNO"));
    assert_eq!(albaran.extra["CampoDesconocido"], "se conserva en extra");
}

#[test]
fn purchase_notes_sigue_aceptando_documents_lists() {
    let parsed: BdpExportPurchaseNotesResponse = serde_json::from_value(serde_json::json!({
        "DocumentsLists": [{"NumAlbaran": 9}]
    }))
    .unwrap();
    assert_eq!(parsed.documents_lists.len(), 1);
}

#[test]
fn purchase_notes_sin_lista_no_falla_y_deja_el_error() {
    let parsed: BdpExportPurchaseNotesResponse = serde_json::from_value(serde_json::json!({
        "ExportedData": {},
        "ErrorMessage": "[403900]-RANGO DE PROVEEDORES INCORRECTO"
    }))
    .unwrap();
    assert!(parsed.documents_lists.is_empty());
    assert_eq!(
        parsed.error_message,
        "[403900]-RANGO DE PROVEEDORES INCORRECTO"
    );
}
