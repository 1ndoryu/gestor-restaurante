// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin macros ni DB en compile-time: query! rompe el build.
/* [267A-9/F4.1] Flujo de sync_venta: guardias, armado, HTTP y cierre de auditoría y venta.
 * Sin cambios de mensajes, SQL, guardas ni orden. */

use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::Mutex as TokioMutex;
use tokio::time::{timeout, Duration};
use tracing::{info, warn};
use uuid::Uuid;

use super::OrderSendFailure;
use crate::models::{ConfiguracionRestaurante, Venta};
use crate::repositories::VentaRepository;
use crate::services::bdp_sync::{BdpSyncService, SyncLockGuard, SYNC_LOCKS};
use crate::services::{ModoEfectivo, ServicioModoOperacion};

impl BdpSyncService {
    /// Orquesta el flujo completo Glory → BDP para una venta.
    /// `idempotency_key` se guarda en el audit log si se proporciona (C1).
    pub async fn sync_venta(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
        is_update: bool,
        idempotency_key: Option<&str>,
    ) {
        if !Self::pasar_guardias_sync_venta(pool, venta, config, is_update).await {
            return;
        }

        let Some(audit_id) =
            Self::armar_orden(pool, venta, config, is_update, idempotency_key).await
        else {
            return;
        };

        /* [R5] Timeout global de 45s para la fase HTTP a BDP. */
        let http_result = timeout(
            Duration::from_secs(45),
            Self::run_http_phase(pool, venta, config),
        )
        .await;
        Self::cerrar_sync_venta(pool, venta, audit_id, http_result).await;
    }

    /* [267A-8] Guardias de entrada de sync_venta: modo, read_only, backup,
     * locks, anulada, ya-sincronizada y advisory lock distribuido.
     * Devuelve `true` si puede continuar; en cada salida temprana limpia
     * el lock local para no bloquear reintentos. */
    async fn pasar_guardias_sync_venta(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
        is_update: bool,
    ) -> bool {
        /* [128A-1/F1-1] M1: el switch maestro modo_operacion también gatea los
         * caminos de escritura; 'standalone' nunca llama a BDP aunque
         * bdp_sync_enabled siga activo. */
        if ServicioModoOperacion::modo_efectivo_desde_config(config) != ModoEfectivo::Bdp {
            return false;
        }

        /* [F3] Gate: en modo read_only, no enviar ventas a BDP */
        if !crate::services::BdpWriteGuard::modo_permite_escritura(&config.bdp_sync_mode) {
            info!(
                "[F3] BDP en modo read_only — sync_venta omitida para venta {}",
                venta.id
            );
            return false;
        }
        if !config.bdp_auto_backup_before_write {
            warn!(
                "[F2] Escritura BDP bloqueada para venta {}: auto-backup desactivado",
                venta.id
            );
            return false;
        }

        let lock = {
            let mut map = SYNC_LOCKS
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            map.entry(venta.id)
                .or_insert_with(|| Arc::new(TokioMutex::new(())))
                .clone()
        };
        let Ok(_guard) = lock.try_lock() else {
            info!(
                "[065A-5] BDP sync ya en progreso para venta {}, saltando",
                venta.id
            );
            return false;
        };
        let _sync_lock_guard = SyncLockGuard::new(venta.id);

        /* [128A-1/F4/M8] Venta anulada localmente: no se sincroniza ni se
         * reintenta hacia BDP (la anulación es local; C3=b no llama CancelOrder). */
        if venta.anulada {
            info!(
                "[128A-1/F4] Venta {} anulada localmente; sync BDP omitida",
                venta.id
            );
            Self::cleanup_lock(venta.id);
            return false;
        }

        if !Self::revisar_estado_y_lock_distribuido(pool, venta, is_update).await {
            return false;
        }

        if let Err(error) = crate::services::BdpWriteGuard::ensure_no_unresolved(
            pool,
            venta.user_id,
            "venta_id",
            venta.id,
            &["create_order", "update_order"],
        )
        .await
        {
            warn!("[BDP-SAFE] {error}");
            Self::cleanup_lock(venta.id);
            return false;
        }
        true
    }

