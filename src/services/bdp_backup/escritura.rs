// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
use sqlx::PgPool;
use uuid::Uuid;

use super::tipos::NuevoSnapshot;
use super::BdpBackupService;
use crate::models::ConfiguracionRestaurante;
use crate::services::bdp_weblink::{response_error_message, BdpWeblinkClient};
use crate::services::bdp_weblink_catalog::{BdpGetOrderRequest, BdpOrderIdentifier};

// =========================================================================
// PRE-WRITE AUDIT LOG — selectivo, máximo 1 llamada BDP
// =========================================================================

impl BdpBackupService {
    /// Prepara la evidencia previa a una escritura. Para pago y factura el
    /// estado remoto de la comanda es obligatorio; si no se puede capturar,
    /// la autorización permanece intacta y no se crea una intención de envío.
    pub async fn preparar_snapshot_escritura(
        pool: &PgPool,
        user_id: Uuid,
        operacion: &str,
        config: &ConfiguracionRestaurante,
        bdp_order_id: Option<i64>,
    ) -> Result<Option<Uuid>, String> {
        let auto_backup = Self::get_auto_backup(pool, user_id).await?;
        if !auto_backup {
            return Err("Escritura BDP bloqueada: auto-backup pre-write desactivado".to_string());
        }

        if !matches!(operacion, "add_payment" | "invoice") {
            return Ok(None);
        }

        let order_id = bdp_order_id
            .ok_or_else(|| format!("Snapshot pre-write {operacion} bloqueado: falta order_id"))?;
        let snapshot = Self::snapshot_order_state(config, order_id).await?;
        let snap = Self::insert_snapshot(
            pool,
            user_id,
            NuevoSnapshot {
                tipo: "pre_write_order",
                direccion: "bdp",
                trigger_tipo: "pre_write",
                datos: snapshot,
                metadata: Some(serde_json::json!({"operacion": operacion, "order_id": order_id})),
                target_base_url: Some(Self::canonical_target(config)?),
                connection_fingerprint: Some(Self::connection_fingerprint(config)?),
                expires_at: None,
                notas: None,
            },
        )
        .await?;
        Ok(Some(snap.id))
    }

    /* Lee el estado de la comanda antes de escribir. Un error de BDP o una
     * respuesta sin Order válido bloquean la escritura: no hay evidencia. */
    async fn snapshot_order_state(
        config: &ConfiguracionRestaurante,
        order_id: i64,
    ) -> Result<serde_json::Value, String> {
        let client = BdpWeblinkClient::new(config);
        let order_data = client
            .get_order(&BdpGetOrderRequest {
                order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            })
            .await
            .map_err(|e| format!("Error obteniendo estado de comanda {order_id}: {e}"))?;
        if let Some(error) = response_error_message(&order_data) {
            return Err(format!(
                "Snapshot pre-write bloqueado: GetOrder {order_id} devolvió {error}"
            ));
        }
        if !order_data
            .get("Order")
            .is_some_and(serde_json::Value::is_object)
        {
            return Err(format!(
                "Snapshot pre-write bloqueado: GetOrder {order_id} no devolvió una orden válida"
            ));
        }
        Ok(order_data)
    }
}

/// [267A-9] Parámetros de `auditar_escritura_directa` agrupados en struct
/// (patrón `Ctx*` de F1): la función recibía 8 argumentos y clippy
/// (`too_many_arguments`, 7 máx) no cerraba desde HEAD. Mismos campos,
/// mismo SQL y mismos mensajes; solo cambia la forma de pasarlos.
pub struct AuditoriaDirecta<'a> {
    pub pool: &'a PgPool,
    pub user_id: Uuid,
    pub operacion: &'a str,
    pub target_entity_type: &'a str,
    pub target_entity_id: Uuid,
    pub datos_enviados: &'a serde_json::Value,
    pub resultado: &'a str,
    pub error_mensaje: Option<&'a str>,
}

impl BdpBackupService {
    /// Registra una operación de escritura BDP directa (sin cadena de
    /// arming/authorize) en `bdp_audit_log`, con su resultado final.
    /// [049A-1/H-W-1] Se usa para `call_waiter`: notificación sin estado ni
    /// riesgo de duplicación, que por diseño no consume armado ni snapshot,
    /// pero exige trazabilidad en el ledger como el resto de escrituras.
    pub async fn auditar_escritura_directa(params: AuditoriaDirecta<'_>) -> Result<(), String> {
        sqlx::query(
            r"INSERT INTO bdp_audit_log
               (user_id, operacion, direccion, datos_enviados, resultado,
                target_entity_type, target_entity_id, error_mensaje,
                authorization_reason)
               VALUES ($1, $2, 'glory_to_bdp', $3, $4, $5, $6, $7,
                       'directa sin arming: operacion sin estado ni riesgo de duplicacion')",
        )
        .bind(params.user_id)
        .bind(params.operacion)
        .bind(params.datos_enviados)
        .bind(params.resultado)
        .bind(params.target_entity_type)
        .bind(params.target_entity_id)
        .bind(params.error_mensaje)
        .execute(params.pool)
        .await
        .map_err(|e| format!("Error registrando auditoría BDP directa: {e}"))?;
        Ok(())
    }
}
