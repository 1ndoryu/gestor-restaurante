/* [Fase 7.1+7.2] Handlers para sync bidireccional de clientes Glory ↔ BDP.
 * POST /api/bdp/customers/import     — Importar clientes desde BDP a Glory (ExportCustomers)
 * POST /api/clientes/:id/bdp-sync    — Push de un cliente Glory a BDP (CreateCustomer)
 *
 * Patrón: sigue el mismo flujo que bdp_article_map::importar_catalogo.
 * Los campos BDP del cliente se mapean así:
 *   BDP.Customer      → clientes.bdp_customer_code
 *   BDP.FiscalName    → clientes.nombre + apellidos
 *   BDP.MobilePhone   → clientes.telefono
 *   BDP.EMail         → clientes.email
 *
 * Gotchas:
 *   - ExportCustomers puede devolver muchos registros (~43k) → import batch con progreso.
 *   - Matching Glory↔BDP: por teléfono (primario) o email (secundario).
 *   - CreateCustomer requiere `code` (entero) → se asigna automático o se reutiliza bdp_customer_code. */

use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::CrearClienteRequest;
use crate::repositories::ClienteRepository;
use crate::services::{
    BdpCreateCustomerRequest, BdpExportCustomersRequest, BdpWeblinkClient,
    ClienteService, ConfiguracionService,
};
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/bdp/customers/import",
            post(importar_clientes_bdp),
        )
        .route(
            "/clientes/:id/bdp-sync",
            post(sincronizar_cliente_bdp),
        )
}

/* [Fase 7.1] Importar clientes desde BDP a Glory.
 * Llama a ExportCustomers, matchea por teléfono/email con clientes existentes,
 * y crea nuevos clientes en Glory si no existen. */
#[utoipa::path(
    post,
    path = "/api/bdp/customers/import",
    tag = "BDP Clientes",
    responses(
        (status = 200, description = "Importación completada", body = serde_json::Value),
        (status = 400, description = "BDP no configurado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
#[allow(clippy::too_many_lines)]
pub async fn importar_clientes_bdp(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation(
            "BDP no está configurado. Configura URL y credenciales primero.".into(),
        ));
    }

    let client = BdpWeblinkClient::new(&config);

    let _session = client
        .login()
        .await
        .map_err(|e| AppError::Internal(format!("Error login BDP: {e}")))?;

    let customers_json = client
        .export_customers(&BdpExportCustomersRequest::default())
        .await
        .map_err(|e| AppError::Internal(format!("Error exportando clientes BDP: {e}")))?;

    /* Parsear array de clientes — BDP devuelve {"Customers": [...]} */
    let customers = customers_json
        .get("Customers")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            AppError::Internal("Respuesta BDP no contiene array 'Customers'.".into())
        })?;

    let error_msg = customers_json
        .get("ErrorMessage")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if !error_msg.is_empty() {
        return Err(AppError::Internal(format!(
            "BDP devolvió error: {error_msg}"
        )));
    }

    let mut importados: u32 = 0;
    let mut actualizados: u32 = 0;
    let mut sin_cambios: u32 = 0;
    let mut errores: u32 = 0;

    for cust in customers {
        #[allow(clippy::cast_possible_truncation)]
        let bdp_code = cust.get("Customer").and_then(serde_json::Value::as_i64).unwrap_or(0) as i32;
        let fiscal_name = cust
            .get("FiscalName")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let commercial_name = cust
            .get("CommercialName")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let mobile_phone = cust
            .get("MobilePhone")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let email = cust.get("EMail").and_then(|v| v.as_str()).unwrap_or("");
        let address = cust.get("Address").and_then(|v| v.as_str()).unwrap_or("");

        if bdp_code == 0 || fiscal_name.is_empty() {
            errores += 1;
            continue;
        }

        /* Buscar cliente existente por teléfono o email */
        let existing = ClienteRepository::find_by_telefono_o_email(
            &state.pool,
            auth.user_id,
            mobile_phone,
            email,
        )
        .await
        .map_err(|e| AppError::Internal(format!("Error buscando cliente: {e}")))?;

        if let Some(cliente) = existing {
            /* Cliente ya existe → actualizar bdp_customer_code si no lo tiene */
            if cliente.bdp_customer_code.is_none() {
                ClienteRepository::update_bdp_sync(
                    &state.pool,
                    cliente.id,
                    Some(bdp_code),
                    true,
                    None,
                )
                .await
                .map_err(|e| {
                    AppError::Internal(format!("Error actualizando sync BDP: {e}"))
                })?;
                actualizados += 1;
            } else {
                sin_cambios += 1;
            }
        } else {
            /* Cliente no existe en Glory → crearlo */
            /* Dividir fiscal_name en nombre + apellidos (heurística: primera palabra = nombre) */
            let (nombre, apellidos) = split_name(fiscal_name);

            let nuevo = CrearClienteRequest {
                nombre: nombre.to_string(),
                apellidos: Some(apellidos.to_string()),
                telefono: Some(mobile_phone.to_string()),
                prefijo_telefono: None,
                email: Some(email.to_string()),
                empresa: Some(commercial_name.to_string()),
                notas: Some(format!("Importado de BDP (código {bdp_code}). {address}")),
                foto_url: None,
                consentimiento_comercial_email: None,
                consentimiento_comercial_sms: None,
                enviar_encuestas: None,
                alergias: None,
                preferencias_bebida: None,
                preferencias_ubicacion: None,
            };

            /* Crear cliente vía servicio (usa ClienteService::create que hace find/create) */
            match ClienteService::create(&state.pool, auth.user_id, nuevo).await {
                Ok(cliente) => {
                    /* Actualizar bdp_customer_code */
                    let _ = ClienteRepository::update_bdp_sync(
                        &state.pool,
                        cliente.id,
                        Some(bdp_code),
                        true,
                        None,
                    )
                    .await;
                    importados += 1;
                }
                Err(_) => {
                    errores += 1;
                }
            }
        }
    }

    Ok(Json(serde_json::json!({
        "imported": importados,
        "updated": actualizados,
        "unchanged": sin_cambios,
        "errors": errores,
        "total": customers.len(),
    })))
}