    /* [F4.4] Revisa estado ya-sincronizada (solo create) y adquiere el
     * advisory lock distribuido. Semántica idéntica al bloque original:
     * cada salida temprana limpia el lock local; el error de lectura no
     * bloquea (continúa hacia el lock distribuido). */
    async fn revisar_estado_y_lock_distribuido(
        pool: &PgPool,
        venta: &Venta,
        is_update: bool,
    ) -> bool {
        /* Guard: si ya sincronizada y es create (no update), saltar */
        if !is_update {
            match VentaRepository::find_by_id(pool, venta.id, venta.user_id).await {
                Ok(Some(fresh)) if fresh.bdp_synced => {
                    info!(
                        "[065A-5] Venta {} ya sincronizada con BDP, saltando",
                        venta.id
                    );
                    Self::cleanup_lock(venta.id);
                    return false;
                }
                Ok(None) => {
                    warn!("[065A-5] Venta {} no encontrada en BD", venta.id);
                    Self::cleanup_lock(venta.id);
                    return false;
                }
                Err(e) => {
                    warn!("[065A-5] Error leyendo venta {} para guard: {e}", venta.id);
                }
                _ => {}
            }
        }

        /* Lock distribuido: el mutex anterior solo protege este proceso. La
         * transacción mantiene un advisory lock durante toda la operación para
         * impedir que otra instancia envíe la misma venta simultáneamente. */
        let mut distributed_lock = match pool.begin().await {
            Ok(transaction) => transaction,
            Err(error) => {
                warn!(
                    "[BDP-SAFE] No se pudo iniciar lock distribuido para venta {}: {error}",
                    venta.id
                );
                Self::cleanup_lock(venta.id);
                return false;
            }
        };
        let acquired = sqlx::query_scalar::<_, bool>(
            "SELECT pg_try_advisory_xact_lock(hashtextextended($1, 0))",
        )
        .bind(format!("bdp-order:{}", venta.id))
        .fetch_one(&mut *distributed_lock)
        .await
        .unwrap_or(false);
        if !acquired {
            info!(
                "[BDP-SAFE] Otra instancia procesa la venta {}; escritura omitida",
                venta.id
            );
            Self::cleanup_lock(venta.id);
            return false;
        }

        /* [R2] Liberar la conexión de Postgres inmediatamente: el lock
         * transaccional ya cumplió su función de evitar que otra instancia
         * adquiera el mismo advisory lock en este instante. Mantener la tx
         * abierta durante las llamadas HTTP a BDP retendría una conexión del
         * pool innecesariamente y podría agotarlo bajo carga. La exclusión
         * dentro de este proceso sigue garantizada por SYNC_LOCKS; el lock
         * distribuido solo actúa como cortina de humo inicial. */
        if let Err(error) = distributed_lock.commit().await {
            warn!(
                "[BDP-SAFE] No se pudo liberar el lock distribuido para venta {}: {error}",
                venta.id
            );
            Self::cleanup_lock(venta.id);
            return false;
        }
        true
    }

    /* [267A-8] Arming de la orden: snapshot pre-write + authorize [187A-1].
     * Devuelve el audit_id autorizado, o `None` si la preparación falló
     * (ya deja estado y limpia el lock). */
    async fn armar_orden(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
        is_update: bool,
        idempotency_key: Option<&str>,
    ) -> Option<Uuid> {
        /* [187A-1] Preparación fail-closed: la autorización, la intención y el
         * retorno a solo lectura se confirman atómicamente antes del HTTP. */
        let operacion = "create_order";
        let datos_enviados = serde_json::json!({
            "venta_id": venta.id,
            "importe_base": venta.importe_base,
            "is_update": is_update
        });
        let snapshot_pre_id = match crate::services::BdpBackupService::preparar_snapshot_escritura(
            pool,
            venta.user_id,
            operacion,
            config,
            None,
        )
        .await
        {
            Ok(id) => id,
            Err(e) => {
                let msg = format!("Pre-write audit BDP falló; escritura bloqueada: {e}");
                warn!("[F2] {msg} (venta {})", venta.id);
                let _ = VentaRepository::update_bdp_status(pool, venta.id, false, Some(&msg), None)
                    .await;
                Self::cleanup_lock(venta.id);
                return None;
            }
        };

        let audit_id = match crate::services::BdpWriteGuard::authorize(
            pool,
            venta.user_id,
            config,
            crate::services::EscrituraAutorizar {
                scope: "create_order",
                target_entity_type: "venta",
                target_entity_id: venta.id,
                entity_json_field: "venta_id",
                datos_enviados: &datos_enviados,
                snapshot_pre_id,
                idempotency_key,
            },
        )
        .await
        {
            Ok(id) => id,
            Err(error) => {
                warn!("[BDP-SAFE] {error}");
                Self::cleanup_lock(venta.id);
                return None;
            }
        };
        Some(audit_id)
    }

