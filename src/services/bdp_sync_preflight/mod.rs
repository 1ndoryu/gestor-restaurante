/* [065A-4] Dry-run de sincronizacion BDP.
 * Valida datos reales de BDP y usa CreateOrder en modo OnlyCheck para probar
 * el payload de comanda sin escribir clientes, comandas, pagos ni facturas en BDP. */

mod campos;
mod checks;
mod ejecucion;
mod payload;
#[cfg(test)]
mod tests;

use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

use crate::models::ConfiguracionRestaurante;

const BDP_DRY_RUN_MARKET_ID: i32 = 9_901;
const BDP_DRY_RUN_PAGE_SIZE: i32 = 10;
/* [169A-2] Nombres de los checks OnlyCheck del dry-run (clásico y P1 con pago). */
const ONLY_CHECK: &str = "CreateOrder OnlyCheck";
const ONLY_CHECK_CON_PAGO: &str = "CreateOrder OnlyCheck con pago";

#[derive(Debug, Serialize, ToSchema)]
#[allow(clippy::struct_excessive_bools)]
pub struct BdpSyncDryRunResponse {
    pub configurado: bool,
    pub sync_habilitado: bool,
    pub escritura_real: bool,
    pub listo_para_sincronizar: bool,
    pub mensaje: String,
    pub checks: Vec<BdpSyncDryRunCheck>,
    pub payload_preview: Option<Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BdpSyncDryRunCheck {
    pub nombre: String,
    pub endpoint: String,
    pub ok: bool,
    pub mensaje: String,
    pub cantidad: Option<usize>,
    pub muestra: Option<String>,
}

struct BdpDryRunArticle {
    id: i64,
    name: String,
    price: f64,
    vat_pct: f64,
}

pub struct BdpSyncPreflightService;

impl BdpSyncDryRunResponse {
    fn new(sync_habilitado: bool) -> Self {
        Self {
            configurado: false,
            sync_habilitado,
            escritura_real: false,
            listo_para_sincronizar: false,
            mensaje: String::new(),
            checks: Vec::new(),
            payload_preview: None,
        }
    }
}

impl BdpSyncDryRunCheck {
    fn ok(
        name: impl Into<String>,
        endpoint: impl Into<String>,
        message: impl Into<String>,
        count: Option<usize>,
        sample: Option<String>,
    ) -> Self {
        Self {
            nombre: name.into(),
            endpoint: endpoint.into(),
            ok: true,
            mensaje: message.into(),
            cantidad: count,
            muestra: sample,
        }
    }

    fn error(
        name: impl Into<String>,
        endpoint: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            nombre: name.into(),
            endpoint: endpoint.into(),
            ok: false,
            mensaje: message.into(),
            cantidad: None,
            muestra: None,
        }
    }
}

/* [065A-5] pub para uso por BdpSyncService en bdp_sync.rs */
pub fn bdp_configurado(config: &ConfiguracionRestaurante) -> bool {
    !config.bdp_base_url.trim().is_empty()
        && !config.bdp_login.trim().is_empty()
        && !config.bdp_password.trim().is_empty()
        && !config.bdp_integrator_code.trim().is_empty()
}
