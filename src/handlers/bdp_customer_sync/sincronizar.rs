/* [267A-8] Sincronización de clientes Glory → BDP (CreateCustomer) y vinculación de existentes. */

use axum::extract::{Path, State};
use axum::Json;
use sqlx::PgPool;
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
    pool: &PgPool,
    cliente: &Cliente,
    codigo_bdp: i32,
    aplicar: bool,
    rec: &mut RecuentoImportacion,
) -> bool {
    /* Nunca reemplazar un vínculo BDP diferente basándonos solo en una
     * coincidencia heurística de teléfono/email. */
    match cliente.bdp_customer_code {
        None if aplicar => {
            match ClienteRepository::update_bdp_sync(pool, cliente.id, Some(codigo_bdp), true, None)
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
 * Exige código explícito, verifica colisión y usa Overwrite=false salvo la
 * autorización de D25 (`bdp_overwrite_clientes`, solo BDP local).
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
    let overwrite = overwrite_autorizado(
        cliente.bdp_customer_code,
        bdp_code,
        config.bdp_overwrite_clientes,
        &config.bdp_base_url,
    );

    let client = BdpWeblinkClient::new(&config);
    /* Con Overwrite autorizado el objetivo es reemplazar el registro ya vinculado,
     * así que el preflight de colisión no aplica. */
    if !overwrite {
        if let Some(respuesta) =
            vincular_si_existe(&client, &state.pool, &cliente, bdp_code).await?
        {
            return Ok(respuesta);
        }
    }

    let (req, audit_id) = armar_create_customer(
        &state.pool,
        auth.user_id,
        &config,
        &cliente,
        bdp_code,
        overwrite,
    )
    .await?;

    ejecutar_create_customer(&client, &state.pool, &req, audit_id, &cliente, bdp_code).await
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

    /* Nunca generar códigos. Un cliente ya vinculado solo se reescribe con la
     * autorización de Overwrite (D25); cualquier otro vínculo queda bloqueado. */
    let vinculado = cliente.bdp_customer_code;
    if vinculado.is_some()
        && !overwrite_autorizado(
            vinculado,
            sync_req.bdp_customer_code,
            config.bdp_overwrite_clientes,
            &config.bdp_base_url,
        )
    {
        return Err(AppError::Validation(
            "Cliente BDP existente: Overwrite requiere bdp_overwrite_clientes activo, BDP en local y el mismo código ya vinculado."
                .into(),
        ));
    }
    Ok(sync_req.bdp_customer_code)
}

/* [D25] Overwrite=true solo si: flag activo, BDP en loopback y el código pedido
 * es el ya vinculado a este cliente. Nunca re-apunta un cliente a otro código. */
fn overwrite_autorizado(
    vinculado: Option<i32>,
    bdp_code: i32,
    flag_activo: bool,
    base_url: &str,
) -> bool {
    flag_activo && vinculado == Some(bdp_code) && es_bdp_local(base_url)
}

/* [D25] BDP local = host de loopback (mismo criterio que `ensure_target_allowed`). */
fn es_bdp_local(base_url: &str) -> bool {
    reqwest::Url::parse(base_url.trim())
        .ok()
        .and_then(|url| url.host_str().map(str::to_ascii_lowercase))
        .is_some_and(|host| matches!(host.as_str(), "localhost" | "127.0.0.1" | "[::1]" | "::1"))
}

/* [267A-8] Preflight de identidad/código contra BDP. Si el código ya existe y
 * la identidad coincide, vincula localmente y devuelve la respuesta temprana;
 * si pertenece a otro cliente, error de conflicto. `None` = vía libre. */
async fn vincular_si_existe(
    client: &BdpWeblinkClient<'_>,
    pool: &PgPool,
    cliente: &Cliente,
    bdp_code: i32,
) -> Result<Option<Json<serde_json::Value>>, AppError> {
    let _session = client
        .login()
        .await
        .map_err(|e| AppError::Internal(format!("Error login BDP: {e}")))?;

    /* Preflight obligatorio de identidad/código. Overwrite=false (sin D25) protege la
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
            ClienteRepository::update_bdp_sync(pool, cliente.id, Some(bdp_code), true, None)
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

#[cfg(test)]
mod tests {
    use super::{es_bdp_local, overwrite_autorizado};

    /* [D25] Overwrite solo con flag, mismo código ya vinculado y BDP en loopback. */
    #[test]
    fn overwrite_exige_flag_mismo_codigo_y_bdp_local() {
        let local = "http://127.0.0.1:8070";
        assert!(overwrite_autorizado(Some(7), 7, true, local));
        assert!(!overwrite_autorizado(Some(7), 7, false, local));
        assert!(!overwrite_autorizado(Some(7), 8, true, local));
        assert!(!overwrite_autorizado(None, 7, true, local));
        assert!(!overwrite_autorizado(
            Some(7),
            7,
            true,
            "https://bdp.ejemplo.es"
        ));
    }

    #[test]
    fn bdp_local_solo_loopback() {
        assert!(es_bdp_local("http://localhost:8070"));
        assert!(es_bdp_local("http://[::1]:8070"));
        assert!(!es_bdp_local("http://192.168.1.10:8070"));
        assert!(!es_bdp_local("no es una url"));
    }
}
