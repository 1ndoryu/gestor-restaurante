/* [267A-8] Sincronización de clientes Glory → BDP (CreateCustomer) y vinculación de existentes. */

use axum::extract::{Path, State};
use axum::Json;
use tracing::warn;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{Cliente, ConfiguracionRestaurante};
use crate::repositories::ClienteRepository;
use crate::services::{BdpExportCustomersRequest, BdpWeblinkClient, ConfiguracionService};
use crate::AppState;

use super::super::bdp_guard::exigir_modo_bdp;
use super::crear_customer::{armar_create_customer, ejecutar_create_customer};
use super::import::RecuentoImportacion;
use super::{customer_code, BdpCustomerSyncRequest};

/// Cliente ya existente en Glory: vincula su código BDP si no tiene ninguno.
pub(super) async fn sincronizar_existente(
    state: &AppState,
    cliente: &Cliente,
    codigo_bdp: i32,
    aplicar: bool,
    rec: &mut RecuentoImportacion,
) -> bool {
    /* Nunca reemplazar un vínculo BDP diferente basándonos solo en una
     * coincidencia heurística de teléfono/email. */
    match cliente.bdp_customer_code {
        None if aplicar => {
            match ClienteRepository::update_bdp_sync(
                &state.pool,
                cliente.id,
                Some(codigo_bdp),
                true,
                None,
            )
            .await
            {
                Ok(()) => {
                    rec.actualizados += 1;
                    false
                }
                Err(e) => {
                    warn!("[AUDIT-N5] Error actualizando sync BDP: {e}");
                    rec.error()
                }
            }
        }
        None => {
            rec.actualizados += 1;
            false
        }
        Some(vinculado) if vinculado == codigo_bdp => {
            rec.sin_cambios += 1;
            false
        }
        Some(_) => {
            rec.conflicto(cliente, codigo_bdp);
            false
        }
    }
}

/* [Fase 7.2] Push controlado de un cliente Glory a BDP (CreateCustomer).
 * Exige código explícito, verifica colisión y siempre usa Overwrite=false.
 *
 * [AUDIT-1.3] Reconciliación: si BDP crea el cliente pero Glory no recibe
 * respuesta (timeout/crash), la auditoría queda "ambiguo". La próxima
 * tentativa será bloqueada por ensure_no_unresolved(). El operador debe:
 *   1. Consultar BDP directamente para verificar si el cliente fue creado.
 *   2. Si fue creado, usar la importación (POST /bdp/customers/import)
 *      para vincular el cliente Glory al código BDP existente.
 *   3. Si no fue creado, limpiar la auditoría ambigua manualmente.
 * No hay MarketplaceOrderId para clientes (solo para comandas). */
