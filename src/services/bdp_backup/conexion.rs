use sha2::{Digest, Sha256};

use super::BdpBackupService;
use crate::models::ConfiguracionRestaurante;

impl BdpBackupService {
    /* [187A-1] La evidencia que habilita escritura pertenece a una conexión
     * exacta. El hash evita persistir secretos y hace inelegible cualquier
     * snapshot tomado con otras credenciales o parámetros operativos. */
    pub fn canonical_target(config: &ConfiguracionRestaurante) -> Result<String, String> {
        let raw = config.bdp_base_url.trim().trim_end_matches('/');
        let parsed = reqwest::Url::parse(raw)
            .map_err(|_| "La URL BDP configurada no es válida".to_string())?;
        if !matches!(parsed.scheme(), "http" | "https")
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || parsed.path() != "/"
        {
            return Err(
                "La URL BDP debe ser un origen HTTP(S) sin credenciales, query ni fragmento"
                    .to_string(),
            );
        }
        Ok(raw.to_string())
    }

    pub fn connection_fingerprint(config: &ConfiguracionRestaurante) -> Result<String, String> {
        let mut hasher = Sha256::new();
        for value in [
            Self::canonical_target(config)?,
            config.bdp_login.trim().to_string(),
            config.bdp_password.clone(),
            config.bdp_integrator_code.trim().to_string(),
            config.bdp_pos_id.to_string(),
            config.bdp_employee_id.to_string(),
            config.bdp_items_profile_id.to_string(),
        ] {
            hasher.update(value.as_bytes());
            hasher.update([0]);
        }
        Ok(format!("{:x}", hasher.finalize()))
    }
}

