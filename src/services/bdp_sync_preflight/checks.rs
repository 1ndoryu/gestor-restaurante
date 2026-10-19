use serde_json::Value;
use sqlx::PgPool;

use super::campos::{number_i64, summarize_value, tender_ids_presentes, value_array};
use super::payload::{build_only_check_order, build_only_check_order_con_pago, first_article};
use super::{
    BdpSyncDryRunCheck, BdpSyncDryRunResponse, BdpSyncPreflightService, ONLY_CHECK,
    ONLY_CHECK_CON_PAGO,
};
use crate::models::ConfiguracionRestaurante;
use crate::repositories::BdpArticleMapRepository;
use crate::services::bdp_weblink::BdpWeblinkClient;

impl BdpSyncPreflightService {
    pub(super) fn check_employee_is_allowed(
        config: &ConfiguracionRestaurante,
        employees: Option<&Value>,
        response: &mut BdpSyncDryRunResponse,
    ) {
        let Some(employees) = employees else {
            response.checks.push(BdpSyncDryRunCheck::error(
                "Empleado permitido en POS",
                "/API/POS/Employees/Get",
                "No se pudo leer la lista de empleados del POS",
            ));
            return;
        };
        let ok = value_array(employees, &["Employees"]).is_some_and(|items| {
            items
                .iter()
                .any(|item| number_i64(item, &["Id"]) == Some(i64::from(config.bdp_employee_id)))
        });
        let check = if ok {
            BdpSyncDryRunCheck::ok(
                "Empleado permitido en POS",
                "/API/POS/Employees/Get",
                "El empleado configurado aparece asociado al terminal",
                None,
                Some(config.bdp_employee_id.to_string()),
            )
        } else {
            BdpSyncDryRunCheck::error(
                "Empleado permitido en POS",
                "/API/POS/Employees/Get",
                format!(
                    "El empleado {} no aparece en el terminal {}",
                    config.bdp_employee_id, config.bdp_pos_id
                ),
            )
        };
        response.checks.push(check);
    }

    /// [F2.7] Verificar que hay mapeos de artículos en `bdp_article_map`.
    /// Si no hay ninguno, la sync usará el artículo default para todas las líneas.
    pub(super) async fn check_article_mapping(
        pool: &PgPool,
        user_id: uuid::Uuid,
        config: &ConfiguracionRestaurante,
        response: &mut BdpSyncDryRunResponse,
    ) {
        match BdpArticleMapRepository::listar(pool, user_id).await {
            Ok(mapas) if !mapas.is_empty() => {
                response.checks.push(BdpSyncDryRunCheck::ok(
                    "Mapeo de articulos",
                    "bdp_article_map",
                    format!("{} articulos mapeados Glory→BDP", mapas.len()),
                    Some(mapas.len()),
                    mapas.first().map(|m| {
                        format!(
                            "{} → {} ({})",
                            m.articulo_glory_codigo, m.articulo_bdp_codigo, m.articulo_bdp_nombre
                        )
                    }),
                ));
            }
            Ok(_) => {
                /* Sin mapeos — no es error pero conviene avisar */
                let msg = if config.bdp_default_article_code.is_empty() {
                    "Sin mapeos y sin articulo default configurado. Las lineas no tendran resolucion.".to_string()
                } else {
                    format!(
                        "Sin mapeos de articulos. Todas las lineas usaran el default: '{}' ({})",
                        config.bdp_default_article_code, config.bdp_default_article_name
                    )
                };
                response.checks.push(BdpSyncDryRunCheck {
                    nombre: "Mapeo de articulos".into(),
                    endpoint: "bdp_article_map".into(),
                    ok: true, /* Warning, no blocker */
                    mensaje: msg,
                    cantidad: Some(0),
                    muestra: None,
                });
            }
            Err(e) => {
                response.checks.push(BdpSyncDryRunCheck::error(
                    "Mapeo de articulos",
                    "bdp_article_map",
                    format!("Error consultando mapeos: {e}"),
                ));
            }
        }
    }

    /// [F3.4] Validar que los `TenderIds` en `bdp_tender_map` existen en el POS.
    /// Compara las claves/valores del JSONB contra la lista de tenders devuelta por BDP.
    pub(super) fn check_tender_mapping(
        config: &ConfiguracionRestaurante,
        tenders: Option<&Value>,
        response: &mut BdpSyncDryRunResponse,
    ) {
        let map = match config.bdp_tender_map.as_object() {
            Some(m) if !m.is_empty() => m,
            _ => {
                response.checks.push(BdpSyncDryRunCheck {
                    nombre: "Mapeo de tenders".into(),
                    endpoint: "bdp_tender_map".into(),
                    ok: true,
                    mensaje: "Sin mapeo de tenders configurado. No se enviara TenderId.".into(),
                    cantidad: Some(0),
                    muestra: None,
                });
                return;
            }
        };

        let Some(tenders_value) = tenders else {
            response.checks.push(BdpSyncDryRunCheck::error(
                "Mapeo de tenders",
                "bdp_tender_map",
                "No se pudo obtener la lista de tenders del POS para validar",
            ));
            return;
        };

        /* Extraer IDs válidos del POS */
        let valid_ids: Vec<i64> = tender_ids_presentes(Some(tenders_value));

        let mut issues = Vec::new();
        let mut mapped = 0;
        for (key, value) in map {
            let Some(tender_str) = value.as_str() else {
                issues.push(format!("{key}: valor no es string"));
                continue;
            };
            let Ok(tender_id) = tender_str.parse::<i64>() else {
                issues.push(format!("{key}: '{tender_str}' no es numérico"));
                continue;
            };
            if valid_ids.is_empty() || valid_ids.contains(&tender_id) {
                mapped += 1;
            } else {
                issues.push(format!(
                    "{key}→{tender_id}: no existe en POS (disponibles: {valid_ids:?})"
                ));
            }
        }

        if issues.is_empty() {
            response.checks.push(BdpSyncDryRunCheck::ok(
                "Mapeo de tenders",
                "bdp_tender_map",
                format!("{mapped} tenders mapeados correctamente"),
                Some(mapped),
                None,
            ));
        } else {
            response.checks.push(BdpSyncDryRunCheck::error(
                "Mapeo de tenders",
                "bdp_tender_map",
                format!("{} problema(s): {}", issues.len(), issues.join("; ")),
            ));
        }
    }

