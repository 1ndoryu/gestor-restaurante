/* [247A-11] Handlers de albaranes de compra BDP (Fase 1 — solo lectura).
 * GET  /api/bdp/purchase-notes      — listar albaranes locales
 * POST /api/bdp/purchase-notes/sync — importar desde BDP (ExportPurchaseNotes)
 * La funcionalidad está protegida por el feature flag ff_bdp_purchase_notes_read. */

use axum::extract::{Path, Query, State};
use axum::routing::{get, post, put};
use axum::{Json, Router};

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    ActualizarBdpPurchaseNoteRequest, BdpPurchaseNote, BdpPurchaseNoteDraftRequest,
    BdpPurchaseNoteListParams, BdpPurchaseNoteSyncRequest, BdpPurchaseNoteSyncResult,
    CrearBdpPurchaseNoteRequest,
};
use crate::repositories::{AuditoriaCicloLocal, BdpPurchaseNoteRepository};
use crate::services::bdp_purchase_sync::importar;
use crate::services::bdp_weblink_catalog::BdpExportPurchaseNotesRequest;
use crate::services::{
    verificar_permiso, AccionPermiso, ConfiguracionService, ModoEfectivo,
};
use crate::AppState;
use uuid::Uuid;

use super::rate_limit::{limitador_escrituras, limitar};

mod conciliacion;
pub use conciliacion::{__path_conciliar_purchase_note, conciliar_purchase_note};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/bdp/purchase-notes",
            get(listar_purchase_notes).post(crear_purchase_note_local),
        )
        .route("/bdp/purchase-notes/sync", post(sincronizar_purchase_notes))
        .route(
            "/bdp/purchase-notes/:id",
            put(actualizar_purchase_note_local).delete(eliminar_purchase_note_local),
        )
        .route(
            "/bdp/purchase-notes/:id/draft",
            post(marcar_borrador_purchase_note),
        )
        .route(
            "/bdp/purchase-notes/:id/reconcile",
            post(conciliar_purchase_note),
        )
        /* Tope por IP de las escrituras de este módulo (ver rate_limit.rs). */
        .layer(axum::middleware::from_fn_with_state(
            limitador_escrituras(),
            limitar,
        ))
}

/// Listar albaranes de compra importados desde BDP.
#[utoipa::path(
    get,
    path = "/api/bdp/purchase-notes",
    tag = "BDP Compras",
    params(
        ("proveedor" = Option<String>, Query, description = "Filtro por código o nombre de proveedor"),
        ("fecha_desde" = Option<chrono::NaiveDate>, Query, description = "Fecha inicial (YYYY-MM-DD)"),
        ("fecha_hasta" = Option<chrono::NaiveDate>, Query, description = "Fecha final (YYYY-MM-DD)")
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
    /* Listar es lectura local (incluye albaranes `origen='local'` que nunca
     * tocan BDP): no se gatea por flag. Solo `sync` importa desde BDP y
     * exige `ff_bdp_purchase_notes_read`. */
    let notes = BdpPurchaseNoteRepository::listar(&state.pool, auth.user_id, &params).await?;
    Ok(Json(notes))
}

