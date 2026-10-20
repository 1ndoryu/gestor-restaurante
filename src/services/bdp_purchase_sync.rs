//! Importación de albaranes de compra desde BDP (`ExportPurchaseNotes`).
//!
//! La usan el endpoint manual y el disparo automático (D28a): ambos pasan por
//! `importar`, así la validación de respuesta y el upsert no se duplican.

use chrono::{Duration, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{BdpPurchaseNoteSyncResult, ConfiguracionRestaurante};
use crate::repositories::BdpPurchaseNoteRepository;
use crate::services::bdp_weblink::BdpWeblinkError;
use crate::services::bdp_weblink_catalog::{
    BdpExportPurchaseNotesRequest, BdpExportPurchaseNotesResponse, BdpPurchaseNoteData,
};
use crate::services::{BdpWeblinkClient, ConfiguracionService, ModoEfectivo, ServicioModoOperacion};

/// Límite de BDP y del endpoint manual: un rango de 31 días como máximo.
pub const DIAS_MAX_RANGO: i64 = 31;
/// Intervalo por defecto del disparo automático, en segundos. 0 lo desactiva.
const SEGUNDOS_AUTO_POR_DEFECTO: u64 = 300;

/// Importa los albaranes del rango y devuelve cuántos se guardaron y cuántos envió BDP.
pub async fn importar(
    pool: &PgPool,
    user_id: Uuid,
    config: &ConfiguracionRestaurante,
    req: &BdpExportPurchaseNotesRequest,
) -> Result<BdpPurchaseNoteSyncResult, AppError> {
    let client = BdpWeblinkClient::new(config);
    let bdp_response: serde_json::Value = client
        .export_purchase_notes(req)
        .await
        .map_err(|e| map_bdp_error(&e))?;

    let parsed: BdpExportPurchaseNotesResponse =
        serde_json::from_value(bdp_response).map_err(|e| {
            AppError::Internal(format!(
                "Respuesta de BDP no tiene el formato esperado: {e}"
            ))
        })?;

    let total_bdp = parsed.documents_lists.len();
    let mut procesados = 0;

    for note in parsed.documents_lists {
        if !albaran_tiene_clave(&note) {
            tracing::warn!(
                "[247A-11] Albarán descartado por no tener serie ni número: {:?}",
                note.extra
            );
            continue;
        }
        match BdpPurchaseNoteRepository::upsert_from_bdp(pool, user_id, &note).await {
            Ok(true) => procesados += 1,
            Ok(false) => {}
            Err(e) => {
                tracing::warn!("[247A-11] Error guardando albarán: {e}");
            }
        }
    }

    Ok(BdpPurchaseNoteSyncResult {
        procesados,
        total_bdp,
    })
}

/// Importa los albaranes de todas las cuentas que tienen el flag activo, en modo BDP
/// y con BDP configurado. Un fallo de una cuenta se registra y no corta a las demás.
/// Devuelve el total de albaranes guardados en este ciclo.
pub async fn sincronizar_automatico(
    pool: &PgPool,
    modo: &ServicioModoOperacion,
    dias: i64,
) -> usize {
    let usuarios: Vec<Uuid> = match sqlx::query_scalar("SELECT user_id FROM configuracion_restaurante")
        .fetch_all(pool)
        .await
    {
        Ok(usuarios) => usuarios,
        Err(error) => {
            tracing::warn!("[D28a] No se pudieron listar cuentas para albaranes: {error}");
            return 0;
        }
    };

    let hoy = Utc::now().date_naive();
    let (desde, hasta) = ventana_automatica(hoy, dias);
    let mut total = 0;
    for user_id in usuarios {
        if !matches!(modo.modo_efectivo(pool, user_id).await, Ok(ModoEfectivo::Bdp)) {
            continue;
        }
        let Ok(config) = ConfiguracionService::obtener(pool, user_id).await else {
            continue;
        };
        if !config.ff_bdp_purchase_notes_read
            || config.bdp_base_url.is_empty()
            || config.bdp_login.is_empty()
        {
            continue;
        }
        // Sin plantilla de exportación no hay forma de pedir los albaranes: se omite la cuenta.
        let Some(perfil) = config.bdp_purchase_notes_profile_id.filter(|c| *c > 0) else {
            continue;
        };
        let req = rango_automatico(perfil, &desde, &hasta);
        match importar(pool, user_id, &config, &req).await {
            Ok(resultado) => total += resultado.procesados,
            Err(error) => tracing::warn!("[D28a] Importación automática fallida: {error:?}"),
        }
    }
    total
}

/// Ventana del disparo automático: los últimos `dias` días hasta hoy, acotados a 1..=31.
pub fn ventana_automatica(hoy: NaiveDate, dias: i64) -> (String, String) {
    let dias = dias.clamp(1, DIAS_MAX_RANGO);
    let desde = hoy - Duration::days(dias);
    (desde.format("%Y-%m-%d").to_string(), hoy.format("%Y-%m-%d").to_string())
}

/// Petición a BDP para el rango automático: sin filtro de proveedor, como el endpoint manual.
fn rango_automatico(perfil: i32, desde: &str, hasta: &str) -> BdpExportPurchaseNotesRequest {
    BdpExportPurchaseNotesRequest {
        export_profile_code: perfil,
        initial_date: Some(desde.to_string()),
        final_date: Some(hasta.to_string()),
        // BDP rechaza proveedores omitidos con 403900 (ver [287A-4]); se envía el rango completo.
        initial_supplier: Some(1),
        final_supplier: Some(999_999),
        initial_serial: None,
        final_serial: None,
    }
}

/// Intervalo del disparo automático, en segundos, leído de `BDP_ALBARANES_AUTO_SEGUNDOS`.
/// 0 lo desactiva. Un valor no numérico usa el por defecto y lo deja registrado.
pub fn segundos_automaticos() -> u64 {
    parsear_segundos(std::env::var("BDP_ALBARANES_AUTO_SEGUNDOS").ok().as_deref())
}

/// Ventana del disparo automático en días, leída de `BDP_ALBARANES_AUTO_DIAS`.
pub fn dias_automaticos() -> i64 {
    std::env::var("BDP_ALBARANES_AUTO_DIAS")
        .ok()
        .and_then(|valor| valor.trim().parse::<i64>().ok())
        .unwrap_or(DIAS_MAX_RANGO)
        .clamp(1, DIAS_MAX_RANGO)
}

fn parsear_segundos(valor: Option<&str>) -> u64 {
    match valor.map(str::trim) {
        None | Some("") => SEGUNDOS_AUTO_POR_DEFECTO,
        Some(texto) => texto.parse::<u64>().unwrap_or_else(|_| {
            tracing::warn!("BDP_ALBARANES_AUTO_SEGUNDOS no es un número: se usa el por defecto");
            SEGUNDOS_AUTO_POR_DEFECTO
        }),
    }
}

/// Mapea un error del cliente BDP a un `AppError` que preserve la semántica HTTP.
pub fn map_bdp_error(err: &BdpWeblinkError) -> AppError {
    match err {
        BdpWeblinkError::NotConfigured | BdpWeblinkError::InvalidBaseUrl(_) => {
            AppError::Validation("BDP no está configurado correctamente".into())
        }
        BdpWeblinkError::Throttled(_) => {
            AppError::Validation("BDP ha limitado las peticiones; inténtalo más tarde".into())
        }
        _ => AppError::Internal(format!("Error ExportPurchaseNotes: {err}")),
    }
}

/// Indica si el albarán tiene clave natural completa (serie y número).
/// Los documentos sin ambos valores se descartan para evitar conflictos de clave
/// vacía en la restricción `UNIQUE (user_id, serie, numero)`.
pub(crate) fn albaran_tiene_clave(note: &BdpPurchaseNoteData) -> bool {
    let serie = note.serie_albaran.as_deref().unwrap_or("").trim();
    let numero = note.num_albaran.as_deref().unwrap_or("").trim();
    !serie.is_empty() && !numero.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ventana_automatica_termina_hoy_y_respeta_el_maximo() {
        let hoy = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        assert_eq!(
            ventana_automatica(hoy, 7),
            ("2026-10-12".to_string(), "2026-10-19".to_string())
        );
        // 90 días se acota a 31: el rango nunca supera el límite de BDP.
        assert_eq!(ventana_automatica(hoy, 90).0, "2026-09-18");
        assert_eq!(ventana_automatica(hoy, 0).0, "2026-10-18");
    }

    #[test]
    fn parsear_segundos_usa_por_defecto_y_acepta_cero() {
        assert_eq!(parsear_segundos(None), SEGUNDOS_AUTO_POR_DEFECTO);
        assert_eq!(parsear_segundos(Some("  ")), SEGUNDOS_AUTO_POR_DEFECTO);
        assert_eq!(parsear_segundos(Some("0")), 0);
        assert_eq!(parsear_segundos(Some("120")), 120);
        assert_eq!(parsear_segundos(Some("cinco")), SEGUNDOS_AUTO_POR_DEFECTO);
    }

    #[test]
    fn rango_automatico_envia_proveedores_completos() {
        let req = rango_automatico(3, "2026-10-12", "2026-10-19");
        assert_eq!(req.export_profile_code, 3);
        assert_eq!(req.initial_supplier, Some(1));
        assert_eq!(req.final_supplier, Some(999_999));
    }
}
