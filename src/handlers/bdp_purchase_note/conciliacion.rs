/* [247A-11] Conciliación de albaranes locales con gastos (fail-closed, [169A-1]).
 * Fuera de mod.rs para mantener el módulo bajo el límite de tamaño. */

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{BdpPurchaseNoteReconcileRequest, BdpPurchaseNoteReconcileResult};
use crate::repositories::gasto::NuevoGasto;
use crate::repositories::{AuditoriaCicloLocal, BdpPurchaseNoteRepository, GastoRepository};
use crate::services::{verificar_permiso, AccionPermiso, ConfiguracionService, ModoEfectivo};
use crate::AppState;

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
    let note = validar_conciliable(&state, &auth, id).await?;

    let mut tx = state.pool.begin().await?;
    let (gasto_id, gasto_nuevo) = resolver_gasto(&mut tx, &note, id, &req, auth.user_id).await?;

    if !BdpPurchaseNoteRepository::vincular_gasto(&mut *tx, id, auth.user_id, gasto_id).await? {
        return Err(AppError::Validation(
            "El albarán ya está conciliado o no está en estado borrador".into(),
        ));
    }

    let accion = if req.gasto_existente_id.is_some() {
        "vinculado"
    } else {
        "creado"
    };
    auditar_conciliacion(
        &mut tx,
        auth.user_id,
        id,
        &note,
        gasto_id,
        accion,
        gasto_nuevo,
    )
    .await?;
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
        accion: accion.to_string(),
    }))
}

/// Comprueba permiso, modo y flag, y devuelve el albarán si sigue en borrador.
async fn validar_conciliable(
    state: &AppState,
    auth: &AuthUser,
    id: Uuid,
) -> Result<crate::models::BdpPurchaseNote, AppError> {
    /* [128A-1/F8] Permiso por acción: gestión de albaranes (D8/M17). */
    verificar_permiso(&state.pool, AccionPermiso::AlbaranesGestion, auth).await?;
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
    /* [128A-1/F5][M12] En `standalone` la conciliación local no consulta flags. */
    let modo = state
        .modo_operacion
        .modo_efectivo(&state.pool, auth.user_id)
        .await?;
    if modo == ModoEfectivo::Bdp && !config.ff_bdp_purchase_notes_receive {
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
    Ok(note)
}

/// Devuelve el gasto a vincular. Si lo crea, devuelve también los datos para auditar su alta.
async fn resolver_gasto(
    conn: &mut sqlx::PgConnection,
    note: &crate::models::BdpPurchaseNote,
    id: Uuid,
    req: &BdpPurchaseNoteReconcileRequest,
    user_id: Uuid,
) -> Result<
    (
        Uuid,
        Option<(String, rust_decimal::Decimal, rust_decimal::Decimal)>,
    ),
    AppError,
> {
    if let Some(gasto_existente_id) = req.gasto_existente_id {
        let gasto = GastoRepository::find_by_id(&mut *conn, gasto_existente_id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Gasto no encontrado".into()))?;
        return Ok((gasto.id, None));
    }

    let proveedor = note.nombre_proveedor.as_deref().unwrap_or("Proveedor BDP");
    let numero_documento = format!("{}-{}", note.serie, note.numero);
    let total = note.total.unwrap_or_default();
    let fecha = note
        .fecha
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    /* [128A-1/F5][A10] Los albaranes locales guardan IVA por línea en
     * `datos_bdp.lineas`; la conciliación usa ese desglose. Los albaranes
     * importados de BDP (sin desglose) registran el total como base. */
    /* [128A-1/F5][F5-4] El fallback a (total, 0) ya no es silencioso: se
     * loguea (importados BDP sin desglose, o locales sin líneas). */
    let (importe_base, importe_iva) =
        if let Some(d) = BdpPurchaseNoteRepository::desglose_desde_datos(&note.datos_bdp) {
            d
        } else {
            tracing::warn!(
                "[128A-1/F5-4] Sin desglose de líneas en albarán {} (origen {}): \
                 gasto con IVA 0 sobre total {}",
                id,
                note.origen,
                total
            );
            (total, rust_decimal::Decimal::ZERO)
        };
    let nuevo = NuevoGasto {
        user_id,
        fecha,
        proveedor,
        categoria_id: req.categoria_id,
        tipo_documento: "albaran",
        metodo_pago: "",
        numero_documento: &numero_documento,
        recurrente: false,
        importe_base,
        importe_iva,
    };
    let gasto = GastoRepository::create(&mut *conn, &nuevo).await?;
    Ok((
        gasto.id,
        Some((numero_documento, importe_base, importe_iva)),
    ))
}

/// Auditoría de la conciliación y, si el gasto es nuevo, de su alta.
/// [169A-1] Va en la misma transacción (fail-closed). El gasto solo se audita
/// como creado cuando lo crea esta conciliación; si se vincula uno existente,
/// su alta es anterior al rastro y no se fabrica.
async fn auditar_conciliacion(
    conn: &mut sqlx::PgConnection,
    user_id: Uuid,
    id: Uuid,
    note: &crate::models::BdpPurchaseNote,
    gasto_id: Uuid,
    accion: &str,
    gasto_nuevo: Option<(String, rust_decimal::Decimal, rust_decimal::Decimal)>,
) -> Result<(), AppError> {
    BdpPurchaseNoteRepository::auditar_ciclo_local(
        &mut *conn,
        user_id,
        AuditoriaCicloLocal {
            operacion: "albaran_local_conciliar",
            target_entity_type: "albaran",
            target_entity_id: id,
            datos: serde_json::json!({
                "serie": note.serie,
                "numero": note.numero,
                "gasto_id": gasto_id,
                "accion": accion,
            }),
            motivo: format!(
                "Albarán local {}-{} conciliado con gasto {} — operación interna, no requiere autorización BDP",
                note.serie, note.numero, gasto_id
            ),
            idempotency_key: &format!("albaran-local-conciliar-{id}"),
        },
    )
    .await?;
    if let Some((numero_documento, importe_base, importe_iva)) = gasto_nuevo {
        BdpPurchaseNoteRepository::auditar_ciclo_local(
            &mut *conn,
            user_id,
            AuditoriaCicloLocal {
                operacion: "gasto_local_crear",
                target_entity_type: "gasto",
                target_entity_id: gasto_id,
                datos: serde_json::json!({
                    "numero_documento": numero_documento,
                    "importe_base": importe_base,
                    "importe_iva": importe_iva,
                    "albaran_id": id,
                }),
                motivo: format!(
                    "Gasto {} creado al conciliar albarán local {}-{} — operación interna, no requiere autorización BDP",
                    numero_documento, note.serie, note.numero
                ),
                idempotency_key: &format!("gasto-local-albaran-{id}"),
            },
        )
        .await?;
    }
    Ok(())
}
