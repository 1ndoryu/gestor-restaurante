/* 128A-1/F6 y 198A-1/D8: Ventas — pagos locales, factura local y propina. */

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::Venta;
use crate::repositories::VentaRepository;
use crate::services::{
    payload_propina, verificar_permiso, AccionPermiso, BdpPushService, VentaService,
};
use crate::AppState;

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
    /* D11c: con `add_tip=false` un importe 0 es válido (deja la propina a cero);
     * sumar 0 no aporta nada, y un importe negativo nunca es válido. */
    let cero_no_valido = req.add_tip;
    if req.amount < rust_decimal::Decimal::ZERO
        || (cero_no_valido && req.amount == rust_decimal::Decimal::ZERO)
    {
        return Err(AppError::Validation(
            "La propina no puede ser negativa; al sumar debe ser mayor que cero".into(),
        ));
    }
    let venta = VentaService::get(&state.pool, id, auth.user_id).await?;
    if venta.anulada {
        return Err(AppError::Validation(
            "No se puede añadir propina a una venta anulada".into(),
        ));
    }

    let venta =
        VentaRepository::actualizar_propina(&state.pool, id, auth.user_id, req.amount, req.add_tip)
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
