/* [BKP-004] Handlers para exploración, backup y auditoría BDP.
 * Endpoints de solo lectura para inventariar BDP y gestionar snapshots.
 * Ningún endpoint de este módulo modifica datos en BDP (excepto restaurar Glory). */

use axum::{extract::Path, extract::State, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::UserRole;
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::BdpDepartmentsExportFromProfileRequest;
use crate::services::ConfiguracionService;
use crate::services::{BdpAuditEntry, BdpBackupService, BdpSnapshot, RestoreResult};
use crate::services::{BdpExploracionResultado, BdpExplorerService};

use super::bdp_guard::exigir_modo_bdp; /* [039A-1/H-P1-03] Guard compartido (N1). */
use super::rate_limit::{limitador_escrituras, limitar};

pub fn routes() -> Router<crate::handlers::AppState> {
    Router::new()
        .route("/bdp/explorar", get(explorar_bdp))
        /* [W-Q2.1/049A-1 F3] Lectura de solo lectura de los departamentos reales
         * del perfil BDP (ExportFromProfile). No modifica nada en BDP; sirve
         * para conocer los DeptCode válidos antes de un alta de artículo. */
        .route("/bdp/departamentos/perfil", get(departamentos_del_perfil))
        /* [D24-m] Formas de pago reales del BDP: sin esta lista el mapeo
         * método de pago → tender se rellena a ciegas y un id inventado solo
         * se descubre al cobrar. */
        .route("/bdp/tenders", get(formas_pago_bdp))
        .route(
            "/bdp/backup/completo",
            axum::routing::post(snapshot_completo),
        )
        .route("/bdp/backup/parcial", axum::routing::post(snapshot_parcial))
        .route("/bdp/backup/glory", axum::routing::post(snapshot_glory))
        .route("/bdp/backup/snapshots", get(listar_snapshots))
        .route(
            "/bdp/backup/snapshots/:id",
            get(obtener_snapshot).delete(eliminar_snapshot),
        )
        .route(
            "/bdp/backup/restaurar/:id",
            axum::routing::post(restaurar_glory),
        )
        .route("/bdp/audit", get(listar_audit))
        /* Tope por IP de las escrituras de este módulo (ver rate_limit.rs). */
        .layer(axum::middleware::from_fn_with_state(
            limitador_escrituras(),
            limitar,
        ))
}

/// Explora BDP de forma segura (solo lectura).
/// Devuelve un inventario de lo que hay en BDP sin modificar nada.
#[utoipa::path(
    get,
    path = "/api/bdp/explorar",
    tag = "BDP Backup",
    responses(
        (status = 200, description = "Exploración completada", body = BdpExploracionResultado),
        (status = 400, description = "BDP no configurado"),
        (status = 500, description = "Error interno"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn explorar_bdp(
    State(state): State<crate::handlers::AppState>,
    auth: AuthUser,
) -> Result<Json<BdpExploracionResultado>, AppError> {
    /* [039A-1/H-P1-03] Explorar BDP hace red real: en modo independiente se
     * rechaza sin tocar red (N1), aunque haya credenciales configuradas. */
    exigir_modo_bdp(&state, auth.user_id).await?;
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation(
            "BDP no está configurado. Configura URL y credenciales primero.".into(),
        ));
    }

    let resultado = BdpExplorerService::explorar_bdp_completo(&config).await;
    Ok(Json(resultado))
}

/// Lee los departamentos reales del perfil BDP configurado (solo lectura).
/// [W-Q2.1/049A-1 F3] Expone `ExportDepartmentsFromProfile` para conocer los
/// `DeptCode`/nombres válidos antes de un alta de artículo. No modifica BDP.
#[utoipa::path(
    get,
    path = "/api/bdp/departamentos/perfil",
    tag = "BDP Backup",
    responses(
        (status = 200, description = "Departamentos del perfil (respuesta cruda del BDP)", body = serde_json::Value),
        (status = 400, description = "BDP no configurado"),
        (status = 500, description = "Error interno"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn departamentos_del_perfil(
    State(state): State<crate::handlers::AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    exigir_modo_bdp(&state, auth.user_id).await?;
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation("BDP no está configurado.".into()));
    }

    let client = BdpWeblinkClient::new(&config);
    let valor = client
        .export_departments_from_profile(&BdpDepartmentsExportFromProfileRequest {
            profile_id: config.bdp_items_profile_id,
        })
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;
    Ok(Json(valor))
}

/* [D24-m] Forma de pago tal como la devuelve `Tenders/GetList`. */
#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct FormaPagoBdp {
    pub id: i64,
    pub nombre: String,
    /* `Type` de BDP (1 efectivo, 0 resto). Se conserva tal cual: el reparto
     * exacto de tipos lo fija BDP y no queremos inventarlo aquí. */
    pub tipo: Option<i64>,
}

