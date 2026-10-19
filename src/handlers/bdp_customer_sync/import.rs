/* [Fase 7.1] Importación de clientes desde BDP a Glory (ExportCustomers). */

use axum::extract::State;
use axum::Json;
use tracing::warn;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{Cliente, CrearClienteRequest};
use crate::repositories::ClienteRepository;
use crate::services::{
    BdpExportCustomersRequest, BdpWeblinkClient, ClienteService, ConfiguracionService,
};
use crate::AppState;

use super::super::bdp_guard::exigir_modo_bdp; /* [039A-1] Guard compartido (N1). */
use super::crear_customer::split_name;
use super::sincronizar::sincronizar_existente;
use super::{customer_code, BdpCustomerImportRequest};

/** Circuit breaker: máximo de errores consecutivos antes de abortar batch. */
const MAX_CONSECUTIVE_ERRORS: u32 = 10;

/* [Fase 7.1] Importar clientes desde BDP a Glory.
 * Llama a ExportCustomers, matchea por teléfono/email con clientes existentes,
 * y crea nuevos clientes en Glory si no existen. */
#[utoipa::path(
    post,
    path = "/api/bdp/customers/import",
    tag = "BDP Clientes",
    request_body = BdpCustomerImportRequest,
    responses(
        (status = 200, description = "Importación completada", body = serde_json::Value),
        (status = 400, description = "BDP no configurado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn importar_clientes_bdp(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<BdpCustomerImportRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    exigir_modo_bdp(&state, auth.user_id).await?;
    if req.aplicar && req.confirmacion.as_deref() != Some("IMPORTAR CLIENTES BDP") {
        return Err(AppError::Validation(
            "Aplicación bloqueada: escriba exactamente IMPORTAR CLIENTES BDP. No se modificó Glory ni BDP."
                .into(),
        ));
    }
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
        .ok_or_else(|| AppError::Internal("Respuesta BDP no contiene array 'Customers'.".into()))?;

    let error_msg = customers_json
        .get("ErrorMessage")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if !error_msg.is_empty() {
        return Err(AppError::Internal(format!(
            "BDP devolvió error: {error_msg}"
        )));
    }

    let mut rec = RecuentoImportacion::default();
    let mut codigos_vistos = std::collections::HashSet::new();
    for cust in customers {
        let cortar = procesar_cliente_bdp(
            &state,
            auth.user_id,
            cust,
            req.aplicar,
            &mut codigos_vistos,
            &mut rec,
        )
        .await;
        if cortar {
            break;
        }
    }

    Ok(Json(serde_json::json!({
        "imported": rec.importados,
        "updated": rec.actualizados,
        "unchanged": rec.sin_cambios,
        "errors": rec.errores,
        "conflicts": rec.conflictos,
        "conflict_sample": rec.muestra_conflictos,
        "total": customers.len(),
        "applied": req.aplicar,
        "writes_to_bdp": false,
    })))
}

/// Contadores de la importación y racha de errores seguidos.
#[derive(Default)]
pub(super) struct RecuentoImportacion {
    importados: u32,
    pub(super) actualizados: u32,
    pub(super) sin_cambios: u32,
    errores: u32,
    conflictos: u32,
    errores_seguidos: u32,
    muestra_conflictos: Vec<serde_json::Value>,
}

impl RecuentoImportacion {
    /// Una fila válida reinicia la racha de errores.
    fn ok(&mut self) {
        self.errores_seguidos = 0;
    }

    /// Cuenta un error. Devuelve `true` si hay que cortar: demasiados seguidos.
    pub(super) fn error(&mut self) -> bool {
        self.errores += 1;
        self.errores_seguidos += 1;
        self.errores_seguidos >= MAX_CONSECUTIVE_ERRORS
    }

    /// Guarda una muestra acotada de conflictos para la respuesta.
    pub(super) fn conflicto(&mut self, cliente: &Cliente, codigo_bdp: i32) {
        self.conflictos += 1;
        if self.muestra_conflictos.len() < 20 {
            self.muestra_conflictos.push(serde_json::json!({
                "cliente_id": cliente.id,
                "codigo_local": cliente.bdp_customer_code,
                "codigo_bdp": codigo_bdp,
                "motivo": "identidad coincide pero ya está vinculada a otro código"
            }));
        }
    }
}

/// Campos de un cliente BDP leídos del JSON, sin validar.
struct FilaBdp<'a> {
    codigo: i32,
    nombre_fiscal: &'a str,
    nombre_comercial: &'a str,
    telefono: &'a str,
    email: &'a str,
    direccion: &'a str,
}

