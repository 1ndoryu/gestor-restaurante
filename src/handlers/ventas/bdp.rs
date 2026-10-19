/* 276A/247A/F8: Ventas — sincronización, pagos, facturación y estado BDP. */

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::Venta;
use crate::repositories::BdpAuditLogRepository;
use crate::services::{
    BdpOrderPollerService, BdpSyncService, ModoEfectivo, ServicioModoOperacion, VentaService,
};
use crate::AppState;

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

/* [D19] POST /api/ventas/:id/bdp-invoice-number — Registra el número de una factura que BDP
 * emitió pero cuya respuesta se perdió. Requiere la confirmación `REGISTRAR FACTURA {id}`. */
#[derive(serde::Deserialize, serde::Serialize)]
pub struct BdpRegistrarNumeroRequest {
    pub confirmacion: String,
    pub invoice_number: String,
}

pub async fn bdp_registrar_numero_factura(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<BdpRegistrarNumeroRequest>,
) -> Result<Json<BdpInvoiceResponse>, AppError> {
    let expected_confirmation = format!("REGISTRAR FACTURA {id}");
    if req.confirmacion.trim() != expected_confirmation {
        return Err(AppError::Validation(format!(
            "Confirmación inválida. Escriba exactamente: {expected_confirmation}"
        )));
    }
    let config = crate::services::ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
    if state.modo_operacion.modo_efectivo_sin_red(&config) != ModoEfectivo::Bdp {
        return Err(AppError::Validation(
            "BDP no disponible: el sistema está en modo independiente.".into(),
        ));
    }
    let venta = crate::repositories::VentaRepository::find_by_id(&state.pool, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Venta no encontrada".into()))?;

    let numero = req.invoice_number.trim().to_owned();
    BdpSyncService::registrar_numero_factura(&state.pool, &venta, &config, &numero)
        .await
        .map_err(AppError::Validation)?;
    Ok(Json(BdpInvoiceResponse {
        venta_id: venta.id,
        invoice_number: numero,
        bdp_invoiced: true,
    }))
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
                    /* [C1-6] Recuperar el InvoiceNumber de la auditoría exitosa previa.
                     * [267A-7] Lectura en repositorio; el handler conserva el flujo. */
                    let invoice_number: Option<String> =
                        BdpAuditLogRepository::ultimo_invoice_number_exitoso(
                            &state.pool,
                            auth.user_id,
                            id,
                        )
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

    /* Lecturas independientes: en paralelo, sin esperar una a la otra */
    let (pagos, pagado) = tokio::try_join!(
        crate::repositories::BdpPagoRepository::listar_por_venta(&state.pool, id),
        crate::repositories::BdpPagoRepository::total_pagado(&state.pool, id),
    )?;
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