/* La forma cambió entre versiones del manual: se aceptan `TenderList` y
 * `Tenders`, y los nombres en inglés o español, sin fallar si falta `Type`. */
fn formas_pago_desde(valor: &Value) -> Result<Vec<FormaPagoBdp>, String> {
    if let Some(mensaje) = valor
        .get("ErrorMessage")
        .and_then(Value::as_str)
        .filter(|m| !m.trim().is_empty())
    {
        return Err(format!("BDP respondió: {mensaje}"));
    }
    let items = ["TenderList", "Tenders"]
        .iter()
        .find_map(|clave| valor.get(*clave).and_then(Value::as_array))
        .ok_or_else(|| "La respuesta del BDP no trae la lista de formas de pago".to_string())?;
    Ok(items
        .iter()
        .filter_map(|item| {
            let id = item
                .get("Id")
                .or_else(|| item.get("TenderId"))
                .and_then(Value::as_i64)?;
            let nombre = ["Name", "Nombre", "Description"]
                .iter()
                .find_map(|clave| item.get(*clave).and_then(Value::as_str))
                .unwrap_or("")
                .trim()
                .to_string();
            let tipo = item.get("Type").and_then(Value::as_i64);
            Some(FormaPagoBdp { id, nombre, tipo })
        })
        .collect())
}

