/* [128A-1/F1] Servicio del conmutador de modo operativo BDP (independencia
 * total del BDP). Decide el modo efectivo por usuario:
 * - 'auto': bdp si bdp_configurado() y bdp_sync_enabled; si no, standalone.
 * - 'standalone': nunca se llama a BDP (providers locales).
 * - 'bdp': fuerza modo BDP.
 * Invariantes M1: modo_operacion es el switch maestro; bdp_sync_enabled solo
 * se interpreta cuando el modo efectivo es bdp. M3: la cache se invalida al
 * actualizar la configuración (el handler de PATCH lo hace al persistir). */

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::ConfiguracionRestaurante;
use crate::services::bdp_sync_preflight::bdp_configurado;
use crate::services::ConfiguracionService;

pub const MODO_AUTO: &str = "auto";
pub const MODO_STANDALONE: &str = "standalone";
pub const MODO_BDP: &str = "bdp";

const TTL: Duration = Duration::from_mins(1);

/// Modo efectivo derivado para un usuario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModoEfectivo {
    Standalone,
    Bdp,
}

impl ModoEfectivo {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Standalone => MODO_STANDALONE,
            Self::Bdp => MODO_BDP,
        }
    }
}

#[derive(Default)]
struct EntradaCache {
    modo: Option<ModoEfectivo>,
    creada_en: Option<Instant>,
}

/// [128A-1/F1] Servicio del conmutador. Cache en memoria por proceso con TTL
/// (riesgo multi-proceso documentado en el plan §5: 2 TPV pueden ver modos
/// distintos hasta expirar el TTL; aceptado como riesgo abierto).
#[derive(Default, Clone)]
pub struct ServicioModoOperacion {
    cache: Arc<Mutex<Vec<(Uuid, EntradaCache)>>>,
}

impl ServicioModoOperacion {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Devuelve el modo efectivo sin tocar BDP (nunca hace red).
    #[must_use]
    pub fn modo_efectivo_desde_config(config: &ConfiguracionRestaurante) -> ModoEfectivo {
        match config.modo_operacion.as_str() {
            MODO_STANDALONE => ModoEfectivo::Standalone,
            MODO_BDP => ModoEfectivo::Bdp,
            /* auto: el switch maestro deriva del estado de sincronización. */
            _ => {
                if config.bdp_sync_enabled && bdp_configurado(config) {
                    ModoEfectivo::Bdp
                } else {
                    ModoEfectivo::Standalone
                }
            }
        }
    }

    /// Modo efectivo con cache TTL (60 s) por usuario. La degradación reactiva
    /// (histéresis M2) y el preflight ligero se añaden en la fase F1.2 del plan.
    pub async fn modo_efectivo(
        &self,
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<ModoEfectivo, AppError> {
        let now = Instant::now();
        if let Some(modo) = self.desde_cache(user_id, now) {
            return Ok(modo);
        }
        let config = ConfiguracionService::obtener(pool, user_id).await?;
        let modo = Self::modo_efectivo_desde_config(&config);
        self.guardar_cache(user_id, modo, now);
        Ok(modo)
    }

    /// [M3] Invalida la cache del usuario al guardar configuración.
    pub fn invalidar(&self, user_id: Uuid) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.retain(|(id, _)| *id != user_id);
        }
    }

    fn desde_cache(&self, user_id: Uuid, now: Instant) -> Option<ModoEfectivo> {
        let cache = self.cache.lock().ok()?;
        cache.iter().find_map(|(id, entrada)| {
            if *id == user_id {
                entrada.creada_en.and_then(|creada| {
                    if now.duration_since(creada) < TTL {
                        entrada.modo
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        })
    }

    fn guardar_cache(&self, user_id: Uuid, modo: ModoEfectivo, now: Instant) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.retain(|(id, _)| *id != user_id);
            cache.push((
                user_id,
                EntradaCache {
                    modo: Some(modo),
                    creada_en: Some(now),
                },
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveTime, Utc};
    use rust_decimal::Decimal;
    use uuid::Uuid;

    fn config(modo: &str, sync_enabled: bool, credenciales: bool) -> ConfiguracionRestaurante {
        ConfiguracionRestaurante {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            reserva_email_obligatorio: false,
            reserva_telefono_obligatorio: true,
            reserva_nombre_obligatorio: true,
            reserva_apellidos_obligatorio: false,
            iva_por_defecto: Decimal::new(10, 0),
            nombre_restaurante: "Nakomi".to_string(),
            groq_api_key: None,
            auto_venta_reserva: false,
            hora_desayuno_inicio: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            hora_desayuno_fin: NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
            hora_comida_inicio: NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
            hora_comida_fin: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
            hora_cena_inicio: NaiveTime::from_hms_opt(20, 0, 0).unwrap(),
            hora_cena_fin: NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
            url_haddock: String::new(),
            haddock_api_token: String::new(),
            haddock_sync_enabled: false,
            bdp_base_url: if credenciales {
                "http://bdp.test".to_string()
            } else {
                String::new()
            },
            bdp_login: if credenciales {
                "usuario".to_string()
            } else {
                String::new()
            },
            bdp_password: if credenciales {
                "secreto".to_string()
            } else {
                String::new()
            },
            bdp_integrator_code: if credenciales {
                "INTEGRADOR".to_string()
            } else {
                String::new()
            },
            bdp_sync_enabled: sync_enabled,
            bdp_pos_id: 31,
            bdp_employee_id: 1,
            bdp_items_profile_id: 1,
            bdp_catalog_price_type: 1,
            bdp_purchase_notes_profile_id: None,
            bdp_default_article_code: "GLORY".to_string(),
            bdp_default_article_name: "Servicio Glory".to_string(),
            bdp_tender_map: serde_json::json!({"efectivo": "1", "tarjeta": "2"}),
            bdp_order_type_map: serde_json::json!({"comedor": "0", "barra": "0"}),
            bdp_default_customer_code: "DEFAULT".to_string(),
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
            modo_operacion: modo.to_string(),
            anulacion_modalidad: "credito_completo".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn auto_sin_credenciales_es_standalone() {
        let cfg = config("auto", true, false);
        assert_eq!(
            ServicioModoOperacion::modo_efectivo_desde_config(&cfg),
            ModoEfectivo::Standalone
        );
    }

    #[test]
    fn auto_con_credenciales_y_sync_es_bdp() {
        let cfg = config("auto", true, true);
        assert_eq!(
            ServicioModoOperacion::modo_efectivo_desde_config(&cfg),
            ModoEfectivo::Bdp
        );
    }

    #[test]
    fn auto_sin_sync_es_standalone_aunque_haya_credenciales() {
        let cfg = config("auto", false, true);
        assert_eq!(
            ServicioModoOperacion::modo_efectivo_desde_config(&cfg),
            ModoEfectivo::Standalone
        );
    }

    #[test]
    fn standalone_fuerza_standalone_aunque_sync_este_activo() {
        let cfg = config("standalone", true, true);
        assert_eq!(
            ServicioModoOperacion::modo_efectivo_desde_config(&cfg),
            ModoEfectivo::Standalone
        );
    }

    #[test]
    fn bdp_fuerza_bdp_aunque_sync_este_inactivo() {
        let cfg = config("bdp", false, true);
        assert_eq!(
            ServicioModoOperacion::modo_efectivo_desde_config(&cfg),
            ModoEfectivo::Bdp
        );
    }
}
