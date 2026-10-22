/* [267A-8] Armado y ejecución de CreateCustomer en BDP con auditoría; división de nombres. */

use axum::Json;
use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{Cliente, ConfiguracionRestaurante};
use crate::repositories::{BdpAuditLogRepository, ClienteRepository};
use crate::services::{BdpCreateCustomerRequest, BdpWeblinkClient};

/* [267A-8] Construye el payload CreateCustomer [048A-8] y deja la escritura
 * armada localmente (guard + snapshot pre-write + authorize). Devuelve el
 * request y el audit_id autorizado. */
pub(super) async fn armar_create_customer(
    pool: &PgPool,
    user_id: Uuid,
    config: &ConfiguracionRestaurante,
    cliente: &Cliente,
    bdp_code: i32,
    overwrite: bool,
) -> Result<(BdpCreateCustomerRequest, Uuid), AppError> {
    /* Construir nombre completo: apellidos + nombre */
    let fiscal_name = if cliente.apellidos.is_empty() {
        cliente.nombre.clone()
    } else {
        format!("{} {}", cliente.apellidos, cliente.nombre)
    };

    /* [D18d] El NIF viaja en FIN: sin él BDP da de alta el cliente sin identificación fiscal. */
    let fin = ClienteRepository::datos_fiscales(pool, cliente.id, user_id)
        .await
        .map_err(|e| AppError::Internal(format!("Error leyendo NIF del cliente: {e}")))?
        .map(|datos| datos.nif.trim().to_string())
        .unwrap_or_default();

    /* [048A-8] Payload completo del contrato CreateCustomer. El BDP real con
     * módulo de gestión devuelve NullReferenceException si faltan los campos
     * de gestión (PaymentMode, Representative, AreaCode, TAVCode, RateCode),
     * así que se envían todos los campos, con valores neutros seguros para
     * los que el cliente local no tiene. FINType=1 (N.I.F.) por defecto. */
    let req = BdpCreateCustomerRequest {
        code: bdp_code,
        fiscal_name,
        commercial_name: cliente.nombre.clone(),
        address: String::new(),
        post_code: String::new(),
        town: String::new(),
        province: String::new(),
        land_line: String::new(),
        mobile_phone: cliente.telefono.clone(),
        fin,
        fin_type: 1,
        email: cliente.email.clone(),
        per_discount: 0.0,
        payment_mode: 1,
        representative: 1,
        area_code: 1,
        tav_code: 1,
        rate_code: 1,
        overwrite,
    };

    /* El overwrite queda en datos_enviados para que la auditoría muestre si se sobrescribió. */
    let datos_cliente = serde_json::json!({
        "cliente_id": cliente.id,
        "bdp_customer_code": bdp_code,
        "overwrite": overwrite
    });
    crate::services::BdpWriteGuard::ensure_no_unresolved(
        pool,
        user_id,
        "cliente_id",
        cliente.id,
        &["create_customer"],
    )
    .await
    .map_err(AppError::Validation)?;
    let snapshot_pre_id = crate::services::BdpBackupService::preparar_snapshot_escritura(
        pool,
        user_id,
        "create_customer",
        config,
        None,
    )
    .await
    .map_err(|e| {
        AppError::Validation(format!(
            "Pre-write audit BDP falló; creación de cliente bloqueada: {e}"
        ))
    })?;

    let audit_id = crate::services::BdpWriteGuard::authorize(
        pool,
        user_id,
        config,
        crate::services::EscrituraAutorizar {
            scope: "create_customer",
            target_entity_type: "cliente",
            target_entity_id: cliente.id,
            entity_json_field: "cliente_id",
            datos_enviados: &datos_cliente,
            snapshot_pre_id,
            idempotency_key: None,
        },
    )
    .await
    .map_err(AppError::Validation)?;
    Ok((req, audit_id))
}

/* [267A-8] Ejecuta CreateCustomer en BDP, gestiona error HTTP y ErrorMessage,
 * y confirma marca local + auditoría en transacción atómica [AUDIT-N1]. */
