/* [267A-9/F4.1] Contexto del pedido BDP: TenderId, order type y datos del cliente. */

use sqlx::PgPool;
use tracing::{info, warn};

use crate::models::{ConfiguracionRestaurante, Venta};
use crate::repositories::ClienteRepository;
use crate::services::bdp_sync::{BdpSyncService, OrderContext};

impl BdpSyncService {
    /// [F3.1-3.3] Resuelve el contexto del pedido: tender, order type y datos del cliente.
    /// Se ejecuta una sola vez por `sync_venta` y el resultado se reutiliza en `build_order`.
    pub(super) async fn resolve_order_context(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
    ) -> OrderContext {
        /* [F3.2] TenderId: mapear metodo_pago → ID BDP desde config.bdp_tender_map */
        let tender_id = Self::resolve_tender_id(venta, config);

        /* [F3.3] Order type: mapear canal → tipo BDP desde config.bdp_order_type_map */
        let order_type = Self::resolve_order_type(venta, config);

        /* [F3.1] Customer: lookup cliente si existe */
        let (customer_code, customer_name, customer_phone) =
            Self::resolve_customer(pool, venta, config).await;

        OrderContext {
            tender_id,
            order_type,
            customer_code,
            customer_name,
            customer_phone,
        }
    }

    /// [F3.2] Resuelve `TenderId` buscando `venta.metodo_pago` en `bdp_tender_map`.
    /// Ej: `{"efectivo": "1", "tarjeta": "2"}` → `metodo_pago`="Efectivo" → `tender_id=Some(1)`.
    pub(crate) fn resolve_tender_id(
        venta: &Venta,
        config: &ConfiguracionRestaurante,
    ) -> Option<i32> {
        let map = config.bdp_tender_map.as_object()?;
        let key = venta.metodo_pago.to_lowercase();
        let value = map.get(&key)?;
        let id = value
            .as_i64()
            .and_then(|id| i32::try_from(id).ok())
            .or_else(|| value.as_str()?.trim().parse::<i32>().ok())?;
        if id > 0 {
            Some(id)
        } else {
            None
        }
    }

    /// [F3.3] Resuelve el order type buscando venta.canal en `bdp_order_type_map`.
    /// Default: 0 (Barra/Takeaway) si no hay mapeo o el canal no está configurado.
    pub(crate) fn resolve_order_type(venta: &Venta, config: &ConfiguracionRestaurante) -> i32 {
        let Some(map) = config.bdp_order_type_map.as_object() else {
            return 0;
        };
        let key = venta.canal.to_lowercase();
        let Some(value) = map.get(&key) else {
            return 0;
        };
        match value
            .as_i64()
            .and_then(|value| i32::try_from(value).ok())
            .or_else(|| value.as_str().and_then(|s| s.trim().parse::<i32>().ok()))
        {
            Some(t) if t >= 0 => t,
            _ => 0,
        }
    }

    /// [F3.1] Resuelve datos del cliente si la venta tiene `cliente_id`.
    /// Devuelve (code, name, phone) — cada uno puede ser None.
    async fn resolve_customer(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let Some(cliente_id) = venta.cliente_id else {
            /* Sin cliente — usar default_customer_code si existe */
            let code = &config.bdp_default_customer_code;
            if code.is_empty() {
                return (None, None, None);
            }
            return (Some(code.clone()), None, None);
        };

        let user_id = venta.user_id;

        match ClienteRepository::find_by_id(pool, cliente_id, user_id).await {
            Ok(Some(cliente)) => {
                let name = {
                    let full = format!("{} {}", cliente.nombre, cliente.apellidos)
                        .trim()
                        .to_string();
                    if full.is_empty() {
                        None
                    } else {
                        Some(full)
                    }
                };
                let phone = if cliente.telefono.is_empty() {
                    None
                } else {
                    Some(cliente.telefono.clone())
                };
                /* [F7.5] Priorizar bdp_customer_code del cliente sobre default config.
                 * Si el cliente ya fue sincronizado con BDP, usamos su código real.
                 * Si no, fallback al código genérico de config. */
                let code = if let Some(bdp_code) = cliente.bdp_customer_code {
                    Some(bdp_code.to_string())
                } else {
                    let default = config.bdp_default_customer_code.clone();
                    if default.is_empty() {
                        None
                    } else {
                        Some(default)
                    }
                };
                (code, name, phone)
            }
            Ok(None) => {
                info!(
                    "[F3.1] Cliente {} no encontrado para venta {}",
                    cliente_id, venta.id
                );
                (None, None, None)
            }
            Err(e) => {
                warn!("[F3.1] Error buscando cliente {}: {e}", cliente_id);
                (None, None, None)
            }
        }
    }
}
