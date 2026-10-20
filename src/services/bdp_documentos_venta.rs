//! Lectura incremental de documentos de venta desde BDP (`ExportProfiles/Documents`).
//!
//! [L8/D28] La ventana sale del cursor de `bdp_sync_cursor` con un día de solape,
//! para no perder documentos cerrados con retraso. Los campos de documento no
//! están documentados: el parser acepta varios nombres y guarda el JSON original.
//! El cursor solo avanza cuando el lote se ha guardado entero.

use std::collections::BTreeMap;

use chrono::{Duration, NaiveDate, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::ConfiguracionRestaurante;
use crate::repositories::{BdpDocumentosVentaRepository, DocumentoVentaLocal};
use crate::services::bdp_purchase_sync::DIAS_MAX_RANGO;
use crate::services::bdp_weblink::BdpWeblinkError;
use crate::services::bdp_weblink_catalog::{objetos_de, BdpExportDocumentsRequest};
use crate::services::{
    BdpWeblinkClient, ConfiguracionService, ModoEfectivo, ServicioModoOperacion,
};

/// Días que se releen del último rango: BDP puede cerrar documentos con retraso.
const DIAS_SOLAPE: i64 = 1;
/// Ventana de la primera lectura, cuando todavía no hay cursor.
const DIAS_PRIMERA_LECTURA: i64 = 7;
/// Claves bajo las que BDP puede devolver la lista; gana la primera que exista.
const CLAVES_LISTA: [&str; 3] = ["DocumentsLists", "Documents", "DocumentsList"];
/// `idWeblink` es el único identificador que trae el documento real: sin él la
/// lectura entera se descarta documento a documento (comprobado contra el BDP).
const CLAVES_ID: [&str; 5] = ["Id", "DocumentId", "Document_Id", "Number", "idWeblink"];
const CLAVES_FECHA: [&str; 3] = ["Date", "DocumentDate", "Fecha"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResultadoDocumentos {
    /// Documentos guardados (nuevos o actualizados).
    pub procesados: usize,
    /// Documentos distintos recibidos de BDP tras deduplicar por id.
    pub total_bdp: usize,
}

/// Ventana `[desde, hasta]` de la lectura. Con cursor, desde `cursor - solape`;
/// sin cursor, los últimos días de la primera lectura. Nunca supera `DIAS_MAX_RANGO`.
#[must_use]
pub fn ventana(cursor: Option<NaiveDate>, hoy: NaiveDate) -> (NaiveDate, NaiveDate) {
    let desde = match cursor {
        Some(ultimo) => ultimo - Duration::days(DIAS_SOLAPE),
        None => hoy - Duration::days(DIAS_PRIMERA_LECTURA),
    };
    let minimo = hoy - Duration::days(DIAS_MAX_RANGO);
    // Un cursor futuro (desfase de reloj) no puede dejar la ventana invertida.
    (desde.clamp(minimo, hoy), hoy)
}

/// Documentos de la respuesta. La lista llega agrupada por serie
/// (`"DocumentsLists": { "TB": [ … ], "FA": [ … ] }`) o como array directo;
/// `objetos_de` acepta las dos. Sin ninguna de las claves, la respuesta entera
/// puede ser la lista.
fn documentos_de(respuesta: &Value) -> Vec<&Value> {
    match CLAVES_LISTA
        .iter()
        .find_map(|clave| respuesta.get(*clave))
    {
        Some(lista) => objetos_de(Some(lista)),
        None if respuesta.is_array() => objetos_de(Some(respuesta)),
        None => Vec::new(),
    }
}

/// Parsea la respuesta de forma tolerante: descarta entradas sin id y deduplica por id.
pub fn parsear_documentos(respuesta: &Value) -> Vec<DocumentoVentaLocal> {
    let items = documentos_de(respuesta);
    if items.is_empty() {
        return Vec::new();
    }
    let mut por_id: BTreeMap<String, DocumentoVentaLocal> = BTreeMap::new();
    for item in items {
        let Some(id) = texto_de(item, &CLAVES_ID) else {
            tracing::warn!("[D28] Documento de BDP sin identificador: descartado");
            continue;
        };
        let fecha = texto_de(item, &CLAVES_FECHA).and_then(|texto| fecha_de(&texto));
        por_id.insert(
            id.clone(),
            DocumentoVentaLocal {
                bdp_documento_id: id,
                fecha,
                datos: item.clone(),
            },
        );
    }
    por_id.into_values().collect()
}

/// Primer valor no vacío bajo cualquiera de las claves, como texto (acepta números).
fn texto_de(item: &Value, claves: &[&str]) -> Option<String> {
    claves.iter().find_map(|clave| match item.get(*clave)? {
        Value::String(texto) if !texto.trim().is_empty() => Some(texto.trim().to_string()),
        Value::Number(numero) => Some(numero.to_string()),
        _ => None,
    })
}

/// Acepta `AAAA-MM-DD` (con o sin hora) y `DD/MM/AAAA`; cualquier otro formato da `None`.
fn fecha_de(texto: &str) -> Option<NaiveDate> {
    let inicio: String = texto.chars().take(10).collect();
    NaiveDate::parse_from_str(&inicio, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(&inicio, "%d/%m/%Y"))
        .ok()
}

/// Lectura incremental de un perfil: pide la ventana, guarda el lote y avanza el cursor.
/// Ante un fallo registra el error y deja el cursor donde estaba.
pub async fn sincronizar(
    pool: &PgPool,
    user_id: Uuid,
    config: &ConfiguracionRestaurante,
    perfil: i32,
    solo_ultimos_movimientos: bool,
) -> Result<ResultadoDocumentos, AppError> {
    if perfil <= 0 {
        return Err(AppError::Validation(
            "El perfil de documentos debe ser mayor que 0".into(),
        ));
    }
    let cursor = BdpDocumentosVentaRepository::leer_cursor(pool, user_id, perfil)
        .await
        .map_err(|e| AppError::Internal(format!("Error leyendo cursor de documentos: {e}")))?;
    let hoy = Utc::now().date_naive();
    let (desde, hasta) = ventana(cursor, hoy);
    if cursor.is_some_and(|ultimo| ultimo - Duration::days(DIAS_SOLAPE) < desde) {
        tracing::warn!(
            "[D28] El cursor es más antiguo que el máximo de BDP: hay un hueco de lectura"
        );
    }

    let req = BdpExportDocumentsRequest {
        export_profile_code: perfil,
        initial_date: desde.format("%Y-%m-%d").to_string(),
        final_date: hasta.format("%Y-%m-%d").to_string(),
        only_last_movements: solo_ultimos_movimientos,
    };
    let client = BdpWeblinkClient::new(config);
    let respuesta = match client.export_documents(&req).await {
        Ok(respuesta) => respuesta,
        Err(error) => {
            let mensaje = error_documentos(&error).to_string();
            // Si el propio registro falla, se prefiere devolver el error original.
            let _ = BdpDocumentosVentaRepository::registrar_error(pool, user_id, perfil, &mensaje)
                .await;
            return Err(error_documentos(&error));
        }
    };

    let documentos = parsear_documentos(&respuesta);
    let total_bdp = documentos.len();
    let procesados =
        BdpDocumentosVentaRepository::guardar_lote(pool, user_id, perfil, &documentos, hasta)
            .await
            .map_err(|e| AppError::Internal(format!("Error guardando documentos de venta: {e}")))?;
    Ok(ResultadoDocumentos {
        procesados,
        total_bdp,
    })
}

/// Mapea un error del cliente BDP a un `AppError` con la semántica HTTP de cada caso.
fn error_documentos(err: &BdpWeblinkError) -> AppError {
    match err {
        BdpWeblinkError::NotConfigured | BdpWeblinkError::InvalidBaseUrl(_) => {
            AppError::Validation("BDP no está configurado correctamente".into())
        }
        BdpWeblinkError::Throttled(_) => {
            AppError::Validation("BDP ha limitado las peticiones; inténtalo más tarde".into())
        }
        _ => AppError::Internal(format!("Error ExportDocuments: {err}")),
    }
}

/// Intervalo del disparo automático en segundos, de `BDP_DOCUMENTOS_AUTO_SEGUNDOS`.
/// Por defecto 0: apagado hasta que se configure, porque llama a BDP en cada ciclo.
#[must_use]
pub fn segundos_automaticos() -> u64 {
    std::env::var("BDP_DOCUMENTOS_AUTO_SEGUNDOS")
        .ok()
        .and_then(|valor| valor.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

/// Perfil de exportación para el disparo automático, de `BDP_DOCUMENTOS_PERFIL`.
/// Sin perfil válido el disparo no hace nada.
#[must_use]
pub fn perfil_automatico() -> Option<i32> {
    std::env::var("BDP_DOCUMENTOS_PERFIL")
        .ok()
        .and_then(|valor| valor.trim().parse::<i32>().ok())
        .filter(|perfil| *perfil > 0)
}

/// Lectura incremental de todas las cuentas en modo BDP con sincronización activa.
/// Un fallo de una cuenta se registra y no corta a las demás.
pub async fn sincronizar_automatico(
    pool: &PgPool,
    modo: &ServicioModoOperacion,
    perfil: i32,
) -> usize {
    let usuarios: Vec<Uuid> =
        match sqlx::query_scalar("SELECT user_id FROM configuracion_restaurante")
            .fetch_all(pool)
            .await
        {
            Ok(usuarios) => usuarios,
            Err(error) => {
                tracing::warn!("[D28] No se pudieron listar cuentas para documentos: {error}");
                return 0;
            }
        };

    let mut total = 0;
    for user_id in usuarios {
        if !matches!(
            modo.modo_efectivo(pool, user_id).await,
            Ok(ModoEfectivo::Bdp)
        ) {
            continue;
        }
        let Ok(config) = ConfiguracionService::obtener(pool, user_id).await else {
            continue;
        };
        if !config.bdp_sync_enabled || config.bdp_base_url.is_empty() || config.bdp_login.is_empty()
        {
            continue;
        }
        match sincronizar(pool, user_id, &config, perfil, true).await {
            Ok(resultado) => total += resultado.procesados,
            Err(error) => {
                tracing::warn!("[D28] Lectura automática de documentos fallida: {error:?}");
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn dia(texto: &str) -> NaiveDate {
        NaiveDate::parse_from_str(texto, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn ventana_sin_cursor_usa_la_primera_lectura() {
        let hoy = dia("2026-10-20");
        assert_eq!(ventana(None, hoy), (dia("2026-10-13"), hoy));
    }

    /* [D28-auto-R] Con cursor, la ventana empieza un día antes: no hay huecos ni salto. */
    #[test]
    fn ventana_con_cursor_solapa_un_dia() {
        let hoy = dia("2026-10-20");
        assert_eq!(
            ventana(Some(dia("2026-10-18")), hoy),
            (dia("2026-10-17"), hoy)
        );
    }

    #[test]
    fn ventana_nunca_supera_el_maximo_de_bdp() {
        let hoy = dia("2026-10-20");
        let (desde, _) = ventana(Some(dia("2026-08-01")), hoy);
        assert_eq!((hoy - desde).num_days(), DIAS_MAX_RANGO);
    }

    #[test]
    fn ventana_con_cursor_futuro_no_se_invierte() {
        let hoy = dia("2026-10-20");
        let (desde, hasta) = ventana(Some(dia("2026-10-25")), hoy);
        assert!(desde <= hasta);
    }

    #[test]
    fn parsear_acepta_alias_numeros_y_fechas() {
        let respuesta = json!({"Documents": [
            {"DocumentId": 77, "DocumentDate": "2026-10-19T10:00:00"},
            {"Number": "A-2", "Fecha": "19/10/2026"},
        ]});
        let docs = parsear_documentos(&respuesta);
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].bdp_documento_id, "77");
        assert_eq!(docs[0].fecha, Some(dia("2026-10-19")));
        assert_eq!(docs[1].bdp_documento_id, "A-2");
        assert_eq!(docs[1].fecha, Some(dia("2026-10-19")));
    }

    /* [D28] La forma REAL que devuelve el BDP local, copiada de una respuesta
     * del 2026-10-20: la lista viene agrupada por serie y el único identificador
     * es `idWeblink` —no hay `Id`, `DocumentId`, `Document_Id` ni `Number`—.
     * Con la forma que el contrato suponía (array y `Id`) el parser devolvía
     * cero documentos y lo hacía en silencio: `serde` no se quejaba porque la
     * respuesta se lee como `Value`. */
    #[test]
    fn parsear_lee_la_forma_real_agrupada_por_serie() {
        let respuesta = json!({"DocumentsLists": {"TB": [
            {"Serie": "00001TB", "Numero": 11.0, "Fecha": "2026-10-18T00:00:00",
             "Total_Documento": 11.3, "idWeblink": 18, "Items": [{"PLU": 1006.0}], "Pagos": [{}]},
            {"Serie": "00001TB", "Numero": 30.0, "Fecha": "2026-10-18T00:00:00",
             "Total_Documento": 1.0, "idWeblink": 53, "Items": [{}], "Pagos": [{}]},
        ]}});
        let docs = parsear_documentos(&respuesta);
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].bdp_documento_id, "18");
        assert_eq!(docs[1].bdp_documento_id, "53");
        assert_eq!(docs[0].fecha, Some(dia("2026-10-18")));
        assert_eq!(docs[0].datos["Total_Documento"], 11.3);
        assert_eq!(docs[1].datos["Items"].as_array().unwrap().len(), 1);
    }

    /* Dos series en la misma respuesta: se leen en orden alfabético de serie y
     * el mismo id en dos series cuenta una vez. */
    #[test]
    fn parsear_ordena_las_series_y_deduplica_entre_ellas() {
        let respuesta = json!({"DocumentsLists": {
            "TB": [{"idWeblink": 2}],
            "FA": [{"idWeblink": 1}, {"idWeblink": 2}],
        }});
        let ids: Vec<String> = parsear_documentos(&respuesta)
            .into_iter()
            .map(|d| d.bdp_documento_id)
            .collect();
        assert_eq!(ids, vec!["1", "2"]);
    }

    /* [D28-auto-R] Una ventana solapada que repite documentos no los duplica. */
    #[test]
    fn parsear_deduplica_por_id_y_descarta_sin_id() {
        let respuesta = json!({"DocumentsLists": [
            {"Id": "1", "Total": 10},
            {"Id": "1", "Total": 12},
            {"Total": 5},
            {"Id": "  "},
        ]});
        let docs = parsear_documentos(&respuesta);
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].datos["Total"], 12);
        assert_eq!(docs[0].fecha, None);
    }

    #[test]
    fn parsear_respuesta_sin_lista_devuelve_vacio() {
        assert!(parsear_documentos(&json!({"ErrorMessage": ""})).is_empty());
    }

    #[test]
    fn fecha_de_descarta_formatos_desconocidos() {
        assert_eq!(fecha_de("no es fecha"), None);
        assert_eq!(fecha_de("2026-13-40"), None);
    }
}
