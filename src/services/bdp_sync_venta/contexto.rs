/* [267A-9/F4.1] Contexto del pedido BDP: TenderId, order type y datos del cliente. */

use sqlx::PgPool;
use tracing::{info, warn};

use crate::models::{ConfiguracionRestaurante, Venta};
use crate::repositories::ClienteRepository;
use crate::services::bdp_sync::{BdpSyncService, OrderContext};

/* [21AB-6] Tipo de pedido de domicilio. BDP solo lo admite en la aplicación
 * «Servicio a Domicilio»; Hostelería responde `300009`. Guillermo pidió el
 * 20/11 dejar la función bloqueada hasta que BDP conteste, así que se corta
 * aquí, antes del HTTP, con un mensaje que dice por qué. */
const ORDER_TYPE_DOMICILIO: i32 = 2;

/// Canales que en la web significan reparto a domicilio. Se cortan por nombre
/// y no solo por el `Type` resultante: sin mapeo caerían en `Type=0` (barra) y
/// se enviaría un pedido de reparto como consumición de barra.
fn canal_es_domicilio(canal: &str) -> bool {
    matches!(
        canal.trim().to_lowercase().as_str(),
        "delivery" | "just_eat"
    )
}

/// Motivo por el que un pedido a domicilio no se envía a Hostelería, o `None`
/// si el pedido puede salir. Pura para poder probarla sin TPV ni base de datos.
fn rechazo_por_domicilio(canal: &str, order_type: i32) -> Option<String> {
    if order_type != ORDER_TYPE_DOMICILIO && !canal_es_domicilio(canal) {
        return None;
    }
    Some(format!(
        "El canal '{canal}' es un pedido a domicilio, que este TPV no admite: BDP solo lo \
         crea en su aplicación Servicio a Domicilio. Pendiente de disponibilidad en BDP."
    ))
}

impl BdpSyncService {
    /// [F3.1-3.3] Resuelve el contexto del pedido: tender, order type y datos del cliente.
    /// Se ejecuta una sola vez por `sync_venta` y el resultado se reutiliza en `build_order`.
    pub(super) async fn resolve_order_context(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
    ) -> Result<OrderContext, String> {
        /* [F3.2] TenderId: mapear metodo_pago → ID BDP desde config.bdp_tender_map */
        let tender_id = Self::resolve_tender_id(venta, config);

        /* [F3.3] Order type: mapear canal → tipo BDP desde config.bdp_order_type_map */
        let order_type = Self::resolve_order_type(venta, config);
        if let Some(motivo) = rechazo_por_domicilio(&venta.canal, order_type) {
            return Err(motivo);
        }

        /* [F3.1] Customer: lookup cliente si existe */
        let (customer_code, customer_name, customer_phone) =
            Self::resolve_customer(pool, venta, config).await;

        Ok(OrderContext {
            tender_id,
            order_type,
            customer_code,
            customer_name,
            customer_phone,
        })
    }

    /// [F3.2] Resuelve `TenderId` buscando `venta.metodo_pago` en `bdp_tender_map`.
    /// Ej: `{"efectivo": "1", "tarjeta": "2"}` → `metodo_pago`="Efectivo" → `tender_id=Some(1)`.
    pub(crate) fn resolve_tender_id(
        venta: &Venta,
        config: &ConfiguracionRestaurante,
    ) -> Option<i32> {
        let map = config.bdp_tender_map.as_object()?;
        /* La clave del mapa puede venir en mayúsculas («Tarjeta», como en el
         * bootstrap de demo): se compara sin distinguir mayúsculas, o el cobro
         * no llevaría TenderId y BDP guardaría la comanda sin pago. */
        let key = venta.metodo_pago.trim().to_lowercase();
        let value = map
            .iter()
            .find_map(|(clave, valor)| (clave.trim().to_lowercase() == key).then_some(valor))?;
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
        let key = venta.canal.trim().to_lowercase();
        /* Igual que el mapa de tenders: la clave puede venir «Comedor». */
        let Some(value) = map
            .iter()
            .find_map(|(clave, valor)| (clave.trim().to_lowercase() == key).then_some(valor))
        else {
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

#[cfg(test)]
mod tests {
    use super::*;

    /* [21AB-6] Guillermo pidió el 20/11 dejar bloqueados los pedidos `Type=2`.
     * El corte tiene que estar en el servidor: la UI sola no es frontera. */
    #[test]
    fn domicilio_se_rechaza_por_canal() {
        for canal in ["delivery", "just_eat", " Delivery ", "JUST_EAT"] {
            let motivo = rechazo_por_domicilio(canal, 0).expect("debe rechazar");
            assert!(motivo.contains("Servicio a Domicilio"), "{motivo}");
            assert!(
                motivo.contains("Pendiente de disponibilidad en BDP"),
                "{motivo}"
            );
        }
    }

    /* Aunque el canal no se llame «delivery», un mapa que lo traduzca a `Type=2`
     * también se rechaza: BDP lo devolvería con `300009`. */
    #[test]
    fn domicilio_se_rechaza_por_order_type() {
        let motivo = rechazo_por_domicilio("eventos", 2).expect("debe rechazar");
        assert!(motivo.contains("'eventos'"), "{motivo}");
    }

    /* Los canales normales del restaurante siguen saliendo. */
    #[test]
    fn canales_normales_pasan() {
        for (canal, tipo) in [("comedor", 1), ("barra", 0), ("terraza", 1), ("eventos", 1)] {
            assert!(
                rechazo_por_domicilio(canal, tipo).is_none(),
                "{canal} con Type={tipo} no debe rechazarse"
            );
        }
    }

    /* Los únicos `Type` que admite Hostelería son 0, 1 (y los de mesa); un 2
     * colado por el mapa no puede pasar por buen canal. */
    #[test]
    fn type_dos_gana_a_cualquier_canal() {
        assert!(rechazo_por_domicilio("comedor", 2).is_some());
        assert!(rechazo_por_domicilio("barra", 2).is_some());
    }
}
