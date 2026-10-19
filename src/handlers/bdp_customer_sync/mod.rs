/* [Fase 7.1+7.2] Handlers para sync bidireccional de clientes Glory ↔ BDP.
 * POST /api/bdp/customers/import     — Importar clientes desde BDP a Glory (ExportCustomers)
 * POST /api/clientes/:id/bdp-sync    — Push de un cliente Glory a BDP (CreateCustomer)
 *
 * Patrón: sigue el mismo flujo que bdp_article_map::importar_catalogo.
 * Los campos BDP del cliente se mapean así:
 *   BDP.Customer      → clientes.bdp_customer_code
 *   BDP.FiscalName    → clientes.nombre + apellidos
 *   BDP.MobilePhone   → clientes.telefono
 *   BDP.EMail         → clientes.email
 *
 * Gotchas:
 *   - ExportCustomers puede devolver muchos registros (~43k) → import batch con progreso.
 *   - Matching Glory↔BDP: por teléfono (primario) o email (secundario).
 *   - CreateCustomer requiere `code` (entero) → siempre lo proporciona explícitamente el usuario. */

mod crear_customer;
mod import;
mod sincronizar;

pub use import::*;
pub use sincronizar::*;

use axum::routing::post;
use axum::Router;

use crate::AppState;

use super::rate_limit::{limitador_escrituras, limitar};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/bdp/customers/import", post(importar_clientes_bdp))
        .route("/clientes/:id/bdp-sync", post(sincronizar_cliente_bdp))
        /* Tope por IP de las escrituras de este módulo (ver rate_limit.rs). */
        .layer(axum::middleware::from_fn_with_state(
            limitador_escrituras(),
            limitar,
        ))
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct BdpCustomerSyncRequest {
    /// Código reservado explícitamente para este cliente. Nunca se calcula.
    pub bdp_customer_code: i32,
    /// Debe identificar exactamente el cliente y código que se crearán.
    pub confirmacion: String,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct BdpCustomerImportRequest {
    /// `false` solo calcula el impacto; `true` aplica cambios locales en Glory.
    #[serde(default)]
    pub aplicar: bool,
    /// Al aplicar debe ser exactamente `IMPORTAR CLIENTES BDP`.
    pub confirmacion: Option<String>,
}

pub(super) fn customer_code(value: &serde_json::Value) -> Option<i32> {
    value
        .get("Customer")
        .or_else(|| value.get("Code"))
        .and_then(|code| {
            code.as_i64()
                .or_else(|| code.as_str()?.trim().parse::<i64>().ok())
        })
        .and_then(|code| i32::try_from(code).ok())
        .filter(|code| *code > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::crear_customer::split_name;

    #[test]
    fn customer_code_accepts_number_or_string_and_rejects_invalid() {
        assert_eq!(
            customer_code(&serde_json::json!({"Customer": 42})),
            Some(42)
        );
        assert_eq!(customer_code(&serde_json::json!({"Code": "43"})), Some(43));
        assert_eq!(customer_code(&serde_json::json!({"Customer": 0})), None);
        assert_eq!(
            customer_code(&serde_json::json!({"Customer": "otro"})),
            None
        );
    }

    #[test]
    fn split_name_never_panics_on_empty_or_single_name() {
        assert_eq!(split_name(""), ("Sin nombre", ""));
        assert_eq!(split_name("Ana"), ("Ana", ""));
        assert_eq!(split_name("Ana Pérez"), ("Ana", "Pérez"));
    }
}
