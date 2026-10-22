#![cfg(test)]
/* Tests de parseo de respuestas BDP: errores embebidos, compras, stock y propinas. */

use rust_decimal::Decimal;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::services::bdp_weblink_catalog::{
    BdpAddOrderTipRequest, BdpCreateArticlesRequest, BdpGetApplicationVersionRequest,
};

use super::tests::config;
use super::tipos::{BdpWeblinkClient, BdpWeblinkError};
use super::transporte::{ensure_embedded_errors_empty, response_error_message};

#[test]
fn response_error_message_extracts_non_empty_bdp_errors() {
    let value = serde_json::json!({ "ErrorMessage": " [300041]-BDP error " });

    assert_eq!(
        response_error_message(&value),
        Some("[300041]-BDP error".to_string())
    );
}

/* [049A-1/H-W] El BDP responde HTTP 200 con ErrorMessage vacío pero
 * errores en ListaErroresArticulo (alta/modificación rechazadas). Sin
 * detectarlo, la fila se marcaría `sincronizado` (falso positivo). */
#[test]
fn embedded_errors_detected_when_list_not_empty() {
    let value = serde_json::json!({
        "ErrorMessage": "",
        "ListaErroresArticulo": [
            "TAVCode: EL CÓDIGO DE IVA DE VENTA 0 NO EXISTE",
            "DeptCode: EL DEPARTAMENTO 0 NO EXISTE"
        ]
    });

    let error = ensure_embedded_errors_empty(&value).unwrap_err();
    match error {
        BdpWeblinkError::Remote(message) => {
            assert!(message.contains("EL CÓDIGO DE IVA DE VENTA 0 NO EXISTE"));
            assert!(message.contains("(+1 más)"));
        }
        other => panic!("se esperaba Remote, got {other:?}"),
    }
}

#[test]
fn embedded_errors_ok_when_list_empty_or_absent() {
    assert!(ensure_embedded_errors_empty(&serde_json::json!({
        "ErrorMessage": "",
        "ListaErroresArticulo": []
    }))
    .is_ok());
    assert!(ensure_embedded_errors_empty(&serde_json::json!({
        "ErrorMessage": "",
        "Articles": []
    }))
    .is_ok());
    assert!(ensure_embedded_errors_empty(&serde_json::json!({
        "ErrorMessage": ""
    }))
    .is_ok());
}

/* [D26h] Stock masivo: HTTP 200, `ErrorMessage` vacío y el rechazo por artículo
 * en `ErrorList`. Antes la cola lo marcaba `sincronizado` (falso positivo). */
#[test]
fn error_list_de_stock_con_mensaje_es_rechazo() {
    let value = serde_json::json!({
        "ErrorMessage": "",
        "ErrorList": [
            {"Article": 1002, "ErrorMessage": ""},
            {"Article": 10002, "ErrorMessage": "EL ARTÍCULO 10002 NO ES DEL TIPO WEB"}
        ]
    });

    let error = ensure_embedded_errors_empty(&value).unwrap_err();
    match error {
        BdpWeblinkError::Remote(message) => {
            assert!(
                message.contains("10002"),
                "debe nombrar el artículo: {message}"
            );
            assert!(message.contains("NO ES DEL TIPO WEB"), "mensaje: {message}");
        }
        other => panic!("se esperaba Remote, got {other:?}"),
    }
}

#[test]
fn error_list_de_stock_toda_vacia_es_exito() {
    assert!(ensure_embedded_errors_empty(&serde_json::json!({
        "ErrorMessage": "",
        "ErrorList": [
            {"Article": 1001, "ErrorMessage": ""},
            {"Article": 1002, "ErrorMessage": "  "}
        ]
    }))
    .is_ok());
}