#[utoipa::path(
    post,
    path = "/api/clientes/{id}/bdp-sync",
    tag = "BDP Clientes",
    params(("id" = Uuid, Path, description = "ID del cliente")),
    request_body = BdpCustomerSyncRequest,
    responses(
        (status = 200, description = "Cliente sincronizado con BDP", body = serde_json::Value),
        (status = 400, description = "BDP no configurado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 404, description = "Cliente no encontrado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
/* [187A-1] La secuencia de seguridad se mantiene lineal para que confirmación,
 * preflight, autorización, llamada única y cierre de auditoría sean auditables
 * en un solo flujo y no pueda omitirse accidentalmente una guarda. */
pub async fn sincronizar_cliente_bdp(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(sync_req): Json<BdpCustomerSyncRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    exigir_modo_bdp(&state, auth.user_id).await?;
    let cliente = ClienteRepository::find_by_id(&state.pool, id, auth.user_id)
        .await
        .map_err(|e| AppError::Internal(format!("Error buscando cliente: {e}")))?
        .ok_or_else(|| AppError::NotFound("Cliente no encontrado".into()))?;

    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;
    let bdp_code = validar_puerta_sincronizacion(&cliente, &sync_req, &config)?;

    let client = BdpWeblinkClient::new(&config);
    if let Some(respuesta) = vincular_si_existe(&client, &state, &cliente, bdp_code).await? {
        return Ok(respuesta);
    }

    let (req, audit_id) =
        armar_create_customer(&state, auth.user_id, &config, &cliente, bdp_code).await?;

    ejecutar_create_customer(&client, &state, &req, audit_id, &cliente, bdp_code).await
}

/* [267A-8] Puerta de sincronización: confirmación exacta + config + modo +
 * backup + código explícito + no-sobrescritura. Devuelve el código validado. */
fn validar_puerta_sincronizacion(
    cliente: &Cliente,
    sync_req: &BdpCustomerSyncRequest,
    config: &ConfiguracionRestaurante,
) -> Result<i32, AppError> {
    let expected_confirmation = format!(
        "CREAR CLIENTE {} {} {}",
        cliente.nombre, cliente.apellidos, sync_req.bdp_customer_code
    );
    if sync_req.confirmacion.trim() != expected_confirmation {
        return Err(AppError::Validation(format!(
            "Confirmación inválida. Escriba exactamente: {expected_confirmation}"
        )));
    }

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation(
            "BDP no está configurado. Configura URL y credenciales primero.".into(),
        ));
    }

    if !crate::services::BdpWriteGuard::modo_permite_escritura(&config.bdp_sync_mode) {
        return Err(AppError::Validation(
            "BDP está en modo solo lectura; no se ejecutó ninguna escritura.".into(),
        ));
    }
    if !config.bdp_auto_backup_before_write {
        return Err(AppError::Validation(
            "Escritura BDP bloqueada: auto-backup pre-write desactivado.".into(),
        ));
    }

    if sync_req.bdp_customer_code <= 0 {
        return Err(AppError::Validation(
            "Debe indicar un bdp_customer_code explícito mayor que cero.".into(),
        ));
    }

    /* Nunca sobrescribir ni generar códigos. */
    if cliente.bdp_customer_code.is_some() {
        return Err(AppError::Validation(
            "Cliente BDP existente: Overwrite está bloqueado y requiere una autorización específica no implementada."
                .into(),
        ));
    }
    Ok(sync_req.bdp_customer_code)
}

/* [267A-8] Preflight de identidad/código contra BDP. Si el código ya existe y
 * la identidad coincide, vincula localmente y devuelve la respuesta temprana;
 * si pertenece a otro cliente, error de conflicto. `None` = vía libre. */
async fn vincular_si_existe(
    client: &BdpWeblinkClient<'_>,
    state: &AppState,
    cliente: &Cliente,
    bdp_code: i32,
) -> Result<Option<Json<serde_json::Value>>, AppError> {
    let _session = client
        .login()
        .await
        .map_err(|e| AppError::Internal(format!("Error login BDP: {e}")))?;

    /* Preflight obligatorio de identidad/código. Overwrite=false protege la
     * carrera final, pero primero evitamos intentar una colisión conocida. */
    let exported = client
        .export_customers(&BdpExportCustomersRequest::default())
        .await
        .map_err(|e| AppError::Internal(format!("No se pudo verificar el código BDP: {e}")))?;
    let remote = exported
        .get("Customers")
        .and_then(serde_json::Value::as_array)
        .and_then(|customers| {
            customers
                .iter()
                .find(|customer| customer_code(customer) == Some(bdp_code))
        });
    if let Some(remote) = remote {
        let remote_phone = remote
            .get("MobilePhone")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .trim();
        let remote_email = remote
            .get("EMail")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .trim();
        let same_identity = (!cliente.telefono.trim().is_empty()
            && cliente.telefono.trim() == remote_phone)
            || (!cliente.email.trim().is_empty()
                && cliente.email.trim().eq_ignore_ascii_case(remote_email));
        if same_identity {
            ClienteRepository::update_bdp_sync(&state.pool, cliente.id, Some(bdp_code), true, None)
                .await
                .map_err(|e| AppError::Internal(format!("Error vinculando cliente BDP: {e}")))?;
            return Ok(Some(Json(serde_json::json!({
                "cliente_id": cliente.id,
                "bdp_customer_code": bdp_code,
                "bdp_synced": true,
                "linked_existing": true
            }))));
        }
        return Err(AppError::Conflict(format!(
            "El código BDP {bdp_code} ya pertenece a otro cliente; no se escribió nada."
        )));
    }
    Ok(None)
}