/// Crear un albarán de compra local (F5, M18). Funciona sin BDP y sin gate de
/// flags: es CRUD local sobre `bdp_purchase_notes` con `origen='local'`.
#[utoipa::path(
    post,
    path = "/api/bdp/purchase-notes",
    tag = "BDP Compras",
    request_body = CrearBdpPurchaseNoteRequest,
    responses(
        (status = 200, description = "Albarán local creado", body = BdpPurchaseNote),
        (status = 400, description = "Validación fallida", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn crear_purchase_note_local(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CrearBdpPurchaseNoteRequest>,
) -> Result<Json<BdpPurchaseNote>, AppError> {
    /* [128A-1/F8] Permiso por acción: gestión de albaranes (D8/M17). */
    verificar_permiso(&state.pool, AccionPermiso::AlbaranesGestion, &auth).await?;
    if req.nombre_proveedor.is_none() && req.codigo_proveedor.is_none() {
        return Err(AppError::Validation(
            "Debes indicar el proveedor (nombre o código)".into(),
        ));
    }
    if req.total.is_none() && req.lineas.as_ref().is_none_or(Vec::is_empty) {
        return Err(AppError::Validation(
            "Debes indicar un total o al menos una línea".into(),
        ));
    }
    /* [128A-1/F5][F5-5] Fecha con formato estricto YYYY-MM-DD: una fecha
     * malformada no se ignora silenciosamente en el alta. */
    validar_fecha_local(req.fecha.as_deref())?;

    let note = BdpPurchaseNoteRepository::crear_local(&state.pool, auth.user_id, &req)
        .await
        .map_err(map_repo_error)?;
    tracing::info!(
        "[128A-1/F5] Albarán local {} ({}-{}) creado por usuario {}",
        note.id,
        note.serie,
        note.numero,
        auth.user_id
    );
    Ok(Json(note))
}

/// Actualizar un albarán de compra local (F5). Solo `origen='local'`; los
/// albaranes importados de BDP no se editan localmente.
#[utoipa::path(
    put,
    path = "/api/bdp/purchase-notes/{id}",
    tag = "BDP Compras",
    request_body = ActualizarBdpPurchaseNoteRequest,
    params(("id" = Uuid, Path, description = "ID del albarán")),
    responses(
        (status = 200, description = "Albarán actualizado", body = BdpPurchaseNote),
        (status = 400, description = "El albarán no es local", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 404, description = "Albarán no encontrado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn actualizar_purchase_note_local(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ActualizarBdpPurchaseNoteRequest>,
) -> Result<Json<BdpPurchaseNote>, AppError> {
    /* [128A-1/F8] Permiso por acción: gestión de albaranes (D8/M17). */
    verificar_permiso(&state.pool, AccionPermiso::AlbaranesGestion, &auth).await?;
    let note = BdpPurchaseNoteRepository::find_by_id(&state.pool, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Albarán no encontrado".into()))?;
    if note.origen != "local" {
        return Err(AppError::Validation(
            "Solo se pueden editar albaranes de origen local".into(),
        ));
    }
    /* [128A-1/F5][F5-5] Misma validación de fecha que en el alta. */
    validar_fecha_local(req.fecha.as_deref())?;

    let ok = BdpPurchaseNoteRepository::actualizar_local(&state.pool, id, auth.user_id, &req)
        .await
        .map_err(map_repo_error)?;
    if !ok {
        return Err(AppError::NotFound("Albarán local no encontrado".into()));
    }

    let updated = BdpPurchaseNoteRepository::find_by_id(&state.pool, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Albarán no encontrado".into()))?;
    tracing::info!(
        "[128A-1/F5] Albarán local {} actualizado por usuario {}",
        id,
        auth.user_id
    );
    Ok(Json(updated))
}

/// Eliminar un albarán de compra local (F5). Solo `pendiente`/`borrador`; los
/// conciliados no se borran (D5) y los importados de BDP no se tocan.
#[utoipa::path(
    delete,
    path = "/api/bdp/purchase-notes/{id}",
    tag = "BDP Compras",
    params(("id" = Uuid, Path, description = "ID del albarán")),
    responses(
        (status = 200, description = "Albarán eliminado", body = serde_json::Value),
        (status = 400, description = "No se puede eliminar (no local o conciliado)", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 404, description = "Albarán no encontrado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn eliminar_purchase_note_local(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    /* [128A-1/F8] Permiso por acción: gestión de albaranes (D8/M17). */
    verificar_permiso(&state.pool, AccionPermiso::AlbaranesGestion, &auth).await?;
    let ok = BdpPurchaseNoteRepository::eliminar_local(&state.pool, id, auth.user_id).await?;
    if !ok {
        /* Distinguir el motivo para dar una respuesta útil. */
        let note = BdpPurchaseNoteRepository::find_by_id(&state.pool, id, auth.user_id).await?;
        return match note {
            Some(n) if n.origen != "local" => Err(AppError::Validation(
                "Solo se pueden eliminar albaranes de origen local".into(),
            )),
            Some(n) if matches!(n.estado, crate::models::BdpPurchaseNoteEstado::Conciliado) => Err(
                AppError::Validation("Un albarán conciliado no se puede eliminar".into()),
            ),
            Some(_) => Err(AppError::Validation(
                "No se pudo eliminar el albarán (estado no permitido)".into(),
            )),
            None => Err(AppError::NotFound("Albarán no encontrado".into())),
        };
    }
    tracing::info!(
        "[128A-1/F5] Albarán local {} eliminado por usuario {}",
        id,
        auth.user_id
    );
    Ok(Json(serde_json::json!({ "mensaje": "Albarán eliminado" })))
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
    /* [128A-1/F5][M12] En modo efectivo `standalone` la sincronización con BDP
     * está desactivada por diseño (cero llamadas a BDP). */
    let modo = state
        .modo_operacion
        .modo_efectivo(&state.pool, auth.user_id)
        .await?;
    if modo == ModoEfectivo::Standalone {
        return Err(AppError::Validation(
            "Modo independiente: la sincronización con BDP está desactivada".into(),
        ));
    }
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

    let resultado = importar(&state.pool, auth.user_id, &config, &bdp_request).await?;
    Ok(Json(resultado))
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
    /* [128A-1/F8] Permiso por acción: gestión de albaranes (D8/M17). */
    verificar_permiso(&state.pool, AccionPermiso::AlbaranesGestion, &auth).await?;
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
    /* [128A-1/F5][M12] En `standalone` el ciclo de vida local no consulta flags. */
    let modo = state
        .modo_operacion
        .modo_efectivo(&state.pool, auth.user_id)
        .await?;
    if modo == ModoEfectivo::Bdp && !config.ff_bdp_purchase_notes_draft {
        return Err(AppError::Validation(
            "La creación de borradores de compra BDP no está activada".into(),
        ));
    }

    /* [169A-1] Transición + auditoría en la misma transacción: sin rastro
     * no hay cambio de estado (fail-closed). */
    let mut tx = state.pool.begin().await?;
    if !BdpPurchaseNoteRepository::marcar_borrador(&mut *tx, id, auth.user_id).await? {
        return Err(AppError::Validation(
            "No se puede marcar como borrador un albarán que no esté pendiente".into(),
        ));
    }

    let note = BdpPurchaseNoteRepository::find_by_id(&mut *tx, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Albarán no encontrado".into()))?;
    BdpPurchaseNoteRepository::auditar_ciclo_local(
        &mut *tx,
        auth.user_id,
        AuditoriaCicloLocal {
            operacion: "albaran_local_borrador",
            target_entity_type: "albaran",
            target_entity_id: note.id,
            datos: serde_json::json!({ "serie": note.serie, "numero": note.numero, "estado": "borrador" }),
            motivo: format!(
                "Albarán local {}-{} pasado a borrador — operación interna, no requiere autorización BDP",
                note.serie, note.numero
            ),
            idempotency_key: &format!("albaran-local-borrador-{id}"),
        },
    )
    .await?;
    tx.commit().await?;

    tracing::info!(
        "[247A-12] Albarán {} marcado como borrador por usuario {}",
        note.id,
        auth.user_id
    );
    Ok(Json(note))
}

/* [128A-1/F5][F5-5] Mapeo de errores del repo local: las validaciones de
 * negocio viajan como `sqlx::Error::Protocol(msg)` → 422 con el mensaje; el
 * `UNIQUE(user_id, serie, numero)` duplicado (23505) → 409 accionable. */
fn map_repo_error(err: sqlx::Error) -> AppError {
    if err
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code)
        .is_some_and(|c| c == "23505")
    {
        return AppError::Conflict(
            "Ya existe un albarán con esa serie y número (duplicado)".into(),
        );
    }
    match &err {
        sqlx::Error::RowNotFound => AppError::NotFound("Albarán no encontrado".into()),
        sqlx::Error::Protocol(msg) => AppError::Validation(msg.clone()),
        _ => AppError::from(err),
    }
}

/// Valida el formato estricto `YYYY-MM-DD` de la fecha de un albarán local.
fn validar_fecha_local(fecha: Option<&str>) -> Result<(), AppError> {
    if let Some(f) = fecha {
        if chrono::NaiveDate::parse_from_str(f, "%Y-%m-%d").is_err() {
            return Err(AppError::Validation(
                "Fecha inválida: formato esperado YYYY-MM-DD".into(),
            ));
        }
    }
    Ok(())
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
        assert!(!crate::services::bdp_purchase_sync::albaran_tiene_clave(&note));
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
        assert!(crate::services::bdp_purchase_sync::albaran_tiene_clave(&note));
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
        assert!(!crate::services::bdp_purchase_sync::albaran_tiene_clave(&note));
    }
}