/// Lee las formas de pago dadas de alta en BDP (solo lectura).
/// [D24-m] Sirve para mapear método de pago local → `TenderId` sin adivinar.
#[utoipa::path(
    get,
    path = "/api/bdp/tenders",
    tag = "BDP Backup",
    responses(
        (status = 200, description = "Formas de pago del BDP", body = Vec<FormaPagoBdp>),
        (status = 400, description = "BDP no configurado"),
        (status = 500, description = "Error interno"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn formas_pago_bdp(
    State(state): State<crate::handlers::AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<FormaPagoBdp>>, AppError> {
    exigir_modo_bdp(&state, auth.user_id).await?;
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation("BDP no está configurado.".into()));
    }

    let client = BdpWeblinkClient::new(&config);
    let valor = client
        .get_tenders()
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let formas = formas_pago_desde(&valor).map_err(AppError::Internal)?;
    Ok(Json(formas))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SnapshotParcialRequest {
    pub tipos: Vec<String>,
    pub notas: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub(crate) struct SnapshotGloryRequest {
    tipos: Vec<String>,
    notas: Option<String>,
}

/// Snapshot completo de BDP.
/// Lee todos los endpoints de lectura y guarda el resultado.
/// ⚠️ Requiere autorización — hace llamadas a la API de BDP.
#[utoipa::path(
    post,
    path = "/api/bdp/backup/completo",
    tag = "BDP Backup",
    request_body = Option<String>,
    responses(
        (status = 200, description = "Snapshot creado", body = BdpSnapshot),
        (status = 400, description = "BDP no configurado"),
        (status = 500, description = "Error interno"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn snapshot_completo(
    State(state): State<crate::handlers::AppState>,
    auth: AuthUser,
    Json(notas): Json<Option<String>>,
) -> Result<Json<BdpSnapshot>, AppError> {
    /* [149A-3/H-06] Descarga de BDP + escritura en el historial de respaldos: solo propietario. */
    auth.require_role(&[UserRole::Admin])?;
    /* [039A-1/H-P1-03] El snapshot "completo" descarga de BDP (red real):
     * en modo independiente se rechaza sin tocar red (N1). */
    exigir_modo_bdp(&state, auth.user_id).await?;
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation("BDP no está configurado.".into()));
    }

    let snapshot =
        BdpBackupService::snapshot_bdp_completo(&state.pool, auth.user_id, &config, notas)
            .await
            .map_err(AppError::Internal)?;

    Ok(Json(snapshot))
}

/// Snapshot parcial de BDP (solo los tipos seleccionados).
/// Tipos válidos: articulos, clientes, departamentos, salones, empleados
#[utoipa::path(
    post,
    path = "/api/bdp/backup/parcial",
    tag = "BDP Backup",
    request_body = SnapshotParcialRequest,
    responses(
        (status = 200, description = "Snapshot creado", body = BdpSnapshot),
        (status = 400, description = "BDP no configurado o tipos inválidos"),
        (status = 500, description = "Error interno"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn snapshot_parcial(
    State(state): State<crate::handlers::AppState>,
    auth: AuthUser,
    Json(req): Json<SnapshotParcialRequest>,
) -> Result<Json<BdpSnapshot>, AppError> {
    /* [149A-3/H-06] Respaldo de seguridad pre-escritura: solo propietario. */
    auth.require_role(&[UserRole::Admin])?;
    if req.tipos.is_empty() {
        return Err(AppError::Validation(
            "Debes seleccionar al menos un tipo de dato.".into(),
        ));
    }

    /* [039A-1/H-P1-03] El snapshot parcial descarga de BDP (red real):
     * en modo independiente se rechaza sin tocar red (N1). */
    exigir_modo_bdp(&state, auth.user_id).await?;
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation("BDP no está configurado.".into()));
    }

    let snapshot = BdpBackupService::snapshot_bdp_parcial(
        &state.pool,
        auth.user_id,
        &config,
        &req.tipos,
        req.notas,
    )
    .await
    .map_err(AppError::Internal)?;

    Ok(Json(snapshot))
}

/// Snapshot de datos locales de Glory (0 llamadas a BDP).
/// Tipos válidos: ventas, clientes, mapeos
#[utoipa::path(
    post,
    path = "/api/bdp/backup/glory",
    tag = "BDP Backup",
    request_body = SnapshotGloryRequest,
    responses(
        (status = 200, description = "Snapshot creado", body = BdpSnapshot),
        (status = 500, description = "Error interno"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn snapshot_glory(
    State(state): State<crate::handlers::AppState>,
    auth: AuthUser,
    Json(req): Json<SnapshotGloryRequest>,
) -> Result<Json<BdpSnapshot>, AppError> {
    if req.tipos.is_empty() {
        return Err(AppError::Validation(
            "Debes seleccionar al menos un tipo de dato.".into(),
        ));
    }

    let snapshot =
        BdpBackupService::snapshot_glory(&state.pool, auth.user_id, &req.tipos, req.notas)
            .await
            .map_err(AppError::Internal)?;

    Ok(Json(snapshot))
}

/// Lista snapshots del usuario (historial).
#[utoipa::path(
    get,
    path = "/api/bdp/backup/snapshots",
    tag = "BDP Backup",
    params(
        ("limit" = Option<i64>, Query, description = "Máximo de snapshots a devolver (default 50)")
    ),
    responses(
        (status = 200, description = "Lista de snapshots", body = Vec<BdpSnapshot>),
        (status = 500, description = "Error interno"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_snapshots(
    State(state): State<crate::handlers::AppState>,
    auth: AuthUser,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<BdpSnapshot>>, AppError> {
    let limit = params
        .get("limit")
        .and_then(|l| l.parse::<i64>().ok())
        .unwrap_or(50);

    let snapshots = BdpBackupService::listar_snapshots(&state.pool, auth.user_id, limit)
        .await
        .map_err(AppError::Internal)?;

    Ok(Json(snapshots))
}

/// Obtiene un snapshot por ID.
#[utoipa::path(
    get,
    path = "/api/bdp/backup/snapshots/{id}",
    tag = "BDP Backup",
    params(("id" = Uuid, Path, description = "ID del snapshot")),
    responses(
        (status = 200, description = "Snapshot encontrado", body = BdpSnapshot),
        (status = 404, description = "Snapshot no encontrado"),
        (status = 500, description = "Error interno"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn obtener_snapshot(
    State(state): State<crate::handlers::AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<BdpSnapshot>, AppError> {
    let snapshot = BdpBackupService::obtener_snapshot(&state.pool, id)
        .await
        .map_err(AppError::Internal)?
        .ok_or_else(|| AppError::NotFound("Snapshot no encontrado".into()))?;

    if snapshot.user_id != auth.user_id {
        return Err(AppError::NotFound("Snapshot no encontrado".into()));
    }

    Ok(Json(snapshot))
}

/// Elimina un snapshot.
#[utoipa::path(
    delete,
    path = "/api/bdp/backup/snapshots/{id}",
    tag = "BDP Backup",
    params(("id" = Uuid, Path, description = "ID del snapshot")),
    responses(
        (status = 200, description = "Snapshot eliminado"),
        (status = 404, description = "Snapshot no encontrado"),
        (status = 500, description = "Error interno"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn eliminar_snapshot(
    State(state): State<crate::handlers::AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    /* [149A-3/H-06] Borrar respaldos es destructivo: solo propietario. */
    auth.require_role(&[UserRole::Admin])?;
    let eliminado = BdpBackupService::eliminar_snapshot(&state.pool, id, auth.user_id)
        .await
        .map_err(AppError::Internal)?;

    if !eliminado {
        return Err(AppError::NotFound("Snapshot no encontrado".into()));
    }

    Ok(Json(serde_json::json!({ "eliminado": true })))
}

/// Restaura datos locales de Glory desde un snapshot.
/// ⚠️ Solo restaura datos locales — BDP no permite delete/update via API.
#[utoipa::path(
    post,
    path = "/api/bdp/backup/restaurar/{id}",
    tag = "BDP Backup",
    params(("id" = Uuid, Path, description = "ID del snapshot a restaurar")),
    request_body = RestoreGloryRequest,
    responses(
        (status = 200, description = "Restauración completada", body = RestoreResult),
        (status = 400, description = "Confirmación inválida o snapshot no es de tipo Glory"),
        (status = 404, description = "Snapshot no encontrado"),
        (status = 500, description = "Error interno"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn restaurar_glory(
    State(state): State<crate::handlers::AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RestoreGloryRequest>,
) -> Result<Json<RestoreResult>, AppError> {
    /* [149A-3/H-06] Restaurar SOBREESCRIBE datos locales: operación más destructiva del sistema,
     * solo propietario. */
    auth.require_role(&[UserRole::Admin])?;
    /* [AUDIT-11.3] Confirmación textual explícita antes de restaurar. */
    let expected = format!("RESTAURAR {id}");
    if req.confirmacion.trim() != expected {
        return Err(AppError::Validation(format!(
            "Confirmación inválida. Escriba exactamente: {expected}"
        )));
    }
    let result = BdpBackupService::restaurar_glory(&state.pool, id, auth.user_id)
        .await
        .map_err(|e| {
            if e.contains("no encontrado") || e.contains("No autorizado") {
                AppError::NotFound(e)
            } else {
                AppError::Validation(e)
            }
        })?;

    Ok(Json(result))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub(crate) struct RestoreGloryRequest {
    /// Debe ser exactamente "RESTAURAR {uuid}".
    pub confirmacion: String,
}

/// Lista entradas del audit log.
#[utoipa::path(
    get,
    path = "/api/bdp/audit",
    tag = "BDP Backup",
    params(
        ("limit" = Option<i64>, Query, description = "Máximo de entradas a devolver (default 100)")
    ),
    responses(
        (status = 200, description = "Audit log", body = Vec<BdpAuditEntry>),
        (status = 500, description = "Error interno"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_audit(
    State(state): State<crate::handlers::AppState>,
    auth: AuthUser,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<BdpAuditEntry>>, AppError> {
    let limit = params
        .get("limit")
        .and_then(|l| l.parse::<i64>().ok())
        .unwrap_or(100);

    let entries = BdpBackupService::listar_audit(&state.pool, auth.user_id, limit)
        .await
        .map_err(AppError::Internal)?;

    Ok(Json(entries))
}

#[cfg(test)]
mod tests_formas_pago {
    use super::{formas_pago_desde, FormaPagoBdp};
    use serde_json::json;

    #[test]
    fn lee_tender_list_del_manual() {
        let valor = json!({
            "ErrorMessage": "",
            "TenderList": [
                {"Id": 1, "Name": "Efectivo", "Type": 1},
                {"Id": 4, "Name": "A 30 - 60 - 90", "Type": 0}
            ]
        });
        let formas = formas_pago_desde(&valor).expect("debe leer");
        assert_eq!(
            formas,
            vec![
                FormaPagoBdp {
                    id: 1,
                    nombre: "Efectivo".into(),
                    tipo: Some(1)
                },
                FormaPagoBdp {
                    id: 4,
                    nombre: "A 30 - 60 - 90".into(),
                    tipo: Some(0)
                },
            ]
        );
    }

    #[test]
    fn acepta_la_variante_tenders_y_nombre_en_espanol() {
        let valor = json!({"Tenders": [{"TenderId": 2, "Nombre": "Tarjeta"}]});
        let formas = formas_pago_desde(&valor).expect("debe leer");
        assert_eq!(
            formas,
            vec![FormaPagoBdp {
                id: 2,
                nombre: "Tarjeta".into(),
                tipo: None
            }]
        );
    }

    #[test]
    fn un_error_remoto_no_se_disfraza_de_lista_vacia() {
        let valor = json!({"ErrorMessage": "[201701]-EL ALMACÉN DE SALIDA 0 NO EXISTE"});
        let error = formas_pago_desde(&valor).expect_err("debe fallar");
        assert!(error.contains("201701"), "{error}");
    }

    #[test]
    fn una_respuesta_sin_lista_es_un_error_explicito() {
        let valor = json!({"ErrorMessage": ""});
        let error = formas_pago_desde(&valor).expect_err("debe fallar");
        assert!(error.contains("no trae la lista"), "{error}");
    }
}
