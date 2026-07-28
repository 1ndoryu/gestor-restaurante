/* [247A-11] Handlers de albaranes de compra BDP (Fase 1 — solo lectura).
 * GET  /api/bdp/purchase-notes      — listar albaranes locales
 * POST /api/bdp/purchase-notes/sync — importar desde BDP (ExportPurchaseNotes)
 * La funcionalidad está protegida por el feature flag ff_bdp_purchase_notes_read. */

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    BdpPurchaseNote, BdpPurchaseNoteDraftRequest, BdpPurchaseNoteListParams,
    BdpPurchaseNoteReconcileRequest, BdpPurchaseNoteReconcileResult, BdpPurchaseNoteSyncRequest,
    BdpPurchaseNoteSyncResult,
};
use crate::repositories::gasto::NuevoGasto;
use crate::repositories::{BdpPurchaseNoteRepository, GastoRepository};
use crate::services::bdp_weblink::BdpWeblinkError;
use crate::services::bdp_weblink_catalog::{
    BdpExportPurchaseNotesRequest, BdpExportPurchaseNotesResponse,
};
use crate::services::{BdpWeblinkClient, ConfiguracionService};
use crate::AppState;
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/bdp/purchase-notes", get(listar_purchase_notes))
        .route("/bdp/purchase-notes/sync", post(sincronizar_purchase_notes))
        .route(
            "/bdp/purchase-notes/:id/draft",
            post(marcar_borrador_purchase_note),
        )
        .route(
            "/bdp/purchase-notes/:id/reconcile",
            post(conciliar_purchase_note),
        )
}

/// Listar albaranes de compra importados desde BDP.
#[utoipa::path(
    get,
    path = "/api/bdp/purchase-notes",
    tag = "BDP Compras",
    params(
        ("proveedor" = Option<String>, Query, description = "Filtro por código o nombre de proveedor"),
        ("fecha_desde" = Option<String>, Query, description = "Fecha inicial (YYYY-MM-DD)"),
        ("fecha_hasta" = Option<String>, Query, description = "Fecha final (YYYY-MM-DD)")
    ),
    responses(
        (status = 200, description = "Lista de albaranes", body = [BdpPurchaseNote]),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 403, description = "Feature flag desactivado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_purchase_notes(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<BdpPurchaseNoteListParams>,
) -> Result<Json<Vec<BdpPurchaseNote>>, AppError> {
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
    if !config.ff_bdp_purchase_notes_read {
        return Err(AppError::Validation(
            "La lectura de albaranes de compra BDP no está activada".into(),
        ));
    }
    let notes = BdpPurchaseNoteRepository::listar(&state.pool, auth.user_id, &params).await?;
    Ok(Json(notes))
}

/// Sincroniza albaranes de compra desde BDP (`ExportPurchaseNotes`).
#[utoipa::path(
    post,
    path = "/api/bdp/purchase-notes/sync",
    tag = "BDP Compras",
    request_body = BdpPurchaseNoteSyncRequest,
    responses(
        (status = 200, description = "Albaranes sincronizados", body = BdpPurchaseNoteSyncResult),
        (status = 400, description = "BDP no configurado o rango inválido", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 403, description = "Feature flag desactivado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn sincronizar_purchase_notes(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<BdpPurchaseNoteSyncRequest>,
) -> Result<Json<BdpPurchaseNoteSyncResult>, AppError> {
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
    if !config.ff_bdp_purchase_notes_read {
        return Err(AppError::Validation(
            "La sincronización de albaranes BDP no está activada".into(),
        ));
    }
    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation(
            "BDP no está configurado. Configura URL y credenciales primero.".into(),
        ));
    }

    /* Evitar importes masivos descontrolados: exigir rango de fechas de <= 31 días. */
    validar_rango_fechas(&req)?;

    let export_profile_code = resolver_perfil_exportacion(
        req.export_profile_code,
        config.bdp_purchase_notes_profile_id,
    )?;

    let client = BdpWeblinkClient::new(&config);

    let bdp_request = BdpExportPurchaseNotesRequest {
        export_profile_code,
        initial_date: req.fecha_desde.clone(),
        final_date: req.fecha_hasta.clone(),
        /* [287A-4] BDP real rechaza proveedores omitidos con 403900.
         * La UI mantiene filtros opcionales, pero WebLink recibe el rango
         * completo cuando el usuario no limita proveedores. */
        initial_supplier: Some(req.proveedor_desde.unwrap_or(1)),
        final_supplier: Some(req.proveedor_hasta.unwrap_or(999_999)),
        initial_serial: None,
        final_serial: None,
    };

    let bdp_response: serde_json::Value = client
        .export_purchase_notes(&bdp_request)
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
        match BdpPurchaseNoteRepository::upsert_from_bdp(&state.pool, auth.user_id, &note).await {
            Ok(true) => procesados += 1,
            Ok(false) => {}
            Err(e) => {
                tracing::warn!("[247A-11] Error guardando albarán: {e}");
            }
        }
    }

    Ok(Json(BdpPurchaseNoteSyncResult {
        procesados,
        total_bdp,
    }))
}