    /* [267A-8] Cierre de sync_venta: despacha el resultado de la fase HTTP
     * (éxito, rechazo, ambiguo o timeout) a sus anotadores. */
    async fn cerrar_sync_venta(
        pool: &PgPool,
        venta: &Venta,
        audit_id: Uuid,
        http_result: Result<Result<i64, OrderSendFailure>, tokio::time::error::Elapsed>,
    ) {
        match http_result {
            Ok(Ok(order_id)) => {
                Self::confirmar_orden(pool, venta, audit_id, order_id).await;
            }
            Ok(Err(OrderSendFailure::Rejected(msg))) => {
                let safe_msg = Self::sanitize_error(&msg);
                Self::anotar_fracaso(pool, audit_id, "error", None, &msg, "fallida", venta.id)
                    .await;
                warn!(
                    "[BDP-SAFE] Escritura BDP error para venta {}: {safe_msg}; no se reintenta a ciegas",
                    venta.id
                );
                Self::marcar_error_venta(pool, venta, &safe_msg).await;
            }
            Ok(Err(OrderSendFailure::Ambiguous(msg))) => {
                Self::anotar_fracaso(pool, audit_id, "ambiguo", None, &msg, "ambigua", venta.id)
                    .await;
                warn!(
                    "[BDP-SAFE] Escritura BDP ambigua para venta {}: {msg}; no se reintenta a ciegas",
                    venta.id
                );
                Self::marcar_error_venta(pool, venta, &msg).await;
            }
            Err(_) => {
                let msg = "Timeout esperando respuesta de BDP (45s)".to_string();
                warn!("[BDP-SAFE] {msg} (venta {})", venta.id);
                Self::anotar_fracaso(pool, audit_id, "ambiguo", None, &msg, "ambigua", venta.id)
                    .await;
                Self::marcar_error_venta(pool, venta, &msg).await;
            }
        }
    }

    /* [267A-8] Confirma orden BDP: marca local + auditoría en tx atómica
     * [AUDIT-2.11]; si la tx falla, la auditoría queda ambigua porque BDP
     * ya creó la comanda. */
    async fn confirmar_orden(pool: &PgPool, venta: &Venta, audit_id: Uuid, order_id: i64) {
        info!(
            "[065A-5] Venta {} sincronizada con BDP → OrderId={order_id}",
            venta.id
        );
        let respuesta = serde_json::json!({"order_id": order_id});
        let commit_result = async {
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| format!("Error iniciando tx post-create_order: {e}"))?;

            sqlx::query(
                "UPDATE ventas SET bdp_synced = true, bdp_synced_at = NOW(), bdp_order_id = $2, bdp_sync_error = NULL WHERE id = $1",
            )
            .bind(venta.id)
            .bind(order_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!(
                "BDP confirmó OrderId={order_id}, pero no se pudo persistir localmente: {e}"
            ))?;

            sqlx::query(
                r"UPDATE bdp_audit_log
                SET resultado = 'exito', datos_respuesta = $2, error_mensaje = NULL, updated_at = NOW()
                WHERE id = $1",
            )
            .bind(audit_id)
            .bind(Some(&respuesta))
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Venta confirmada, pero falló el cierre de auditoría: {e}"))?;

            /* [D04] Mapeo Glory↔BDP por OrderId: la reconciliación posterior puede
             * encontrar la comanda por su id, no solo por la huella. Upsert para
             * que un reintento confirmado no duplique la fila. */
            sqlx::query(
                r"INSERT INTO bdp_id_map (id, user_id, entidad, glory_id, bdp_id, market_id, marketplace_order_id)
                VALUES ($1, $2, 'venta', $3, $4, $5, $6)
                ON CONFLICT (user_id, entidad, glory_id)
                DO UPDATE SET bdp_id = EXCLUDED.bdp_id, market_id = EXCLUDED.market_id,
                              marketplace_order_id = EXCLUDED.marketplace_order_id,
                              actualizado_en = NOW()",
            )
            .bind(Uuid::new_v4())
            .bind(venta.user_id)
            .bind(venta.id.to_string())
            .bind(order_id.to_string())
            .bind(crate::services::bdp_sync::BDP_SYNC_MARKET_ID.to_string())
            .bind(Self::marketplace_order_id(venta.id))
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Venta confirmada, pero falló el mapeo bdp_id_map: {e}"))?;

            tx.commit()
                .await
                .map_err(|e| format!("Error confirmando tx post-create_order: {e}"))
        }
        .await;