/* [Fase 7.2] Push de un cliente Glory a BDP (CreateCustomer).
 * Si el cliente ya tiene bdp_customer_code, usa Overwrite=true para actualizar.
 * Si no, asigna un código BDP basado en el teléfono o uno aleatorio alto. */
#[utoipa::path(
    post,
    path = "/api/clientes/{id}/bdp-sync",
    tag = "BDP Clientes",
    params(("id" = Uuid, Path, description = "ID del cliente")),
    responses(
        (status = 200, description = "Cliente sincronizado con BDP", body = serde_json::Value),
        (status = 400, description = "BDP no configurado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 404, description = "Cliente no encontrado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn sincronizar_cliente_bdp(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation(
            "BDP no está configurado. Configura URL y credenciales primero.".into(),
        ));
    }

    let cliente = ClienteRepository::find_by_id(&state.pool, id, auth.user_id)
        .await
        .map_err(|e| AppError::Internal(format!("Error buscando cliente: {e}")))?
        .ok_or_else(|| AppError::NotFound("Cliente no encontrado".into()))?;

    /* Determinar código BDP: reutilizar existente o generar uno nuevo */
    let bdp_code = cliente.bdp_customer_code.unwrap_or_else(|| {
        /* Generar código BDP alto (900000+) para evitar colisiones con clientes existentes */
        900_000 + (cliente.id.as_u128() % 99_999) as i32
    });

    let client = BdpWeblinkClient::new(&config);
    let _session = client
        .login()
        .await
        .map_err(|e| AppError::Internal(format!("Error login BDP: {e}")))?;

    /* Construir nombre completo: apellidos + nombre */
    let fiscal_name = if cliente.apellidos.is_empty() {
        cliente.nombre.clone()
    } else {
        format!("{} {}", cliente.apellidos, cliente.nombre)
    };

    let req = BdpCreateCustomerRequest {
        code: bdp_code,
        fiscal_name,
        commercial_name: cliente.nombre.clone(),
        mobile_phone: cliente.telefono.clone(),
        email: cliente.email.clone(),
        overwrite: cliente.bdp_customer_code.is_some(), /* Si ya existe en BDP, sobrescribir */
    };

    let resp = client
        .create_customer(&req)
        .await
        .map_err(|e| AppError::Internal(format!("Error creando cliente en BDP: {e}")))?;

    /* Verificar ErrorMessage de BDP */
    let error_msg = resp
        .get("ErrorMessage")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if !error_msg.is_empty() {
        /* Guardar error en el cliente */
        let _ = ClienteRepository::update_bdp_sync(
            &state.pool,
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

    /* Actualizar bdp_customer_code y bdp_synced */
    ClienteRepository::update_bdp_sync(&state.pool, cliente.id, Some(bdp_code), true, None)
        .await
        .map_err(|e| AppError::Internal(format!("Error actualizando sync BDP: {e}")))?;

    Ok(Json(serde_json::json!({
        "cliente_id": cliente.id,
        "bdp_customer_code": bdp_code,
        "bdp_synced": true,
        "bdp_synced_at": chrono::Utc::now().to_rfc3339(),
    })))
}

/// Heurística para dividir un nombre completo BDP en nombre + apellidos.
/// BDP envía "APELLIDOS NOMBRE" o "NOMBRE APELLIDOS" — asumimos que la
/// primera palabra es el nombre si el string tiene pocas palabras,
/// o que las últimas 2+ palabras son apellidos.
fn split_name(full_name: &str) -> (&str, &str) {
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