pub(super) async fn ejecutar_create_customer(
    client: &BdpWeblinkClient<'_>,
    pool: &PgPool,
    req: &BdpCreateCustomerRequest,
    audit_id: Uuid,
    cliente: &Cliente,
    bdp_code: i32,
) -> Result<Json<serde_json::Value>, AppError> {
    let resp = match client.create_customer(req).await {
        Ok(resp) => resp,
        Err(e) => {
            let msg = format!("Error creando cliente en BDP: {e}");
            let resultado = if matches!(
                e,
                crate::services::bdp_weblink::BdpWeblinkError::Http(_)
                    | crate::services::bdp_weblink::BdpWeblinkError::Api { .. }
            ) {
                "ambiguo"
            } else {
                "error"
            };
            crate::services::BdpBackupService::actualizar_resultado(
                pool,
                audit_id,
                resultado,
                None,
                Some(&msg),
            )
            .await
            .map_err(|audit_error| {
                AppError::Internal(format!(
                    "{msg}; además falló el cierre de auditoría: {audit_error}"
                ))
            })?;
            return Err(AppError::Internal(msg));
        }
    };

    /* Verificar ErrorMessage de BDP */
    let error_msg = resp
        .get("ErrorMessage")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if !error_msg.is_empty() {
        crate::services::BdpBackupService::actualizar_resultado(
            pool,
            audit_id,
            "error",
            Some(&resp),
            Some(error_msg),
        )
        .await
        .map_err(|audit_error| {
            AppError::Internal(format!(
                "BDP rechazó el cliente y falló el cierre de auditoría: {audit_error}"
            ))
        })?;
        /* Guardar error en el cliente */
        let _ = ClienteRepository::update_bdp_sync(
            pool,
            cliente.id,
            None,
            false,
            Some(error_msg),
        )
        .await;
        return Err(AppError::Validation(format!(
            "BDP devolvió error: {error_msg}"
        )));
    }

    confirmar_cliente_creado(pool, cliente.id, audit_id, bdp_code, &resp).await?;

    Ok(Json(serde_json::json!({
        "cliente_id": cliente.id,
        "bdp_customer_code": bdp_code,
        "bdp_synced": true,
        "bdp_synced_at": chrono::Utc::now().to_rfc3339(),
    })))
}

/* [AUDIT-N1] Marca local + auditoría en una sola transacción: si el proceso
 * muere después del HTTP, no queda `bdp_synced = true` sin su auditoría (ni al
 * revés). [267A-7] El SQL vive en los repositorios; aquí quedan mensajes y
 * puerta. Se extrajo de `ejecutar_create_customer` para no pasar de 100 líneas
 * efectivas ([189A-6], regla `funcion-larga-rs`). */
async fn confirmar_cliente_creado(
    pool: &sqlx::PgPool,
    cliente_id: Uuid,
    audit_id: Uuid,
    bdp_code: i32,
    resp: &serde_json::Value,
) -> Result<(), AppError> {
    let commit_result = async {
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| format!("Error iniciando tx post-create_customer: {e}"))?;

        ClienteRepository::update_bdp_sync_confirmado_tx(&mut tx, cliente_id, bdp_code)
            .await
            .map_err(|e| {
                format!(
                    "BDP confirmó el cliente {bdp_code}, pero no se pudo persistir el vínculo local: {e}"
                )
            })?;

        BdpAuditLogRepository::marcar_exito_tx(&mut tx, audit_id, Some(resp))
            .await
            .map_err(|e| format!("Cliente confirmado, pero falló el cierre de auditoría: {e}"))?;

        tx.commit()
            .await
            .map_err(|e| format!("Error confirmando tx post-create_customer: {e}"))
    }
    .await;

    if let Err(e) = commit_result {
        /* La tx falló pero BDP ya creó el cliente → auditoría ambigua. */
        if let Err(error_auditoria) = crate::services::BdpBackupService::actualizar_resultado(
            pool,
            audit_id,
            "ambiguo",
            Some(resp),
            Some(&e),
        )
        .await
        {
            warn!("[BDP-AUDITORIA] no se pudo cerrar la auditoría {audit_id}: {error_auditoria}");
        }
        return Err(AppError::Internal(e));
    }
    Ok(())
}

/// Heurística para dividir un nombre completo BDP en nombre + apellidos.
/// BDP envía "APELLIDOS NOMBRE" o "NOMBRE APELLIDOS" — asumimos que la
/// primera palabra es el nombre si el string tiene pocas palabras,
/// o que las últimas 2+ palabras son apellidos.
pub(super) fn split_name(full_name: &str) -> (&str, &str) {
    let parts: Vec<&str> = full_name.split_whitespace().collect();
    match parts.len() {
        0 => ("Sin nombre", ""),
        1 => (parts[0], ""),
        2 => (parts[0], parts[1]),
        _ => {
            /* 3+ palabras: primera = nombre, resto = apellidos */
            let nombre = parts[0];
            let apellidos_start = full_name.find(parts[1]).unwrap_or(0);
            (nombre, &full_name[apellidos_start..])
        }
    }
}
