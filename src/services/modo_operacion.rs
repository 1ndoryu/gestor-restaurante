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

/* [128A-1/F1/M2] Umbral de fallos BDP consecutivos a partir del cual el modo
 * efectivo degrada a standalone (histéresis reactiva mínima en memoria). */
const UMBRAL_FALLOS_BDP: u32 = 3;

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
    /* [M2] Fallos consecutivos hacia BDP registrados por el poller/sync. */
    fallos_consecutivos: u32,
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

    /// Devuelve el modo efectivo derivado solo de la configuración, sin tocar
    /// BDP (nunca hace red) y sin considerar la histéresis M2.
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

    /// [M2] Modo efectivo considerando la histéresis reactiva: si la
    /// configuración derivaría `Bdp` pero hay `UMBRAL_FALLOS_BDP` fallos
    /// consecutivos hacia BDP registrados para este usuario, degrada a
    /// `Standalone`. No hace red ni usa cache TTL.
    #[must_use]
    pub fn modo_efectivo_sin_red(&self, config: &ConfiguracionRestaurante) -> ModoEfectivo {
        let base = Self::modo_efectivo_desde_config(config);
        if base == ModoEfectivo::Bdp && self.degradado(config.user_id) {
            ModoEfectivo::Standalone
        } else {
            base
        }
    }

    /// Modo efectivo con cache TTL (60 s) por usuario y degradación M2.
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
        let modo = self.modo_efectivo_sin_red(&config);
        self.guardar_cache(user_id, modo, now);
        Ok(modo)
    }

    /// [M3] Invalida la cache del usuario al guardar configuración.
    pub fn invalidar(&self, user_id: Uuid) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.retain(|(id, _)| *id != user_id);
        }
    }

    /// [M2] Registra un fallo hacia BDP para el usuario. Al alcanzar
    /// `UMBRAL_FALLOS_BDP` fallos consecutivos, `modo_efectivo_sin_red` y
    /// `modo_efectivo` degradan a standalone. Invalida el modo cacheado para
    /// que la siguiente entrada re-evalúe con el contador actualizado.
    pub fn registrar_fallo_bdp(&self, user_id: Uuid) {
        if let Ok(mut cache) = self.cache.lock() {
            if let Some((_, entrada)) = cache.iter_mut().find(|(id, _)| *id == user_id) {
                entrada.fallos_consecutivos = entrada
                    .fallos_consecutivos
                    .saturating_add(1)
                    .min(UMBRAL_FALLOS_BDP);
                /* Re-evaluar en la próxima consulta (M2: no cambiar a mitad de operación). */
                entrada.modo = None;
                entrada.creada_en = None;
            } else {
                cache.push((
                    user_id,
                    EntradaCache {
                        modo: None,
                        creada_en: None,
                        fallos_consecutivos: 1,
                    },
                ));
            }
        }
    }

    /// [M2] Registra un éxito hacia BDP: resetea el contador de fallos y
    /// re-evalúa el modo en la próxima consulta.
    pub fn registrar_exito_bdp(&self, user_id: Uuid) {
        if let Ok(mut cache) = self.cache.lock() {
            if let Some((_, entrada)) = cache.iter_mut().find(|(id, _)| *id == user_id) {
                entrada.fallos_consecutivos = 0;
                entrada.modo = None;
                entrada.creada_en = None;
            }
        }
    }

    fn degradado(&self, user_id: Uuid) -> bool {
        self.cache.lock().ok().is_some_and(|cache| {
            cache
                .iter()
                .find(|(id, _)| *id == user_id)
                .is_some_and(|(_, entrada)| entrada.fallos_consecutivos >= UMBRAL_FALLOS_BDP)
        })
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
            /* [128A-1/F1-4] Podar entradas expiradas al insertar para que la
             * cache no crezca de forma monótona con usuarios inactivos. */
            cache.retain(|(id, entrada)| {
                *id == user_id
                    || entrada
                        .creada_en
                        .is_none_or(|creada| now.duration_since(creada) < TTL)
            });
            cache.retain(|(id, _)| *id != user_id);
            cache.push((
                user_id,
                EntradaCache {
                    modo: Some(modo),
                    creada_en: Some(now),
                    fallos_consecutivos: 0,
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
            permisos_catalogo_edicion: "admin".to_string(),
            permisos_stock_ajuste: "admin".to_string(),
            permisos_albaranes_gestion: "admin".to_string(),
            permisos_anulacion_ventas: "admin".to_string(),
            permisos_pagos_locales: "admin".to_string(),
            permisos_facturacion_local: "admin".to_string(),
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

    /* [128A-1/F1-2] M2: histéresis reactiva mínima. Tres fallos consecutivos
     * degradan a standalone; un éxito resetea y restaura Bdp. */
    #[test]
    fn m2_degrada_a_standalone_tras_tres_fallos_consecutivos() {
        let cfg = config("auto", true, true);
        let servicio = ServicioModoOperacion::new();
        assert_eq!(
            servicio.modo_efectivo_sin_red(&cfg),
            ModoEfectivo::Bdp,
            "sin fallos el modo derivado es Bdp"
        );
        servicio.registrar_fallo_bdp(cfg.user_id);
        servicio.registrar_fallo_bdp(cfg.user_id);
        assert_eq!(
            servicio.modo_efectivo_sin_red(&cfg),
            ModoEfectivo::Bdp,
            "con menos de 3 fallos no degrada"
        );
        servicio.registrar_fallo_bdp(cfg.user_id);
        assert_eq!(
            servicio.modo_efectivo_sin_red(&cfg),
            ModoEfectivo::Standalone,
            "3 fallos consecutivos degradan a standalone"
        );
    }

    #[test]
    fn m2_exito_resetea_el_contador_y_restaura_bdp() {
        let cfg = config("auto", true, true);
        let servicio = ServicioModoOperacion::new();
        servicio.registrar_fallo_bdp(cfg.user_id);
        servicio.registrar_fallo_bdp(cfg.user_id);
        servicio.registrar_fallo_bdp(cfg.user_id);
        assert_eq!(
            servicio.modo_efectivo_sin_red(&cfg),
            ModoEfectivo::Standalone
        );
        servicio.registrar_exito_bdp(cfg.user_id);
        assert_eq!(
            servicio.modo_efectivo_sin_red(&cfg),
            ModoEfectivo::Bdp,
            "un éxito resetea el contador y restaura Bdp"
        );
    }

    #[test]
    fn m2_no_degrada_en_modo_standalone_forzado() {
        let cfg = config("standalone", true, true);
        let servicio = ServicioModoOperacion::new();
        servicio.registrar_fallo_bdp(cfg.user_id);
        servicio.registrar_fallo_bdp(cfg.user_id);
        servicio.registrar_fallo_bdp(cfg.user_id);
        assert_eq!(
            servicio.modo_efectivo_sin_red(&cfg),
            ModoEfectivo::Standalone
        );
    }

    /* [128A-1/F1-4] La cache poda entradas expiradas al insertar. */
    #[test]
    fn cache_purga_entradas_expiradas_al_insertar() {
        let servicio = ServicioModoOperacion::new();
        let usuario_viejo = Uuid::new_v4();
        let ahora = Instant::now();
        /* Entrada expirada (creada hace TTL + 1s). */
        servicio.guardar_cache(
            usuario_viejo,
            ModoEfectivo::Standalone,
            ahora.checked_sub(TTL + Duration::from_secs(1)).unwrap(),
        );
        /* Insertar una entrada nueva para otro usuario debe podar la expirada. */
        let usuario_nuevo = Uuid::new_v4();
        servicio.guardar_cache(usuario_nuevo, ModoEfectivo::Bdp, ahora);
        assert!(
            servicio.desde_cache(usuario_viejo, ahora).is_none(),
            "la entrada expirada se purgó"
        );
        assert_eq!(
            servicio.desde_cache(usuario_nuevo, ahora),
            Some(ModoEfectivo::Bdp)
        );
    }
}
