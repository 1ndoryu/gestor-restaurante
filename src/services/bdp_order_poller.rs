// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [276A-4.2] Servicio de polling BDP — consulta periódicamente el estado de comandas
 * enviadas al POS para detectar facturación (Status=3) u otros estados finales.
 *
 * Flujo:
 *   1. VentaRepository::list_bdp_pending() → ventas con bdp_synced=true, bdp_order_status no final
 *   2. Para cada venta: GetOrder(bdp_order_id) → Status de BDP
 *   3. Mapear status integer → string legible: pending(0), accepted(1), cancelled(2), invoiced(3)
 *   4. Actualizar bdp_order_status en la venta
 *
 * Status BDP (de la API Weblink REST, estructura Order):
 *   0 = Esperando validación → "pending"
 *   1 = Aceptada por el establecimiento → "accepted"
 *   2 = Cancelada → "cancelled"
 *   3 = Facturada → "invoiced"
 *
 * Gotchas:
 *   - GetOrder gratuito solo devuelve Status, no el Order completo (limitación subscripción).
 *   - El intervalo de polling se configura en bdp_poll_interval_secs (configuración restaurante).
 *   - Cada venta tiene un mutex para evitar polling concurrente del mismo order.
 *
 * Fachada: el ciclo de polling vive en bdp_order_poller/polling.rs y la reconciliación
 * de auditorías ambiguas en bdp_order_poller/reconciliacion.rs. Las firmas públicas
 * se mantienen aquí para que los llamadores no cambien. */

mod polling;
mod reconciliacion;

pub struct BdpOrderPollerService;

#[cfg(test)]
mod tests {
    use super::reconciliacion::{json_f64, json_i64};
    use super::*;

    #[test]
    fn test_map_status() {
        assert_eq!(BdpOrderPollerService::map_status(0), "pending");
        assert_eq!(BdpOrderPollerService::map_status(1), "accepted");
        assert_eq!(BdpOrderPollerService::map_status(2), "cancelled");
        assert_eq!(BdpOrderPollerService::map_status(3), "invoiced");
        assert_eq!(BdpOrderPollerService::map_status(99), "unknown_99");
    }

    #[test]
    fn status_accepts_numeric_string_shape() {
        let value = serde_json::json!({"Order": {"Status": "3"}});
        assert_eq!(BdpOrderPollerService::parse_status(&value), Ok(3));
        assert!(BdpOrderPollerService::parse_status(&serde_json::json!({"Status": "x"})).is_err());
    }

    #[test]
    fn numeric_evidence_accepts_number_or_string_but_rejects_invalid_values() {
        assert_eq!(json_f64(&serde_json::json!("50.50")), Some(50.5));
        assert_eq!(json_f64(&serde_json::json!(50.5)), Some(50.5));
        assert_eq!(json_f64(&serde_json::json!("NaN")), None);
        assert_eq!(json_f64(&serde_json::Value::Null), None);
        assert_eq!(json_i64(&serde_json::json!("7")), Some(7));
        assert_eq!(json_i64(&serde_json::json!(7)), Some(7));
        assert_eq!(json_i64(&serde_json::json!("7.2")), None);
    }
}