/* El mismo rechazo por la vía real del cliente: no debe devolver Ok. */
#[tokio::test]
async fn massive_inventory_detecta_error_list_embebida() {
    use crate::services::bdp_weblink_catalog::BdpMassiveStockRequest;

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Auth/Login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "AuthSession": { "Token": "token-bdp", "ExpiresIn_InSecconds": 3540 }
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/API/Warehouse/UpdateMassiveInventory"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "ErrorList": [
                {"Article": 10002, "ErrorMessage": "EL ARTÍCULO 10002 NO ES DEL TIPO WEB"}
            ]
        })))
        .mount(&server)
        .await;

    let config = config(server.uri());
    let client = BdpWeblinkClient::new(&config);
    let error = client
        .update_massive_inventory(&BdpMassiveStockRequest {
            cod_reg: 1,
            store: 1,
            date_reg: "2026-10-21".into(),
            articles_list: Vec::new(),
        })
        .await
        .unwrap_err();
    assert!(
        matches!(&error, BdpWeblinkError::Remote(message) if message.contains("NO ES DEL TIPO WEB")),
        "error esperado Remote con el mensaje del artículo, got {error:?}"
    );
}

#[tokio::test]
async fn create_article_detects_embedded_rejection() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Auth/Login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "AuthSession": {
                "Token": "token-bdp",
                "ExpiresIn_InSecconds": 3540
            }
        })))
        .mount(&server)
        .await;

    /* [049A-1/H-W] El BDP real devuelve HTTP 200 con ErrorMessage vacío y
     * la lista de errores por artículo; antes este caso se marcaba como
     * éxito (falso positivo) en vez de rechazo. */
    Mock::given(method("POST"))
        .and(path("/API/Articles/CreateAndUpdateProfiles"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "ListaErroresArticulo": [
                "TAVCode: EL CÓDIGO DE IVA DE VENTA 0 NO EXISTE"
            ]
        })))
        .mount(&server)
        .await;

    let config = config(server.uri());
    let client = BdpWeblinkClient::new(&config);
    let error = client
        .create_articles_and_update_profiles(&BdpCreateArticlesRequest {
            automatic_code: false,
            article_data: serde_json::json!({
                "ArtCode": 90_000_003,
                "ArtDescription": "PRUEBA W1",
                "DeptCode": 1,
                "TavCode": 1,
                "TavPer": 10.0,
                "Price1": 1.0,
                "WebArticle": true,
                "IsInventoriable": true
            }),
            profiles_list: None,
            all_profiles: Some(true),
        })
        .await
        .unwrap_err();

    assert!(
        matches!(&error, BdpWeblinkError::Remote(message) if message.contains("NO EXISTE")),
        "el alta rechazada por BDP debe ser un rechazo Remote, got {error:?}"
    );
}

/* [247A-11] Fase 1 compras BDP: ExportPurchaseNotes envía el perfil y
 * devuelve la lista de albaranes. */
#[tokio::test]
async fn export_purchase_notes_posts_profile_and_parses_documents() {
    use crate::services::bdp_weblink_catalog::BdpExportPurchaseNotesRequest;

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Auth/Login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "AuthSession": {
                "Token": "token-bdp",
                "ExpiresIn_InSecconds": 3540
            }
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/API/ExportProfiles/PurchaseNotes"))
        .and(header("authorization", "Bearer token-bdp"))
        .and(body_json(serde_json::json!({
            "ExportProfileCode": 1,
            "InitialDate": "2026-07-01",
            "FinalDate": "2026-07-25"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "DocumentsLists": [
                {
                    "Serie_Albaran": "S1",
                    "Num_Albaran": "42",
                    "Fecha_Albaran": "2026-07-15T00:00:00",
                    "Cod_Proveedor": 1,
                    "Nom_Proveedor": "Proveedor A",
                    "Total_Albaran": 123.45
                }
            ]
        })))
        .mount(&server)
        .await;

    let config = config(server.uri());
    let client = BdpWeblinkClient::new(&config);
    let request = BdpExportPurchaseNotesRequest {
        export_profile_code: 1,
        initial_date: Some("2026-07-01".to_string()),
        final_date: Some("2026-07-25".to_string()),
        initial_supplier: None,
        final_supplier: None,
        initial_serial: None,
        final_serial: None,
    };
    let response = client.export_purchase_notes(&request).await.unwrap();

    assert!(response["DocumentsLists"].is_array());
    assert_eq!(response["DocumentsLists"].as_array().unwrap().len(), 1);
}

/* [128A-1/F3] N6: GetStock y GetListStock — paths especulativos del manual
 * WEBLINK RESTAPI.md. Verifican transporte (POST autenticado) y parsing
 * tolerante de la respuesta; no bloquean standalone. */
