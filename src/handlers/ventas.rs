/* 253A-5: Handlers de ventas — CRUD endpoints */

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    ActualizarVentaRequest, AnularVentaRequest, CrearVentaRequest, Venta, VentaLinea,
    VentasPaginadas, VentasQuery,
};
use crate::repositories::VentaRepository;
use crate::services::{
    payload_propina, verificar_permiso, AccionPermiso, BdpOrderPollerService, BdpPushService,
    BdpSyncService, ModoEfectivo, ServicioModoOperacion, VentaService,
};
use crate::AppState;

/// Crear una venta
#[utoipa::path(
    post,
    path = "/api/ventas",
    tag = "Ventas",
    request_body = CrearVentaRequest,
    responses(
        (status = 201, description = "Venta creada", body = Venta),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 422, description = "Error de validación", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn crear_venta(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CrearVentaRequest>,
) -> Result<(StatusCode, Json<Venta>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let venta = VentaService::create(&state.pool, auth.user_id, req).await?;
    Ok((StatusCode::CREATED, Json(venta)))
}

/// Obtener una venta por ID
#[utoipa::path(
    get,
    path = "/api/ventas/{id}",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    responses(
        (status = 200, description = "Venta encontrada", body = Venta),
        (status = 404, description = "Venta no encontrada", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn obtener_venta(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Venta>, AppError> {
    let venta = VentaService::get(&state.pool, id, auth.user_id).await?;
    Ok(Json(venta))
}

#[utoipa::path(
    get,
    path = "/api/ventas/{id}/lineas",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    responses(
        (status = 200, description = "Líneas de la venta", body = [VentaLinea]),
        (status = 404, description = "Venta no encontrada", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn obtener_lineas_venta(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<VentaLinea>>, AppError> {
    VentaService::get(&state.pool, id, auth.user_id).await?;
    let lineas =
        crate::repositories::VentaLineaRepository::listar_por_venta(&state.pool, id).await?;
    Ok(Json(lineas))
}

/// Listar ventas con paginación y filtros de fecha
#[utoipa::path(
    get,
    path = "/api/ventas",
    tag = "Ventas",
    params(VentasQuery),
    responses(
        (status = 200, description = "Lista de ventas", body = VentasPaginadas),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_ventas(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<VentasQuery>,
) -> Result<Json<VentasPaginadas>, AppError> {
    let ventas = VentaService::list(
        &state.pool,
        auth.user_id,
        params.page,
        params.per_page,
        params.desde,
        params.hasta,
        params.busqueda,
        params.turno,
        params.canal,
        params.metodo_pago,
        params.estado_haddock,
        params.estado_bdp,
        params.sort_by,
        params.sort_order,
    )
    .await?;
    Ok(Json(ventas))
}

/// Actualizar una venta
#[utoipa::path(
    put,
    path = "/api/ventas/{id}",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    request_body = ActualizarVentaRequest,
    responses(
        (status = 200, description = "Venta actualizada", body = Venta),
        (status = 404, description = "Venta no encontrada", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 422, description = "Error de validación", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn actualizar_venta(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ActualizarVentaRequest>,
) -> Result<Json<Venta>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let venta = VentaService::update(&state.pool, id, auth.user_id, req).await?;
    Ok(Json(venta))
}

/// Eliminar una venta
#[utoipa::path(
    delete,
    path = "/api/ventas/{id}",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    responses(
        (status = 204, description = "Venta eliminada"),
        (status = 404, description = "Venta no encontrada", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn eliminar_venta(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    /* [128A-1/F8-2] Escritura sensible (histórico fiscal local): el DELETE
     * exige el mismo permiso que la anulación. Decisión documentada: reusar
     * `AnulacionVentas` en vez de añadir una acción dedicada (misma clase de
     * escritura destructiva sobre ventas, default 'admin'). */
    verificar_permiso(&state.pool, AccionPermiso::AnulacionVentas, &auth).await?;
    VentaService::delete(&state.pool, id, auth.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/* [128A-1/F4] POST /api/ventas/:id/anular — Anulación local de ventas (D4).
 * Modalidades configuradas en `anulacion_modalidad` (credito_completo default |
 * estado_solo). Confirmación dinámica `ANULAR {id}` (patrón PAGAR/FACTURAR).
 * M9: solo ventas no facturadas. C3=b: sin llamada CancelOrder. */
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct AnularVentaResponse {
    pub venta: Venta,
    pub anulada: bool,
}

#[utoipa::path(
    post,
    path = "/api/ventas/{id}/anular",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    request_body = AnularVentaRequest,
    responses(
        (status = 200, description = "Venta anulada", body = AnularVentaResponse),
        (status = 404, description = "Venta no encontrada", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 409, description = "Venta facturada o idempotency_key ya usada con otro resultado", body = ErrorResponse),
        (status = 422, description = "Motivo obligatorio en credito_completo", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn anular_venta(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AnularVentaRequest>,
) -> Result<Json<AnularVentaResponse>, AppError> {
    /* [128A-1/F8] Permiso por acción: anulación de ventas (D8/M17). */
    verificar_permiso(&state.pool, AccionPermiso::AnulacionVentas, &auth).await?;
    let venta = VentaService::anular(&state.pool, id, auth.user_id, req).await?;
    Ok(Json(AnularVentaResponse {
        anulada: venta.anulada,
        venta,
    }))
}

/// Reintentar sincronización con Haddock
#[utoipa::path(
    post,
    path = "/api/ventas/{id}/haddock-sync",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    responses(
        (status = 200, description = "Sincronización completada", body = Venta),
        (status = 404, description = "Venta no encontrada", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 422, description = "Sync no habilitado o sin token", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn reintentar_sync_haddock(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Venta>, AppError> {
    let venta = VentaService::retry_haddock_sync(&state.pool, id, auth.user_id).await?;
    Ok(Json(venta))
}

/// Reintentar sincronización con `BDP` `WebLink`
#[utoipa::path(
    post,
    path = "/api/ventas/{id}/bdp-sync",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    request_body = ReintentarBdpSyncRequest,
    responses(
        (status = 200, description = "Sincronización BDP completada", body = Venta),
        (status = 404, description = "Venta no encontrada", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 422, description = "BDP sync no habilitado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn reintentar_sync_bdp(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ReintentarBdpSyncRequest>,
) -> Result<Json<Venta>, AppError> {
    /* [C1-3] Auto-arming: si el request incluye idempotency_key y auto_arm,
     * verificamos duplicados y, si no existe, creamos un armado temporal. */
    if let Some(idempotency_key) = &req.idempotency_key {
        if let Some((_audit_id, resultado)) = crate::services::BdpWriteGuard::check_idempotency(
            &state.pool,
            auth.user_id,
            idempotency_key,
        )
        .await
        .map_err(AppError::Internal)?
        {
            if resultado == "exito" {
                /* Idempotente: la operación ya fue exitosa. */
                let venta = VentaService::get(&state.pool, id, auth.user_id).await?;
                return Ok(Json(venta));
            }
        }
    }

    if req.auto_arm {
        let config =
            crate::services::ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
        let confirmation = req.confirmation_text.unwrap_or_default();
        crate::services::BdpWriteGuard::try_auto_arm(
            &state.pool,
            auth.user_id,
            &config,
            "create_order",
            "venta",
            id,
            &confirmation,
        )
        .await
        .map_err(AppError::Validation)?;
    }

    let idempotency_key = req.idempotency_key.as_deref();
    let venta =
        VentaService::retry_bdp_sync(&state.pool, id, auth.user_id, idempotency_key).await?;
    Ok(Json(venta))
}

#[derive(serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct ReintentarBdpSyncRequest {
    #[serde(default)]
    pub auto_arm: bool,
    pub confirmation_text: Option<String>,
    pub idempotency_key: Option<String>,
}

/* [276A-4.3] GET /api/ventas/:id/bdp-status — Consulta el estado BDP de una venta.
 * Llama a GetOrder en BDP para obtener el status actual y actualiza bdp_order_status. */
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct BdpOrderStatusResponse {
    pub venta_id: Uuid,
    pub bdp_order_id: Option<i64>,
    pub bdp_order_status: Option<String>,
    pub bdp_synced: bool,
    pub bdp_sync_error: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/ventas/{id}/bdp-status",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    responses(
        (status = 200, description = "Estado BDP de la venta", body = BdpOrderStatusResponse),
        (status = 404, description = "Venta no encontrada", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn obtener_bdp_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<BdpOrderStatusResponse>, AppError> {
    let config = crate::services::ConfiguracionService::obtener(&state.pool, auth.user_id).await?;

    /* Buscar la venta y refrescar su status vía BDP si tiene order_id */
    let venta = crate::repositories::VentaRepository::find_by_id(&state.pool, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Venta no encontrada".into()))?;

    /* Si tiene order_id y BDP está configurado, refrescar solo esta venta. */
    if venta.bdp_order_id.is_some()
        && ServicioModoOperacion::modo_efectivo_desde_config(&config) == ModoEfectivo::Bdp
    {
        let _ = BdpOrderPollerService::refresh_one(&state.pool, &venta, &config).await;
    }

    /* Releer la venta para obtener el estado actualizado */
    let venta = crate::repositories::VentaRepository::find_by_id(&state.pool, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Venta no encontrada".into()))?;

    Ok(Json(BdpOrderStatusResponse {
        venta_id: venta.id,
        bdp_order_id: venta.bdp_order_id,
        bdp_order_status: venta.bdp_order_status,
        bdp_synced: venta.bdp_synced,
        bdp_sync_error: venta.bdp_sync_error,
    }))
}

/* [276A-4.2] POST /api/ventas/bdp-poll — Dispara polling manual de todas las ventas pendientes. */
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct BdpPollResponse {
    pub updated: usize,
}

#[utoipa::path(
    post,
    path = "/api/ventas/bdp-poll",
    tag = "Ventas",
    responses(
        (status = 200, description = "Polling completado", body = BdpPollResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn bdp_poll(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<BdpPollResponse>, AppError> {
    let config = crate::services::ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
    let updated = BdpOrderPollerService::poll_pending(
        &state.pool,
        auth.user_id,
        &config,
        &state.modo_operacion,
    )
    .await
    .map_err(AppError::Validation)?;
    Ok(Json(BdpPollResponse { updated }))
}

/* [F8.5] POST /api/ventas/:id/bdp-invoice — Facturar una orden BDP existente.
 * Llama a InvoiceOrder en BDP y marca la venta como facturada.
 * ⚠️ Requiere bdp_order_id (la venta debe estar sincronizada con BDP primero). */
#[derive(serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct BdpInvoiceRequest {
    pub confirmacion: String,
    #[serde(default)]
    pub auto_arm: bool,
    pub confirmation_destino: Option<String>,
    pub idempotency_key: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct BdpPaymentRequest {
    pub amount: rust_decimal::Decimal,
    pub tender_id: i32,
    pub confirmacion: String,
    #[serde(default)]
    pub auto_arm: bool,
    pub confirmation_destino: Option<String>,
    pub idempotency_key: Option<String>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct BdpPaymentResponse {
    pub venta_id: Uuid,
    pub registrado: bool,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct BdpPaymentHistoryItem {
    pub id: Uuid,
    pub amount: rust_decimal::Decimal,
    pub tender_id: i32,
    pub resultado: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct BdpPaymentsResponse {
    pub venta_id: Uuid,
    pub total: rust_decimal::Decimal,
    pub pagado: rust_decimal::Decimal,
    pub pendiente: rust_decimal::Decimal,
    pub pagos: Vec<BdpPaymentHistoryItem>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct BdpInvoiceResponse {
    pub venta_id: Uuid,
    pub invoice_number: String,
    pub bdp_invoiced: bool,
}

#[utoipa::path(
    post,
    path = "/api/ventas/{id}/bdp-invoice",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    request_body = BdpInvoiceRequest,
    responses(
        (status = 200, description = "Orden facturada en BDP", body = BdpInvoiceResponse),
        (status = 404, description = "Venta no encontrada", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 422, description = "Venta no sincronizada con BDP o BDP no habilitado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn bdp_invoice(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<BdpInvoiceRequest>,
) -> Result<Json<BdpInvoiceResponse>, AppError> {
    let expected_confirmation = format!("FACTURAR {id}");
    if req.confirmacion.trim() != expected_confirmation {
        return Err(AppError::Validation(format!(
            "Confirmación inválida. Escriba exactamente: {expected_confirmation}"
        )));
    }
    let config = crate::services::ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
    /* [128A-1/F1-2] M2: en degradación/standalone se bloquea la escritura con
     * mensaje claro en lugar de intentar y fallar en silencio. */
    if state.modo_operacion.modo_efectivo_sin_red(&config) != ModoEfectivo::Bdp {
        return Err(AppError::Validation(
            "BDP no disponible: el sistema está en modo independiente.".into(),
        ));
    }

    let venta = crate::repositories::VentaRepository::find_by_id(&state.pool, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Venta no encontrada".into()))?;

    if venta.bdp_order_id.is_none() {
        return Err(AppError::Validation(
            "La venta no tiene bdp_order_id — sincróniza con BDP primero".into(),
        ));
    }
    /* [128A-1/F6] M9: doble facturación — si ya se facturó localmente, el
     * flujo BDP no debe volver a facturar la misma venta. */
    if venta.facturada_local {
        return Err(AppError::Validation(
            "La venta ya está facturada localmente (factura local).".into(),
        ));
    }

    if req.auto_arm {
        let destino = req.confirmation_destino.unwrap_or_default();
        crate::services::BdpWriteGuard::try_auto_arm(
            &state.pool,
            auth.user_id,
            &config,
            "invoice",
            "venta",
            id,
            &destino,
        )
        .await
        .map_err(AppError::Validation)?;
    }

    let idempotency_key = req.idempotency_key.as_deref();
    let invoice_number =
        match BdpSyncService::invoice_order(&state.pool, &venta, &config, idempotency_key).await {
            Ok(number) => {
                state.modo_operacion.registrar_exito_bdp(auth.user_id);
                number
            }
            Err(ref e) if e.starts_with("idempotencia_duplicada:") => {
                /* [C1-6] Idempotencia: si la operación ya fue exitosa, devolvemos el
                 * estado actual en lugar de 422. */
                let resultado = e.splitn(3, ':').nth(2).unwrap_or("");
                if resultado == "exito" {
                    /* [C1-6] Recuperar el InvoiceNumber de la auditoría exitosa previa. */
                    let invoice_number: Option<String> = sqlx::query_scalar(
                        r"SELECT datos_respuesta ->> 'InvoiceNumber'
                       FROM bdp_audit_log
                       WHERE user_id = $1
                         AND operacion = 'invoice'
                         AND target_entity_type = 'venta'
                         AND target_entity_id = $2
                         AND resultado = 'exito'
                       ORDER BY created_at DESC
                       LIMIT 1",
                    )
                    .bind(auth.user_id)
                    .bind(id)
                    .fetch_optional(&state.pool)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                    return Ok(Json(BdpInvoiceResponse {
                        venta_id: venta.id,
                        invoice_number: invoice_number.unwrap_or_default(),
                        bdp_invoiced: true,
                    }));
                }
                return Err(AppError::Validation(e.clone()));
            }
            Err(e) => {
                state.modo_operacion.registrar_fallo_bdp(auth.user_id);
                return Err(AppError::Validation(e));
            }
        };

    Ok(Json(BdpInvoiceResponse {
        venta_id: venta.id,
        invoice_number,
        bdp_invoiced: true,
    }))
}

#[utoipa::path(
    post,
    path = "/api/ventas/{id}/bdp-payment",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    request_body = BdpPaymentRequest,
    responses(
        (status = 200, description = "Pago registrado en BDP", body = BdpPaymentResponse),
        (status = 422, description = "Pago bloqueado por validación o estado ambiguo", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn bdp_payment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<BdpPaymentRequest>,
) -> Result<Json<BdpPaymentResponse>, AppError> {
    if req.amount <= rust_decimal::Decimal::ZERO || req.tender_id <= 0 {
        return Err(AppError::Validation(
            "El pago requiere amount mayor que cero y tender_id válido.".into(),
        ));
    }
    if req.amount != req.amount.round_dp(2) {
        return Err(AppError::Validation(
            "El pago BDP admite como máximo dos decimales.".into(),
        ));
    }
    let expected_confirmation = format!("PAGAR {id} {:.2}", req.amount);
    if req.confirmacion.trim() != expected_confirmation {
        return Err(AppError::Validation(format!(
            "Confirmación inválida. Escriba exactamente: {expected_confirmation}"
        )));
    }
    let config = crate::services::ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
    /* [128A-1/F1-2] M2: en degradación/standalone se bloquea la escritura. */
    if state.modo_operacion.modo_efectivo_sin_red(&config) != ModoEfectivo::Bdp {
        return Err(AppError::Validation(
            "BDP no disponible: el sistema está en modo independiente.".into(),
        ));
    }
    let venta = crate::repositories::VentaRepository::find_by_id(&state.pool, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Venta no encontrada".into()))?;

    if req.auto_arm {
        let destino = req.confirmation_destino.unwrap_or_default();
        crate::services::BdpWriteGuard::try_auto_arm(
            &state.pool,
            auth.user_id,
            &config,
            "add_payment",
            "venta",
            id,
            &destino,
        )
        .await
        .map_err(AppError::Validation)?;
    }

    let idempotency_key = req.idempotency_key.as_deref();
    match BdpSyncService::add_order_payment(
        &state.pool,
        &venta,
        &config,
        req.amount,
        req.tender_id,
        idempotency_key,
    )
    .await
    {
        Ok(_) => state.modo_operacion.registrar_exito_bdp(auth.user_id),
        Err(ref e) if e.starts_with("idempotencia_duplicada:") => {
            /* [C1-6] Idempotencia: si la operación ya fue exitosa, devolvemos el
             * estado actual en lugar de 422. */
            let resultado = e.splitn(3, ':').nth(2).unwrap_or("");
            if resultado != "exito" {
                return Err(AppError::Validation(e.clone()));
            }
        }
        Err(e) => {
            state.modo_operacion.registrar_fallo_bdp(auth.user_id);
            return Err(AppError::Validation(e));
        }
    }

    Ok(Json(BdpPaymentResponse {
        venta_id: id,
        registrado: true,
    }))
}

/* [247A-9] GET /api/ventas/:id/bdp-payments — Historial y balance de pagos
 * parciales de una venta. Incluye pagos locales del ledger, independientes de
 * los pagos que se hayan registrado directamente en BDP. */

#[utoipa::path(
    get,
    path = "/api/ventas/{id}/bdp-payments",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    responses(
        (status = 200, description = "Historial de pagos BDP", body = BdpPaymentsResponse),
        (status = 404, description = "Venta no encontrada", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_bdp_payments(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<BdpPaymentsResponse>, AppError> {
    let venta = crate::repositories::VentaRepository::find_by_id(&state.pool, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Venta no encontrada".into()))?;

    let pagos = crate::repositories::BdpPagoRepository::listar_por_venta(&state.pool, id).await?;
    let pagado = crate::repositories::BdpPagoRepository::total_pagado(&state.pool, id).await?;
    let total = venta.importe_base + venta.importe_iva;
    let pendiente = (total - pagado).max(rust_decimal::Decimal::ZERO);

    Ok(Json(BdpPaymentsResponse {
        venta_id: id,
        total,
        pagado,
        pendiente,
        pagos: pagos
            .into_iter()
            .map(|p| BdpPaymentHistoryItem {
                id: p.id,
                amount: p.amount,
                tender_id: p.tender_id,
                resultado: p.resultado,
                created_at: p.created_at,
            })
            .collect(),
    }))
}

/* [128A-1/F6] POST /api/ventas/:id/pagos-locales — Pago parcial local (A8/M13).
 * Escribe sobre el ledger existente `bdp_pagos` (fila local, sin bdp_order_id)
 * con idempotencia, saldo pendiente y guards M9. Confirmación dinámica
 * `PAGO LOCAL {id} {amount:.2}` (patrón PAGAR/FACTURAR). Con BDP se conserva
 * el flujo actual (`bdp-payment`); este endpoint cubre el caso local. */
#[derive(serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct PagoLocalRequest {
    pub amount: rust_decimal::Decimal,
    pub tender_id: i32,
    pub confirmacion: String,
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct PagoLocalResponse {
    pub venta_id: Uuid,
    pub pago: crate::models::BdpPago,
    pub duplicado: bool,
    pub total: rust_decimal::Decimal,
    pub pagado: rust_decimal::Decimal,
    pub pendiente: rust_decimal::Decimal,
}

#[utoipa::path(
    post,
    path = "/api/ventas/{id}/pagos-locales",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    request_body = PagoLocalRequest,
    responses(
        (status = 200, description = "Pago parcial local registrado", body = PagoLocalResponse),
        (status = 404, description = "Venta no encontrada", body = ErrorResponse),
        (status = 409, description = "Venta anulada/facturada o idempotency_key ya usada", body = ErrorResponse),
        (status = 422, description = "Confirmación inválida o importe fuera del saldo pendiente", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn pago_parcial_local(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<PagoLocalRequest>,
) -> Result<Json<PagoLocalResponse>, AppError> {
    /* [128A-1/F8-1] Operación monetaria: requiere permiso operativo aunque el
     * modo efectivo sea standalone (las variantes locales de F6 no son
     * BDP-bound). */
    verificar_permiso(&state.pool, AccionPermiso::PagosLocales, &auth).await?;
    /* [128A-1/F6][F6-6] Contrato de `tender_id`: no existe tabla local de
     * tenders; el mapeo método_pago Glory → tender BDP vive en
     * `configuracion_restaurante.bdp_tender_map` (JSONB) y `bdp_pagos` no
     * tiene FK. La validación es `tender_id > 0` (referencia simbólica del
     * ledger); se documenta el contrato en vez de inventar una tabla nueva. */
    if req.amount <= rust_decimal::Decimal::ZERO || req.tender_id <= 0 {
        return Err(AppError::Validation(
            "El pago requiere amount mayor que cero y tender_id válido.".into(),
        ));
    }
    if req.amount != req.amount.round_dp(2) {
        return Err(AppError::Validation(
            "El pago local admite como máximo dos decimales.".into(),
        ));
    }
    let expected_confirmation = format!("PAGO LOCAL {id} {:.2}", req.amount);
    if req.confirmacion.trim() != expected_confirmation {
        return Err(AppError::Validation(format!(
            "Confirmación inválida. Escriba exactamente: {expected_confirmation}"
        )));
    }

    let (pago, audit_id) = crate::services::VentaService::pago_parcial_local(
        &state.pool,
        auth.user_id,
        id,
        req.amount,
        req.tender_id,
        req.idempotency_key.as_deref(),
    )
    .await?;

    /* Balance actualizado para la respuesta. */
    let venta = crate::repositories::VentaRepository::find_by_id(&state.pool, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Venta no encontrada".into()))?;
    let total = venta.importe_base + venta.importe_iva;
    let pagado = crate::repositories::BdpPagoRepository::total_pagado(&state.pool, id).await?;
    let pendiente = (total - pagado).max(rust_decimal::Decimal::ZERO);

    Ok(Json(PagoLocalResponse {
        venta_id: id,
        duplicado: audit_id.is_none(),
        total,
        pagado,
        pendiente,
        pago,
    }))
}

/* [128A-1/F6] POST /api/ventas/:id/factura-local — Factura local mínima (A7/D9).
 * Numeración local secuencial `F-{año}-{n}` + estado `facturada` + auditoría
 * con `origen_operacion='local'`. Confirmación `FACTURA LOCAL {id}`. Guards M9:
 * no facturar anuladas, ni doble facturación, ni con pagos parciales
 * pendientes en el ledger. Con BDP, `bdp-invoice` (InvoiceOrder) no cambia. */
#[derive(serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct FacturaLocalRequest {
    pub confirmacion: String,
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct FacturaLocalResponse {
    pub venta_id: Uuid,
    pub factura_numero: String,
    pub facturada: bool,
    pub venta: crate::models::Venta,
}

#[utoipa::path(
    post,
    path = "/api/ventas/{id}/factura-local",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    request_body = FacturaLocalRequest,
    responses(
        (status = 200, description = "Venta facturada localmente", body = FacturaLocalResponse),
        (status = 404, description = "Venta no encontrada", body = ErrorResponse),
        (status = 409, description = "Venta anulada/facturada o idempotency_key ya usada", body = ErrorResponse),
        (status = 422, description = "Confirmación inválida o pagos pendientes", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn factura_local(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<FacturaLocalRequest>,
) -> Result<Json<FacturaLocalResponse>, AppError> {
    /* [128A-1/F8-1] Emisión de factura local: operación monetaria protegida
     * por permiso operativo (igual que pago_parcial_local). */
    verificar_permiso(&state.pool, AccionPermiso::FacturacionLocal, &auth).await?;
    let expected_confirmation = format!("FACTURA LOCAL {id}");
    if req.confirmacion.trim() != expected_confirmation {
        return Err(AppError::Validation(format!(
            "Confirmación inválida. Escriba exactamente: {expected_confirmation}"
        )));
    }

    let venta = crate::services::VentaService::facturar_local(
        &state.pool,
        id,
        auth.user_id,
        req.idempotency_key.as_deref(),
    )
    .await?;

    Ok(Json(FacturaLocalResponse {
        venta_id: id,
        factura_numero: venta.factura_numero.clone().unwrap_or_default(),
        facturada: venta.facturada_local,
        venta,
    }))
}

/* [198A-1/D8] Propina por venta. Localmente guarda `ventas.propina`
 * (independiente de BDP); con BDP y `bdp_order_id`, encola AddOrderTip.
 * `add_tip`: true suma a la propina existente en BDP, false la sustituye. */
#[derive(serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct AgregarPropinaRequest {
    pub amount: rust_decimal::Decimal,
    #[serde(default = "default_add_tip")]
    pub add_tip: bool,
}

fn default_add_tip() -> bool {
    true
}

#[utoipa::path(
    post,
    path = "/api/ventas/{id}/propina",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    request_body = AgregarPropinaRequest,
    responses(
        (status = 200, description = "Propina guardada", body = Venta),
        (status = 404, description = "Venta no encontrada", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn agregar_propina(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AgregarPropinaRequest>,
) -> Result<Json<Venta>, AppError> {
    if req.amount <= rust_decimal::Decimal::ZERO {
        return Err(AppError::Validation(
            "La propina debe ser mayor que cero".into(),
        ));
    }
    let venta = VentaService::get(&state.pool, id, auth.user_id).await?;
    if venta.anulada {
        return Err(AppError::Validation(
            "No se puede añadir propina a una venta anulada".into(),
        ));
    }

    let venta = VentaRepository::actualizar_propina(&state.pool, id, auth.user_id, req.amount)
        .await?
        .ok_or_else(|| AppError::NotFound("Venta no encontrada".into()))?;

    /* [M16] Solo encolar si la comanda está en BDP; si no, queda local con el
     * aviso "comanda no sincronizada" (no es error del push). */
    if let Some(order_id) = venta.bdp_order_id {
        let payload =
            payload_propina(order_id, req.amount, req.add_tip).map_err(AppError::Internal)?;
        BdpPushService::encolar(
            &state.pool,
            auth.user_id,
            crate::services::bdp_push::DOMINIO_PROPINA,
            &id.to_string(),
            crate::services::bdp_push::OPERACION_PROPINA,
            &payload,
        )
        .await
        .map_err(AppError::Internal)?;
    }

    Ok(Json(venta))
}

/* [263A-15] Axum 0.7 (matchit 0.7.x) usa :param, no {param}.
 * Todas las rutas con path params corregidas de {id} a :id.
 * Las anotaciones #[utoipa::path] mantienen {id} (sintaxis OpenAPI, no afecta routing). */
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/ventas", post(crear_venta).get(listar_ventas))
        .route(
            "/ventas/:id",
            get(obtener_venta)
                .put(actualizar_venta)
                .delete(eliminar_venta),
        )
        .route("/ventas/:id/haddock-sync", post(reintentar_sync_haddock))
        .route("/ventas/:id/bdp-sync", post(reintentar_sync_bdp))
        .route("/ventas/:id/bdp-status", get(obtener_bdp_status))
        .route("/ventas/:id/lineas", get(obtener_lineas_venta))
        .route("/ventas/:id/bdp-payment", post(bdp_payment))
        .route("/ventas/:id/bdp-invoice", post(bdp_invoice))
        .route("/ventas/:id/anular", post(anular_venta))
        .route("/ventas/:id/bdp-payments", get(listar_bdp_payments))
        .route("/ventas/:id/pagos-locales", post(pago_parcial_local))
        .route("/ventas/:id/factura-local", post(factura_local))
        .route("/ventas/:id/propina", post(agregar_propina))
        .route("/ventas/bdp-poll", post(bdp_poll))
}