        if let Err(e) = commit_result {
            warn!("[BDP-SAFE] {e} (venta {})", venta.id);
            let _ = crate::services::BdpBackupService::actualizar_resultado(
                pool,
                audit_id,
                "ambiguo",
                Some(&respuesta),
                Some(&e),
            )
            .await;
        }
    }

    /* [267A-8] Cierra la auditoría con resultado no-éxito. `contexto` distingue
     * el mensaje de aviso ("fallida" / "ambigua"). */
    async fn anotar_fracaso(
        pool: &PgPool,
        audit_id: Uuid,
        resultado: &str,
        respuesta: Option<&serde_json::Value>,
        msg: &str,
        contexto: &str,
        venta_id: Uuid,
    ) {
        if let Err(error) = crate::services::BdpBackupService::actualizar_resultado(
            pool,
            audit_id,
            resultado,
            respuesta,
            Some(msg),
        )
        .await
        {
            warn!("[BDP-SAFE] No se pudo cerrar auditoría {contexto} de venta {venta_id}: {error}");
        }
    }

    /* [267A-8] Persiste el error BDP en la venta local. */
    async fn marcar_error_venta(pool: &PgPool, venta: &Venta, msg_venta: &str) {
        if let Err(e) =
            VentaRepository::update_bdp_status(pool, venta.id, false, Some(msg_venta), None).await
        {
            warn!(
                "[065A-5] Error guardando error BDP de venta {}: {e}",
                venta.id
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rust_decimal::Decimal;

    /// Venta mínima para llamar a `confirmar_orden` sin tocar BDP.
    fn venta_de_prueba() -> Venta {
        let importe_base = Decimal::new(2500, 2);
        Venta {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            fecha: Utc::now().date_naive(),
            comensales: Some(2),
            descripcion: "Test venta BDP".to_string(),
            iva_porcentaje: Decimal::new(10, 0),
            turno: "mediodia".into(),
            canal: "comedor".into(),
            metodo_pago: "efectivo".into(),
            importe_base,
            importe_iva: Decimal::new(250, 2),
            reserva_id: None,
            cliente_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            haddock_synced: false,
            haddock_synced_at: None,
            haddock_sync_error: None,
            bdp_synced: false,
            bdp_synced_at: None,
            bdp_sync_error: None,
            bdp_order_id: None,
            bdp_order_status: None,
            bdp_invoiced: false,
            anulada: false,
            anulada_at: None,
            anulacion_motivo: None,
            anulacion_usuario: None,
            facturada_local: false,
            factura_numero: None,
            factura_fecha: None,
            propina: Decimal::ZERO,
        }
    }

    /// [D04] Tras confirmar, `bdp_id_map` guarda el OrderId de BDP. Un reintento
    /// confirmado actualiza la fila existente y no crea una segunda.
    #[sqlx::test(migrations = "./migrations")]
    async fn confirmar_orden_mapea_order_id_sin_duplicar(pool: PgPool) {
        let venta = venta_de_prueba();
        let audit_id = Uuid::new_v4();
        BdpSyncService::confirmar_orden(&pool, &venta, audit_id, 4242).await;
        BdpSyncService::confirmar_orden(&pool, &venta, audit_id, 4343).await;

        let (filas, bdp_id): (i64, Option<String>) = sqlx::query_as(
            "SELECT COUNT(*), MAX(bdp_id) FROM bdp_id_map
              WHERE user_id = $1 AND entidad = 'venta' AND glory_id = $2",
        )
        .bind(venta.user_id)
        .bind(venta.id.to_string())
        .fetch_one(&pool)
        .await
        .expect("consulta de bdp_id_map");

        assert_eq!(filas, 1);
        assert_eq!(bdp_id.as_deref(), Some("4343"));
    }
}
