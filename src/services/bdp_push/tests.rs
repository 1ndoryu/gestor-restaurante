use serde_json::Value;
use uuid::Uuid;

use crate::models::{BdpArticleMap, ConfiguracionRestaurante};
use crate::services::bdp_weblink::BdpWeblinkError;

use super::clasificacion::{clasificar_error, es_transitorio};
use super::constantes::{ESTADO_ERROR, ESTADO_PENDIENTE_SUSCRIPCION, ESTADO_RECHAZADO};
use super::enriquecimiento::fusionar_article_data;
use super::payloads::{article_data_desde_map, lookup_tav};

mod clasificacion_error_tests {
    use super::*;

    #[test]
    fn suscripcion_exacta_bdp() {
        let error = BdpWeblinkError::Remote("Subscripción no activada".into());
        assert_eq!(
            clasificar_error(&error),
            (ESTADO_PENDIENTE_SUSCRIPCION, false)
        );
    }

    #[test]
    fn suscripcion_con_variacion_case_y_grafia() {
        let error = BdpWeblinkError::Remote("  SUSCRIPCION NO ACTIVADA ".into());
        assert_eq!(
            clasificar_error(&error),
            (ESTADO_PENDIENTE_SUSCRIPCION, false)
        );
    }

    #[test]
    fn suscripcion_dentro_de_mensaje_mas_largo() {
        let error = BdpWeblinkError::Remote("Error: subscripción no activada para este POS".into());
        assert_eq!(
            clasificar_error(&error),
            (ESTADO_PENDIENTE_SUSCRIPCION, false)
        );
    }

    #[test]
    fn error_remoto_ajeno_no_es_suscripcion() {
        let error = BdpWeblinkError::Remote("Otro error de negocio".into());
        assert_eq!(clasificar_error(&error), (ESTADO_ERROR, false));
    }

    #[test]
    fn error_transitorio_api_incrementa_reintento() {
        let error = BdpWeblinkError::Api {
            status: 500,
            body: "boom".into(),
        };
        assert_eq!(clasificar_error(&error), (ESTADO_ERROR, true));
    }

    #[test]
    fn error_api_422_payload_invalido_es_rechazo_definitivo() {
        let error = BdpWeblinkError::Api {
            status: 422,
            body: "el campo code es obligatorio".into(),
        };
        assert_eq!(clasificar_error(&error), (ESTADO_RECHAZADO, false));
        assert!(!es_transitorio(&error));
    }

    #[test]
    fn error_api_400_y_409_tambien_son_rechazo() {
        for status in [400, 409] {
            let error = BdpWeblinkError::Api {
                status,
                body: "conflicto de negocio".into(),
            };
            assert_eq!(clasificar_error(&error), (ESTADO_RECHAZADO, false));
            assert!(!es_transitorio(&error));
        }
    }

    #[test]
    fn error_api_5xx_sigue_siendo_transitorio() {
        let error = BdpWeblinkError::Api {
            status: 503,
            body: "servidor ocupado".into(),
        };
        assert_eq!(clasificar_error(&error), (ESTADO_ERROR, true));
        assert!(es_transitorio(&error));
    }

    #[test]
    fn error_http_timeout_es_transitorio() {
        let error = BdpWeblinkError::Http("timeout al conectar con BDP".into());
        assert!(es_transitorio(&error));
    }
}

mod article_data_merge_tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn map_fixture(activo: bool) -> BdpArticleMap {
        BdpArticleMap {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            articulo_glory_codigo: "90000003".into(),
            articulo_bdp_codigo: "90000003".into(),
            articulo_bdp_nombre: "PRUEBA".into(),
            origen: "local".into(),
            local_dirty: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            descripcion: "PRUEBA".into(),
            precio_tarifa1: Decimal::from_str("1.00").unwrap(),
            iva_pct: Decimal::from_str("10.00").unwrap(),
            departamento: 1,
            familia: 1,
            subfamilia: 1,
            activo,
            barcode: String::new(),
            ultima_sync_at: None,
            stock_actual: Decimal::ZERO,
            inventariable: Some(true),
        }
    }

    /* [149A-2/W-Q2.2] Desactivar un map despublica el artículo en BDP. */
    #[test]
    fn map_inactivo_envia_web_article_false() {
        let parcial =
            article_data_desde_map(&ConfiguracionRestaurante::default(), &map_fixture(false))
                .unwrap();
        let v = serde_json::to_value(&parcial).unwrap();
        assert_eq!(v["WebArticle"], false);
    }

    #[test]
    fn map_activo_envia_web_article_true() {
        let parcial =
            article_data_desde_map(&ConfiguracionRestaurante::default(), &map_fixture(true))
                .unwrap();
        let v = serde_json::to_value(&parcial).unwrap();
        assert_eq!(v["WebArticle"], true);
    }

    #[test]
    fn merge_conserva_campos_completos_y_aplica_patch_parcial() {
        let completo = serde_json::json!({
            "ArtCode": 90_000_001,
            "ArtDescription": "Original BDP",
            "Price5": 42.5,
            "WebArticle": true
        });
        let parcial = serde_json::json!({
            "ArtCode": 90_000_001,
            "ArtDescription": "Editado localmente"
        });

        let resultado = fusionar_article_data(completo, &parcial).unwrap();
        assert_eq!(resultado["ArtDescription"], "Editado localmente");
        assert_eq!(resultado["Price5"], 42.5);
        assert_eq!(resultado["WebArticle"], true);
    }

    #[test]
    fn merge_rechaza_article_data_no_objeto() {
        let error =
            fusionar_article_data(serde_json::json!([]), &serde_json::json!({})).unwrap_err();
        assert_eq!(error, "GetArticle devolvió ArticleData no objeto");
    }
}