/* [S16-H3] Tests unitarios para canonical_target y connection_fingerprint.
 * Valida que la URL se canonicaliza correctamente y que se rechazan URLs
 * con componentes peligrosos (credenciales, query, fragmento, path). */
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use uuid::Uuid;

    fn config_with_url(url: &str) -> ConfiguracionRestaurante {
        ConfiguracionRestaurante {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            reserva_email_obligatorio: false,
            reserva_telefono_obligatorio: true,
            reserva_nombre_obligatorio: true,
            reserva_apellidos_obligatorio: false,
            iva_por_defecto: Decimal::new(10, 0),
            nombre_restaurante: "Test".to_string(),
            groq_api_key: None,
            auto_venta_reserva: false,
            hora_desayuno_inicio: chrono::NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            hora_desayuno_fin: chrono::NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
            hora_comida_inicio: chrono::NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
            hora_comida_fin: chrono::NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
            hora_cena_inicio: chrono::NaiveTime::from_hms_opt(20, 0, 0).unwrap(),
            hora_cena_fin: chrono::NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
            url_haddock: String::new(),
            haddock_api_token: String::new(),
            haddock_sync_enabled: false,
            bdp_base_url: url.to_string(),
            bdp_login: "user".to_string(),
            bdp_password: "pass".to_string(),
            bdp_integrator_code: "INT".to_string(),
            bdp_configurado: true,
            bdp_sync_enabled: true,
            bdp_pos_id: 1,
            bdp_employee_id: 1,
            bdp_items_profile_id: 1,
            bdp_catalog_price_type: 1,
            bdp_purchase_notes_profile_id: None,
            bdp_documentos_auto_segundos: 0,
            bdp_documentos_perfil: None,
            bdp_maestros_auto_segundos: 0,
            bdp_maestros_ultima_lectura: None,
            bdp_default_article_code: String::new(),
            bdp_default_article_name: String::new(),
            bdp_tender_map: serde_json::json!({}),
            bdp_order_type_map: serde_json::json!({}),
            bdp_default_customer_code: String::new(),
            bdp_poll_interval_secs: 60,
            bdp_poll_enabled: false,
            google_review_url: String::new(),
            telefono_restaurante: String::new(),
            url_reservas: String::new(),
            bdp_auto_sync_customers: false,
            bdp_sync_mode: "read_only".to_string(),
            bdp_backup_retention_days: 30,
            bdp_auto_backup_before_write: true,
            bdp_env_bootstrap_applied_at: None,
            ff_bdp_auto_arm: false,
            ff_bdp_partial_payments: false,
            ff_bdp_cancel_order: false,
            ff_bdp_purchase_notes_read: false,
            ff_bdp_purchase_notes_draft: false,
            ff_bdp_purchase_notes_receive: false,
            modo_operacion: "auto".to_string(),
            anulacion_modalidad: "credito_completo".to_string(),
            permisos_catalogo_edicion: "admin".to_string(),
            permisos_stock_ajuste: "admin".to_string(),
            permisos_albaranes_gestion: "admin".to_string(),
            permisos_anulacion_ventas: "admin".to_string(),
            permisos_pagos_locales: "admin".to_string(),
            permisos_facturacion_local: "admin".to_string(),
            push_modalidad: "automatico".to_string(),
            bdp_tav_map: serde_json::json!({}),
            bdp_almacen_default: 1,
            bdp_codreg_default: 1,
            bdp_articulo_rango_inicial: 90_000_000,
            bdp_responsable_no_api: String::new(),
            bdp_order_end_type: 0,
            bdp_billing_details: false,
            bdp_overwrite_clientes: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /* [S16-H3] canonical_target acepta orígenes HTTP/HTTPS limpios */
    #[test]
    fn canonical_target_accepts_clean_http_origin() {
        let c = config_with_url("http://bdp.example.com:8068");
        assert_eq!(
            BdpBackupService::canonical_target(&c).unwrap(),
            "http://bdp.example.com:8068"
        );
    }

    #[test]
    fn canonical_target_accepts_https_origin() {
        let c = config_with_url("https://bdp.cliente.com");
        assert_eq!(
            BdpBackupService::canonical_target(&c).unwrap(),
            "https://bdp.cliente.com"
        );
    }

    #[test]
    fn canonical_target_strips_trailing_slash() {
        let c = config_with_url("http://127.0.0.1:8068/");
        assert_eq!(
            BdpBackupService::canonical_target(&c).unwrap(),
            "http://127.0.0.1:8068"
        );
    }

    /* [S16-H3] canonical_target rechaza URLs con componentes peligrosos */
    #[test]
    fn canonical_target_rejects_url_with_path() {
        let c = config_with_url("http://bdp.example.com/api");
        assert!(BdpBackupService::canonical_target(&c).is_err());
    }

    #[test]
    fn canonical_target_rejects_url_with_query() {
        let c = config_with_url("http://bdp.example.com?token=abc");
        assert!(BdpBackupService::canonical_target(&c).is_err());
    }

    #[test]
    fn canonical_target_rejects_url_with_fragment() {
        let c = config_with_url("http://bdp.example.com#section");
        assert!(BdpBackupService::canonical_target(&c).is_err());
    }

    #[test]
    fn canonical_target_rejects_url_with_credentials() {
        let c = config_with_url("http://user:pass@bdp.example.com");
        assert!(BdpBackupService::canonical_target(&c).is_err());
    }

    #[test]
    fn canonical_target_rejects_empty_url() {
        let c = config_with_url("");
        assert!(BdpBackupService::canonical_target(&c).is_err());
    }

    /* [S16-H3] connection_fingerprint produce hash determinista */
    #[test]
    fn connection_fingerprint_is_deterministic() {
        let c = config_with_url("http://bdp.example.com:8068");
        let fp1 = BdpBackupService::connection_fingerprint(&c).unwrap();
        let fp2 = BdpBackupService::connection_fingerprint(&c).unwrap();
        assert_eq!(fp1, fp2);
        assert_ne!(fp1, "");
    }

    #[test]
    fn connection_fingerprint_differs_for_different_urls() {
        let c1 = config_with_url("http://bdp-a.example.com");
        let c2 = config_with_url("http://bdp-b.example.com");
        assert_ne!(
            BdpBackupService::connection_fingerprint(&c1).unwrap(),
            BdpBackupService::connection_fingerprint(&c2).unwrap()
        );
    }
}