impl<'a> FilaBdp<'a> {
    fn desde(cust: &'a serde_json::Value) -> Self {
        Self {
            codigo: customer_code(cust).unwrap_or(0),
            nombre_fiscal: texto_bdp(cust, "FiscalName"),
            nombre_comercial: texto_bdp(cust, "CommercialName"),
            telefono: texto_bdp(cust, "MobilePhone"),
            email: texto_bdp(cust, "EMail"),
            direccion: texto_bdp(cust, "Address"),
        }
    }

    /// Requisitos mínimos para importar: código, nombre fiscal y algún contacto.
    fn valida(&self) -> bool {
        self.codigo != 0
            && !self.nombre_fiscal.trim().is_empty()
            && !(self.telefono.trim().is_empty() && self.email.trim().is_empty())
    }
}

/// Lee un campo de texto del cliente BDP; vacío si falta.
fn texto_bdp<'a>(cust: &'a serde_json::Value, clave: &str) -> &'a str {
    cust.get(clave).and_then(|v| v.as_str()).unwrap_or("")
}

/// Procesa una fila de BDP. Devuelve `true` si hay que cortar la importación.
async fn procesar_cliente_bdp(
    state: &AppState,
    user_id: Uuid,
    cust: &serde_json::Value,
    aplicar: bool,
    codigos_vistos: &mut std::collections::HashSet<i32>,
    rec: &mut RecuentoImportacion,
) -> bool {
    let fila = FilaBdp::desde(cust);
    /* Un código repetido en la misma exportación no se procesa dos veces */
    if !fila.valida() || !codigos_vistos.insert(fila.codigo) {
        return rec.error();
    }
    rec.ok();

    /* Buscar cliente existente por teléfono o email */
    match ClienteRepository::find_by_telefono_o_email(
        &state.pool,
        user_id,
        fila.telefono,
        fila.email,
    )
    .await
    {
        Ok(Some(cliente)) => {
            sincronizar_existente(state, &cliente, fila.codigo, aplicar, rec).await
        }
        Ok(None) => crear_importado(state, user_id, &fila, aplicar, rec).await,
        Err(e) => {
            warn!("[AUDIT-N5] Error DB buscando cliente: {e}");
            rec.error()
        }
    }
}

/// Cliente inexistente en Glory: lo crea y lo vincula al aplicar.
async fn crear_importado(
    state: &AppState,
    user_id: Uuid,
    fila: &FilaBdp<'_>,
    aplicar: bool,
    rec: &mut RecuentoImportacion,
) -> bool {
    if !aplicar {
        rec.importados += 1;
        return false;
    }
    /* Dividir fiscal_name en nombre + apellidos (heurística: primera palabra = nombre) */
    let (nombre, apellidos) = split_name(fila.nombre_fiscal);
    let nuevo = CrearClienteRequest {
        nombre: nombre.to_string(),
        apellidos: Some(apellidos.to_string()),
        telefono: Some(fila.telefono.to_string()),
        prefijo_telefono: None,
        email: Some(fila.email.to_string()),
        empresa: Some(fila.nombre_comercial.to_string()),
        notas: Some(format!(
            "Importado de BDP (código {}). {}",
            fila.codigo, fila.direccion
        )),
        foto_url: None,
        consentimiento_comercial_email: None,
        consentimiento_comercial_sms: None,
        enviar_encuestas: None,
        alergias: None,
        preferencias_bebida: None,
        preferencias_ubicacion: None,
    };
    /* Crear localmente y vincular. El índice único por usuario y
     * código BDP evita que una carrera duplique la identidad remota. */
    match ClienteService::create_bdp_import(&state.pool, user_id, nuevo, fila.codigo).await {
        Ok(_) => {
            rec.importados += 1;
            false
        }
        Err(e) => {
            warn!("[AUDIT-N5] Error creando cliente importado: {e}");
            rec.error()
        }
    }
}