#[tokio::test]
async fn get_stock_posts_warehouse_path_and_parses_stock() {
    use crate::services::bdp_weblink_catalog::{
        BdpGetListStockRequest, BdpGetListStockResponse, BdpGetStockRequest, BdpGetStockResponse,
        BdpListStockItemRequest,
    };

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Auth/Login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "AuthSession": {
                "Token": "token-bdp",
                "ExpiresIn_InSecconds": 3540
            }
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/API/Warehouse/GetStock"))
        .and(header("authorization", "Bearer token-bdp"))
        .and(body_json(serde_json::json!({
            "Article": 1001,
            "Altern": 0,
            "Store": 1
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "Stock": 12.5,
            "ErrorMessage": ""
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/API/Warehouse/GetListStock"))
        .and(header("authorization", "Bearer token-bdp"))
        .and(body_json(serde_json::json!({
            "Store": 1,
            "Articles": [{ "Article": 1001, "Altern": 0 }]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "Stock": [{ "Article": 1001, "Altern": 0, "Units": 3.25, "ErrorMessage": "" }],
            "ErrorMessage": ""
        })))
        .mount(&server)
        .await;

    let config = config(server.uri());
    let client = BdpWeblinkClient::new(&config);

    let stock = client
        .get_stock(&BdpGetStockRequest {
            article: 1001,
            altern: 0,
            store: 1,
        })
        .await
        .unwrap();
    let parsed: BdpGetStockResponse = serde_json::from_value(stock).unwrap();
    assert_eq!(parsed.stock, Decimal::new(125, 1));
    assert_eq!(parsed.error_message, "");

    let list = client
        .get_list_stock(&BdpGetListStockRequest {
            store: 1,
            articles: vec![BdpListStockItemRequest {
                article: 1001,
                altern: 0,
            }],
        })
        .await
        .unwrap();
    let parsed_list: BdpGetListStockResponse = serde_json::from_value(list).unwrap();
    assert_eq!(parsed_list.stock.len(), 1);
    assert_eq!(parsed_list.stock[0].article, 1001);
    assert_eq!(parsed_list.stock[0].units, Decimal::new(325, 2));
}

/* [D26b] GetListStock con un artículo inexistente: el ErrorMessage global llega
 * junto a líneas válidas y el error debe quedar en su línea, no tumbar el lote.
 * Un almacén inexistente no trae líneas y sí es error del lote. */
#[tokio::test]
async fn get_list_stock_tolera_error_de_linea_y_rechaza_almacen_inexistente() {
    use crate::services::bdp_weblink_catalog::{
        BdpGetListStockRequest, BdpGetListStockResponse, BdpListStockItemRequest,
    };

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Auth/Login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "AuthSession": {
                "Token": "token-bdp",
                "ExpiresIn_InSecconds": 3540
            }
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/API/Warehouse/GetListStock"))
        .and(body_json(serde_json::json!({
            "Store": 1,
            "Articles": [{ "Article": 999999999, "Altern": 0 }, { "Article": 1001, "Altern": 0 }]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "Stock": [
                { "Article": 999999999, "Altern": 0, "Units": 0, "ErrorMessage": "[200201]-EL CÓDIGO DE ARTÍCULO 999999999 NO EXISTE" },
                { "Article": 1001, "Altern": 0, "Units": 3.25, "ErrorMessage": "" }
            ],
            "ErrorMessage": "[200204]-ALGUNO DE LOS ARTÍCULOS CONTIENE ERRORES DE VALIDACIÓN"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/API/Warehouse/GetListStock"))
        .and(body_json(serde_json::json!({
            "Store": 999,
            "Articles": [{ "Article": 1001, "Altern": 0 }]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "Stock": [],
            "ErrorMessage": "[200200]-EL ALMACÉN 999 NO EXISTE"
        })))
        .mount(&server)
        .await;

    let config = config(server.uri());
    let client = BdpWeblinkClient::new(&config);

    let lote = client
        .get_list_stock(&BdpGetListStockRequest {
            store: 1,
            articles: vec![
                BdpListStockItemRequest {
                    article: 999_999_999,
                    altern: 0,
                },
                BdpListStockItemRequest {
                    article: 1001,
                    altern: 0,
                },
            ],
        })
        .await
        .unwrap();
    let parsed: BdpGetListStockResponse = serde_json::from_value(lote).unwrap();
    assert_eq!(parsed.stock.len(), 2);
    assert!(parsed.stock[0].error_message.contains("NO EXISTE"));
    assert_eq!(parsed.stock[1].units, Decimal::new(325, 2));
    assert!(parsed.stock[1].error_message.is_empty());

    let almacen = client
        .get_list_stock(&BdpGetListStockRequest {
            store: 999,
            articles: vec![BdpListStockItemRequest {
                article: 1001,
                altern: 0,
            }],
        })
        .await;
    match almacen {
        Err(BdpWeblinkError::Remote(message)) => assert!(message.contains("ALMACÉN 999")),
        other => panic!("se esperaba error remoto del lote, se obtuvo {other:?}"),
    }
}

/* [S16-H3] Forma de la URL base: se rechazan query strings, fragmentos y vacío;
 * el destino en sí ya no se filtra. */

#[test]
fn write_target_rejects_url_with_query_string() {
    let config = config("http://127.0.0.1:18765?token=abc".to_string());
    let client = BdpWeblinkClient::new(&config);
    assert!(client.ensure_configured().is_err());
}

#[test]
fn write_target_rejects_url_with_fragment() {
    let config = config("http://127.0.0.1:18765#section".to_string());
    let client = BdpWeblinkClient::new(&config);
    assert!(client.ensure_configured().is_err());
}

#[test]
fn write_target_rejects_empty_base_url() {
    let config = config(String::new());
    let client = BdpWeblinkClient::new(&config);
    assert!(matches!(
        client.ensure_configured(),
        Err(BdpWeblinkError::NotConfigured)
    ));
}

/* [S16-H4] canonical_target en bdp_backup rechaza URLs con path/query/fragment/credenciales.
 * Estos tests complementan los de bdp_backup::tests. */
#[test]
fn localhost_with_port_is_allowed() {
    let config = config("http://localhost:8068".to_string());
    let client = BdpWeblinkClient::new(&config);
    assert!(client.ensure_configured().is_ok());
}

/* [S16-H3] IPv6 loopback: reqwest::Url::parse host_str() incluye corchetes
 * en algunas plataformas. El test se valida contra localhost que ya cubre
 * el caso loopback. Verificar en CI si se necesita cubrir IPv6 explícitamente. */

/* [198A-1] Wiremock: las escrituras nuevas autentican y usan el path
 * y payload PascalCase correctos. */
#[tokio::test]
async fn add_order_tip_posts_pascal_case_payload() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Auth/Login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "AuthSession": {"Token": "token-bdp", "ExpiresIn_InSecconds": 3540}
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/API/Orders/Tip/Add"))
        .and(body_json(serde_json::json!({
            "OrderIdentifier": {"OrderId": 123},
            "Amount": "2.5",
            "AddTip": true
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": ""
        })))
        .mount(&server)
        .await;

    let config = config(server.uri());
    let client = BdpWeblinkClient::new(&config);
    let response = client
        .add_order_tip(&BdpAddOrderTipRequest {
            order_identifier: crate::services::bdp_weblink_catalog::BdpOrderIdentifier::by_order_id(
                123,
            ),
            amount: rust_decimal::Decimal::new(25, 1),
            add_tip: true,
        })
        .await
        .unwrap();

    assert_eq!(response["ErrorMessage"], "");
}

#[tokio::test]
async fn get_application_version_posts_application_code() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Auth/Login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ErrorMessage": "",
            "AuthSession": {"Token": "token-bdp", "ExpiresIn_InSecconds": 3540}
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/Service/GetApplicationVersion"))
        .and(body_json(serde_json::json!({ "Application": 84 })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "Version": 1, "ErrorMessage": ""
        })))
        .mount(&server)
        .await;

    let config = config(server.uri());
    let client = BdpWeblinkClient::new(&config);
    let response = client
        .get_application_version(&BdpGetApplicationVersionRequest { application: 84 })
        .await
        .unwrap();

    assert_eq!(response["Version"], 1);
}
