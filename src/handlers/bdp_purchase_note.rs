/* [247A-11] Handlers de albaranes de compra BDP (Fase 1 — solo lectura).
 * GET  /api/bdp/purchase-notes      — listar albaranes locales
 * POST /api/bdp/purchase-notes/sync — importar desde BDP (ExportPurchaseNotes)
 * La funcionalidad está protegida por el feature flag ff_bdp_purchase_notes_read. */

use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    BdpPurchaseNote, BdpPurchaseNoteListParams, BdpPurchaseNoteSyncRequest,
    BdpPurchaseNoteSyncResult,
};
use crate::repositories::BdpPurchaseNoteRepository;
use crate::services::bdp_weblink_catalog::{
    BdpExportPurchaseNotesRequest, BdpExportPurchaseNotesResponse,
};
use crate::services::{BdpWeblinkClient, ConfiguracionService};
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/bdp/purchase-notes", get(listar_purchase_notes))
        .route("/bdp/purchase-notes/sync", post(sincronizar_purchase_notes))
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
    let (fecha_desde, fecha_hasta) = validar_rango_fechas(&req)?;

    let client = BdpWeblinkClient::new(&config);
    let _session = client
        .login()
        .await
        .map_err(|e| AppError::Internal(format!("Error login BDP: {e}")))?;

    let bdp_request = BdpExportPurchaseNotesRequest {
        export_profile_code: req.export_profile_code,
        initial_date: fecha_desde.clone(),
        final_date: fecha_hasta.clone(),
        initial_supplier: req.proveedor_desde,
        final_supplier: req.proveedor_hasta,
        initial_serial: None,
        final_serial: None,
    };

    let bdp_response: serde_json::Value = client
        .export_purchase_notes(&bdp_request)
        .await
        .map_err(|e| AppError::Internal(format!("Error ExportPurchaseNotes: {e}")))?;

    let parsed: BdpExportPurchaseNotesResponse =
        serde_json::from_value(bdp_response).map_err(|e| {
            AppError::Internal(format!("Respuesta de BDP no tiene el formato esperado: {e}"))
        })?;

    let total_bdp = parsed.documents_lists.len();
    let mut procesados = 0;

    for note in parsed.documents_lists {
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

/// Valida que el rango de fechas esté presente y sea menor o igual a 31 días.
fn validar_rango_fechas(req: &BdpPurchaseNoteSyncRequest) -> Result<(Option<String>, Option<String>), AppError> {
    let desde = req.fecha_desde.clone();
    let hasta = req.fecha_hasta.clone();

    if desde.is_none() || hasta.is_none() {
        return Err(AppError::Validation(
            "Debes indicar fecha_desde y fecha_hasta".into(),
        ));
    }

    if let (Some(d), Some(h)) = (&desde, &hasta) {
        let d_parsed = chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d")
            .map_err(|_| AppError::Validation("fecha_desde inválida".into()))?;
        let h_parsed = chrono::NaiveDate::parse_from_str(h, "%Y-%m-%d")
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
    }

    Ok((desde, hasta))
}