    /// [F3.5] Validar que los order types en `bdp_order_type_map` son válidos (>= 0).
    /// BDP Type: 0=Barra, 1=Mesa, 2=Delivery, etc. Valores negativos son inválidos.
    pub(super) fn check_order_type_mapping(
        config: &ConfiguracionRestaurante,
        response: &mut BdpSyncDryRunResponse,
    ) {
        let map = match config.bdp_order_type_map.as_object() {
            Some(m) if !m.is_empty() => m,
            _ => {
                response.checks.push(BdpSyncDryRunCheck {
                    nombre: "Mapeo de order types".into(),
                    endpoint: "bdp_order_type_map".into(),
                    ok: true,
                    mensaje: "Sin mapeo de order types. Todas las ventas usaran Type=0 (Barra)."
                        .into(),
                    cantidad: Some(0),
                    muestra: None,
                });
                return;
            }
        };

        let mut issues = Vec::new();
        for (key, value) in map {
            match value.as_str().and_then(|s| s.parse::<i32>().ok()) {
                Some(t) if t >= 0 => {}
                Some(t) => issues.push(format!("{key}: Type={t} es negativo")),
                None => issues.push(format!("{key}: valor '{value}' no es numérico válido")),
            }
        }

        if issues.is_empty() {
            response.checks.push(BdpSyncDryRunCheck::ok(
                "Mapeo de order types",
                "bdp_order_type_map",
                format!("{} canales mapeados", map.len()),
                Some(map.len()),
                None,
            ));
        } else {
            response.checks.push(BdpSyncDryRunCheck::error(
                "Mapeo de order types",
                "bdp_order_type_map",
                format!("{} problema(s): {}", issues.len(), issues.join("; ")),
            ));
        }
    }

    pub(super) async fn check_order_only(
        config: &ConfiguracionRestaurante,
        client: &BdpWeblinkClient<'_>,
        articles: Option<&Value>,
        tenders: Option<&Value>,
        con_pago_tender: Option<i32>,
        response: &mut BdpSyncDryRunResponse,
    ) {
        /* [R12] Pasar iva_por_defecto de config como fallback */
        let default_iva = {
            use rust_decimal::prelude::ToPrimitive;
            config.iva_por_defecto.to_f64().unwrap_or(10.0)
        };
        let Some(article) = articles.and_then(|a| first_article(a, default_iva)) else {
            response.checks.push(BdpSyncDryRunCheck::error(
                "CreateOrder OnlyCheck",
                "/API/Orders/Create",
                "No hay un articulo valido para construir la comanda de prueba",
            ));
            return;
        };

        /* [169A-2] Con tender se valida el payload de pago-en-creación en modo
         * OnlyCheck (OperationType=1: BDP valida sin crear comanda, pago ni
         * factura). El tender debe existir en el POS; si no, error local sin
         * llamar a BDP. Sin tender, el dry-run sin pago de siempre. */
        let (request, nombre) = match con_pago_tender {
            None => (build_only_check_order(config, &article), ONLY_CHECK),
            Some(tender_id) => {
                if tender_id <= 0 || !tender_ids_presentes(tenders).contains(&i64::from(tender_id))
                {
                    response.checks.push(BdpSyncDryRunCheck::error(
                        ONLY_CHECK_CON_PAGO,
                        "/API/Orders/Create",
                        format!("tender_id={tender_id} no existe en el POS; no se llamo a BDP"),
                    ));
                    return;
                }
                (
                    build_only_check_order_con_pago(config, &article, tender_id),
                    ONLY_CHECK_CON_PAGO,
                )
            }
        };
        response.payload_preview = serde_json::to_value(&request).ok();
        let check = match client.check_order(&request).await {
            Ok(value) => BdpSyncDryRunCheck::ok(
                nombre,
                "/API/Orders/Create",
                "El destino aceptó OnlyCheck; no se asume que una instalación real sea no persistente",
                None,
                summarize_value(&value, &["OrderId", "InvoiceNumber"]),
            ),
            Err(error) => BdpSyncDryRunCheck::error(
                nombre,
                "/API/Orders/Create",
                format!("BDP rechazo el payload de comanda: {error}"),
            ),
        };
        response.checks.push(check);
    }
}