/// Marca un albarán de compra como borrador (Fase 2).
/// Protegido por el feature flag `ff_bdp_purchase_notes_draft`.
#[utoipa::path(
    post,
    path = "/api/bdp/purchase-notes/{id}/draft",
    tag = "BDP Compras",
    request_body = BdpPurchaseNoteDraftRequest,
    params(("id" = Uuid, Path, description = "ID del albarán")),
    responses(
        (status = 200, description = "Albarán marcado como borrador", body = BdpPurchaseNote),
        (status = 400, description = "Transición no permitida", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 403, description = "Feature flag desactivado", body = ErrorResponse),
        (status = 404, description = "Albarán no encontrado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn marcar_borrador_purchase_note(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(_req): Json<BdpPurchaseNoteDraftRequest>,
) -> Result<Json<BdpPurchaseNote>, AppError> {
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
    if !config.ff_bdp_purchase_notes_draft {
        return Err(AppError::Validation(
            "La creación de borradores de compra BDP no está activada".into(),
        ));
    }

    if !BdpPurchaseNoteRepository::marcar_borrador(&state.pool, id, auth.user_id).await? {
        return Err(AppError::Validation(
            "No se puede marcar como borrador un albarán que no esté pendiente".into(),
        ));
    }

    let note = BdpPurchaseNoteRepository::find_by_id(&state.pool, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Albarán no encontrado".into()))?;

    tracing::info!(
        "[247A-12] Albarán {} marcado como borrador por usuario {}",
        note.id,
        auth.user_id
    );
    Ok(Json(note))
}

/// Concilia un albarán de compra con un gasto existente o nuevo (Fase 3).
/// Protegido por el feature flag `ff_bdp_purchase_notes_receive`.
#[utoipa::path(
    post,
    path = "/api/bdp/purchase-notes/{id}/reconcile",
    tag = "BDP Compras",
    request_body = BdpPurchaseNoteReconcileRequest,
    params(("id" = Uuid, Path, description = "ID del albarán")),
    responses(
        (status = 200, description = "Albarán conciliado", body = BdpPurchaseNoteReconcileResult),
        (status = 400, description = "Transición no permitida", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 403, description = "Feature flag desactivado", body = ErrorResponse),
        (status = 404, description = "Albarán o gasto no encontrado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn conciliar_purchase_note(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<BdpPurchaseNoteReconcileRequest>,
) -> Result<Json<BdpPurchaseNoteReconcileResult>, AppError> {
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
    if !config.ff_bdp_purchase_notes_receive {
        return Err(AppError::Validation(
            "La conciliación de compras BDP no está activada".into(),
        ));
    }

    let note = BdpPurchaseNoteRepository::find_by_id(&state.pool, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Albarán no encontrado".into()))?;

    if !matches!(note.estado, crate::models::BdpPurchaseNoteEstado::Borrador) {
        return Err(AppError::Validation(
            "Solo se pueden conciliar albaranes en estado borrador".into(),
        ));
    }

    let mut tx = state.pool.begin().await?;

    let gasto_id = if let Some(gasto_existente_id) = req.gasto_existente_id {
        let gasto = GastoRepository::find_by_id(&mut *tx, gasto_existente_id, auth.user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Gasto no encontrado".into()))?;
        gasto.id
    } else {
        let proveedor = note.nombre_proveedor.as_deref().unwrap_or("Proveedor BDP");
        let numero_documento = format!("{}-{}", note.serie, note.numero);
        let total = note.total.unwrap_or_default();
        let fecha = note
            .fecha
            .unwrap_or_else(|| chrono::Utc::now().date_naive());
        let cero = rust_decimal::Decimal::ZERO;
        /* [247A-12] El albarán de BDP solo nos da el total. Como no tenemos
         * desglose de IVA, registramos el importe completo en importe_base y 0
         * en importe_iva. Si en el futuro BDP exporta el desglose fiscal, se
         * podrá fraccionar aquí. */
        let nuevo = NuevoGasto {
            user_id: auth.user_id,
            fecha,
            proveedor,
            categoria_id: req.categoria_id,
            tipo_documento: "albaran",
            metodo_pago: "",
            numero_documento: &numero_documento,
            recurrente: false,
            importe_base: total,
            importe_iva: cero,
        };
        let gasto = GastoRepository::create(&mut *tx, &nuevo).await?;
        gasto.id
    };

    if !BdpPurchaseNoteRepository::vincular_gasto(&mut *tx, id, auth.user_id, gasto_id).await? {
        return Err(AppError::Validation(
            "El albarán ya está conciliado o no está en estado borrador".into(),
        ));
    }

    tx.commit().await?;

    tracing::info!(
        "[247A-12] Albarán {} conciliado con gasto {} por usuario {}",
        id,
        gasto_id,
        auth.user_id
    );

    Ok(Json(BdpPurchaseNoteReconcileResult {
        albaran_id: id,
        gasto_id,
        accion: if req.gasto_existente_id.is_some() {
            "vinculado".to_string()
        } else {
            "creado".to_string()
        },
    }))
}

/// Mapea un error del cliente BDP a un `AppError` que preserve la semántica HTTP.
fn map_bdp_error(err: &BdpWeblinkError) -> AppError {
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

fn resolver_perfil_exportacion(
    requested: Option<i32>,
    configured: Option<i32>,
) -> Result<i32, AppError> {
    requested
        .or(configured)
        .filter(|code| *code > 0)
        .ok_or_else(|| {
            AppError::Validation(
                "Configura el código de plantilla ExportPurchaseNotes para continuar".into(),
            )
        })
}

/// Indica si el albarán tiene clave natural completa (serie y número).
/// Los documentos sin ambos valores se descartan para evitar conflictos de clave
/// vacía en la restricción `UNIQUE (user_id, serie, numero)`.
fn albaran_tiene_clave(note: &crate::services::bdp_weblink_catalog::BdpPurchaseNoteData) -> bool {
    let serie = note.serie_albaran.as_deref().unwrap_or("").trim();
    let numero = note.num_albaran.as_deref().unwrap_or("").trim();
    !serie.is_empty() && !numero.is_empty()
}

/// Valida que el rango de fechas esté presente y sea menor o igual a 31 días.
fn validar_rango_fechas(req: &BdpPurchaseNoteSyncRequest) -> Result<(), AppError> {
    let Some(desde) = &req.fecha_desde else {
        return Err(AppError::Validation(
            "Debes indicar fecha_desde y fecha_hasta".into(),
        ));
    };
    let Some(hasta) = &req.fecha_hasta else {
        return Err(AppError::Validation(
            "Debes indicar fecha_desde y fecha_hasta".into(),
        ));
    };

    let d_parsed = chrono::NaiveDate::parse_from_str(desde, "%Y-%m-%d")
        .map_err(|_| AppError::Validation("fecha_desde inválida".into()))?;
    let h_parsed = chrono::NaiveDate::parse_from_str(hasta, "%Y-%m-%d")
        .map_err(|_| AppError::Validation("fecha_hasta inválida".into()))?;
    let diff = (h_parsed - d_parsed).num_days();
    if diff < 0 {
        return Err(AppError::Validation(
            "fecha_hasta debe ser mayor o igual que fecha_desde".into(),
        ));
    }
    if diff > 31 {
        return Err(AppError::Validation(
            "El rango de fechas no puede superar los 31 días".into(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::bdp_weblink_catalog::BdpPurchaseNoteData;

    fn req_con_fechas(fecha_desde: &str, fecha_hasta: &str) -> BdpPurchaseNoteSyncRequest {
        BdpPurchaseNoteSyncRequest {
            export_profile_code: Some(1),
            fecha_desde: Some(fecha_desde.to_string()),
            fecha_hasta: Some(fecha_hasta.to_string()),
            proveedor_desde: None,
            proveedor_hasta: None,
        }
    }

    #[test]
    fn perfil_de_peticion_tiene_prioridad_sobre_configuracion() {
        assert_eq!(resolver_perfil_exportacion(Some(7), Some(3)).unwrap(), 7);
    }

    #[test]
    fn usa_perfil_persistido_si_la_peticion_lo_omite() {
        assert_eq!(resolver_perfil_exportacion(None, Some(3)).unwrap(), 3);
    }

    #[test]
    fn rechaza_compras_sin_plantilla_configurada() {
        assert!(resolver_perfil_exportacion(None, None).is_err());
        assert!(resolver_perfil_exportacion(Some(0), None).is_err());
    }

    #[test]
    fn validar_rango_fechas_rejects_missing_dates() {
        let req = BdpPurchaseNoteSyncRequest {
            export_profile_code: Some(1),
            fecha_desde: None,
            fecha_hasta: Some("2024-07-31".to_string()),
            proveedor_desde: None,
            proveedor_hasta: None,
        };
        assert!(validar_rango_fechas(&req).is_err());
    }

    #[test]
    fn validar_rango_fechas_accepts_31_day_range() {
        let req = req_con_fechas("2024-07-01", "2024-08-01");
        assert!(validar_rango_fechas(&req).is_ok());
    }

    #[test]
    fn validar_rango_fechas_rejects_more_than_31_days() {
        let req = req_con_fechas("2024-07-01", "2024-08-02");
        assert!(validar_rango_fechas(&req).is_err());
    }

    #[test]
    fn validar_rango_fechas_rejects_invalid_order() {
        let req = req_con_fechas("2024-08-01", "2024-07-01");
        assert!(validar_rango_fechas(&req).is_err());
    }

    #[test]
    fn albaran_sin_serie_ni_numero_se_descarta() {
        let note = BdpPurchaseNoteData {
            serie_albaran: None,
            num_albaran: None,
            fecha_albaran: None,
            cod_proveedor: None,
            nom_proveedor: None,
            total_albaran: None,
            extra: serde_json::json!({ "DocumentNumber": "X" }),
        };
        assert!(!super::albaran_tiene_clave(&note));
    }

    #[test]
    fn albaran_con_serie_y_numero_se_mantiene() {
        let note = BdpPurchaseNoteData {
            serie_albaran: Some("A".to_string()),
            num_albaran: Some("42".to_string()),
            fecha_albaran: None,
            cod_proveedor: None,
            nom_proveedor: None,
            total_albaran: None,
            extra: serde_json::json!({}),
        };
        assert!(super::albaran_tiene_clave(&note));
    }

    #[test]
    fn albaran_con_solo_serie_o_numero_se_descarta() {
        let note = BdpPurchaseNoteData {
            serie_albaran: Some("A".to_string()),
            num_albaran: None,
            fecha_albaran: None,
            cod_proveedor: None,
            nom_proveedor: None,
            total_albaran: None,
            extra: serde_json::json!({}),
        };
        assert!(!super::albaran_tiene_clave(&note));
    }
}
