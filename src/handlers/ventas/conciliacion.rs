/* [21AB-12] Conciliar una venta con lo que el TPV tiene ahora.
 *
 * Punto 3 de Guillermo (25/11): cuando la comanda se modificó en el TPV, la web
 * detecta el conflicto y no se reescribe sola; la persona tiene que poder
 * aceptar el estado del TPV desde la web. Este módulo es la superficie HTTP de
 * esa decisión; el servicio que la aplica está en
 * `services/bdp_sync_venta/convergencia.rs`.
 *
 * Vive aparte de `bdp.rs` por el límite de líneas por controlador. */

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::handlers::bdp_guard::{exigir_etapa_lectura, exigir_modo_bdp};
use crate::middleware::AuthUser;
use crate::AppState;

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct AceptarEstadoBdpRequest {
    /// Confirmación escrita: `ACEPTAR {id} {comanda}`. Sin ella no se toca nada.
    pub confirmacion: String,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct AceptarEstadoBdpResponse {
    pub venta_id: Uuid,
    pub bdp_order_id: i64,
    pub lineas: usize,
    pub total_antes: f64,
    pub total_ahora: f64,
}

/// Adopta el estado que el TPV tiene ahora para la comanda de la venta.
/// Nunca automático: exige la confirmación escrita y solo lee de BDP.
#[utoipa::path(
    post,
    path = "/api/ventas/{id}/aceptar-bdp",
    tag = "Ventas",
    params(("id" = Uuid, Path, description = "ID de la venta")),
    request_body = AceptarEstadoBdpRequest,
    responses(
        (status = 200, description = "Estado del TPV adoptado", body = AceptarEstadoBdpResponse),
        (status = 400, description = "Confirmación inválida", body = ErrorResponse),
        (status = 404, description = "Venta no encontrada", body = ErrorResponse),
        (status = 409, description = "Comanda anulada, facturada o sin estado que adoptar", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn aceptar_estado_bdp(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AceptarEstadoBdpRequest>,
) -> Result<Json<AceptarEstadoBdpResponse>, AppError> {
    /* Sustituir líneas es la misma clase de escritura destructiva sobre el
     * histórico de ventas que anular o borrar: mismo permiso. */
    crate::services::verificar_permiso(
        &state.pool,
        crate::services::AccionPermiso::AnulacionVentas,
        &auth,
    )
    .await?;
    exigir_modo_bdp(&state, auth.user_id).await?;
    exigir_etapa_lectura(&state, auth.user_id).await?;

    let venta = crate::repositories::VentaRepository::find_by_id(&state.pool, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Venta no encontrada".into()))?;
    let comanda = venta.bdp_order_id.unwrap_or(0);
    let esperada = format!("ACEPTAR {id} {comanda}");
    if comanda <= 0 || req.confirmacion.trim() != esperada {
        return Err(AppError::Validation(format!(
            "Confirmación inválida. Escriba exactamente: {esperada}"
        )));
    }

    let config = crate::services::ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
    let resultado =
        crate::services::BdpSyncService::aceptar_estado_tpv(&state.pool, &venta, &config)
            .await
            .map_err(AppError::Conflict)?;

    crate::services::DashboardService::invalidar_resumen(&state, auth.user_id).await;
    crate::middleware::invalidar_listado(&state, auth.user_id, crate::middleware::ENDPOINT_VENTAS)
        .await;

    Ok(Json(AceptarEstadoBdpResponse {
        venta_id: id,
        bdp_order_id: comanda,
        lineas: resultado.lineas.len(),
        total_antes: resultado.total_antes,
        total_ahora: resultado.total_bdp,
    }))
}