mod lookup_tav_tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn config_con_map(claves: &[&str], code: i32) -> ConfiguracionRestaurante {
        let mut obj = serde_json::Map::new();
        for clave in claves {
            obj.insert(clave.to_string(), Value::from(code));
        }
        ConfiguracionRestaurante {
            bdp_tav_map: Value::Object(obj),
            ..Default::default()
        }
    }

    #[test]
    fn clave_canonica_entera_matchea_con_escala_2() {
        /* [321A-4] El caso real: iva_pct = 10.00 (numeric(6,2)) y el mapa usa
         * la clave canónica "10" (M13: 10 -> 1). */
        let config = config_con_map(&["10"], 1);
        assert_eq!(
            lookup_tav(&config, Decimal::from_str("10.00").unwrap()),
            Some(1)
        );
    }

    #[test]
    fn clave_con_decimales_significativos_matchea_normalizada() {
        let config = config_con_map(&["10.5"], 3);
        assert_eq!(
            lookup_tav(&config, Decimal::from_str("10.50").unwrap()),
            Some(3)
        );
    }

    #[test]
    fn clave_exacta_con_escala_2_sigue_funcionando() {
        /* Compatibilidad: un mapa configurado manualmente con "10.00". */
        let config = config_con_map(&["10.00"], 9);
        assert_eq!(
            lookup_tav(&config, Decimal::from_str("10.00").unwrap()),
            Some(9)
        );
    }

    #[test]
    fn sin_mapa_devuelve_none() {
        let config = ConfiguracionRestaurante {
            ..Default::default()
        };
        assert_eq!(
            lookup_tav(&config, Decimal::from_str("10.00").unwrap()),
            None
        );
    }

    #[test]
    fn iva_fuera_del_mapa_devuelve_none() {
        let config = config_con_map(&["21"], 2);
        assert_eq!(
            lookup_tav(&config, Decimal::from_str("4.00").unwrap()),
            None
        );
    }
}

/* [D26d] Traspaso entre almacenes: el payload usa las claves PascalCase de BDP. */
mod traspaso_tests {
    use super::super::payloads::payload_traspaso;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn payload_traspaso_usa_claves_bdp() {
        let payload =
            payload_traspaso(90000003, Decimal::from_str("2.5").unwrap(), 7, 1, 2).unwrap();
        assert_eq!(payload["Article"], 90000003);
        assert_eq!(payload["CodTransfer"], 7);
        assert_eq!(payload["StoreFrom"], 1);
        assert_eq!(payload["StoreTo"], 2);
        assert!(payload.get("Units").is_some());
        assert!(payload.get("DateTransfer").is_some());
    }
}

/* [D26e] Ajuste por delta: va a `Warehouse/Regularizations` (BDP regulariza el
 * escandallo). Un delta negativo debe viajar con signo y con el almacén indicado. */
mod regularizacion_tests {
    use super::super::payloads::payload_regularizacion;
    use crate::models::ConfiguracionRestaurante;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn payload_regularizacion_conserva_signo_y_almacen() {
        let config = ConfiguracionRestaurante::default();
        let payload =
            payload_regularizacion(&config, 90000003, Decimal::from_str("-2.5").unwrap(), 2)
                .unwrap();
        assert_eq!(payload["Article"], 90000003);
        assert_eq!(payload["Store"], 2);
        assert!(payload.get("Units").is_some());
        assert!(payload.get("DateReg").is_some());
    }
}
