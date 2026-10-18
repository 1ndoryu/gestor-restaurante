/* [267A-9/F4.1] Utilidades de sync_venta: conversión Decimal→f64, sanitizado de
 * errores, limpieza de locks locales y auto-sync de cliente. */

use std::sync::Arc;

use sqlx::PgPool;
use tracing::warn;

use crate::models::ConfiguracionRestaurante;
use crate::repositories::ClienteRepository;
use crate::services::bdp_sync::{BdpSyncService, SYNC_LOCKS};

impl BdpSyncService {
    /* [R16] Conversión Decimal → f64 para serialización JSON a BDP.
     * Se convierte vía string para máxima precisión; es el enfoque más fiable
     * para Decimal→f64. El redondeo monetario se aplica en los call-sites
     * que lo necesiten, no aquí (vat_pct es un porcentaje, no moneda). */
    pub(crate) fn decimal_to_f64(d: &rust_decimal::Decimal) -> f64 {
        use std::str::FromStr;
        match f64::from_str(&d.to_string()) {
            Ok(v) => v,
            Err(e) => {
                warn!("[R16] Error convirtiendo Decimal '{d}' a f64: {e}");
                0.0
            }
        }
    }

    pub(crate) fn sanitize_error(raw: &str) -> String {
        if raw.contains("401") || raw.contains("403") {
            "Error de autenticación con BDP (401/403)".to_string()
        } else if raw.contains("300035") {
            "BDP: serie no válida (300035)".to_string()
        } else if raw.contains("300008") {
            "BDP: salón/mesa no válidos (300008)".to_string()
        } else if raw.contains("300009") {
            "BDP: delivery no soportado en este POS (300009)".to_string()
        } else if raw.contains("301011") {
            "BDP: MarketplaceOrderId demasiado largo (301011)".to_string()
        } else if raw.contains("301400") {
            "BDP: caja cerrada (301400)".to_string()
        } else {
            let truncated: String = raw.chars().take(200).collect();
            format!("Error BDP: {truncated}")
        }
    }

    pub(crate) fn cleanup_lock(venta_id: uuid::Uuid) {
        let mut map = SYNC_LOCKS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(entry) = map.get(&venta_id) {
            if Arc::strong_count(entry) <= 2 {
                map.remove(&venta_id);
            }
        }
        /* [AUDIT-N3] Sweep periódico: eliminar entradas huérfanas cuyo Arc
         * solo vive en el HashMap (strong_count == 1). Esto previene leak
         * de memoria cuando cleanup_lock no se llama por panic o early return. */
        if map.len() > 100 {
            map.retain(|_, arc| Arc::strong_count(arc) > 1);
        }
    }

    /// [F7.5] Auto-sync de cliente Glory → BDP.
    /// Devuelve únicamente un código BDP previamente confirmado. La creación
    /// automática quedó deliberadamente deshabilitada: `max + 1` y hashes no
    /// pueden garantizar ausencia de colisiones con otros escritores del TPV.
    pub async fn ensure_cliente_bdp_synced(
        pool: &PgPool,
        cliente_id: uuid::Uuid,
        user_id: uuid::Uuid,
        _config: &ConfiguracionRestaurante,
    ) -> Option<i32> {
        let Ok(Some(cliente)) = ClienteRepository::find_by_id(pool, cliente_id, user_id).await
        else {
            return None;
        };
        if let Some(code) = cliente.bdp_customer_code {
            return (code > 0).then_some(code);
        }
        let msg = "Creación automática BDP deshabilitada: asigne y verifique un código explícito desde la sincronización manual";
        let _ = ClienteRepository::update_bdp_sync(pool, cliente_id, None, false, Some(msg)).await;
        None
    }
}
