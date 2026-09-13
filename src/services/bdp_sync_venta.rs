// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin macros ni DB en compile-time: query! rompe el build.
/* [267A-9/F4.1] FASE 7: sync_venta y helpers, movidos tal cual desde bdp_sync.rs.
 * Sin cambios de mensajes, SQL, guardas ni orden. BdpSyncService sigue definido en el hub. */

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use rust_decimal::prelude::Decimal;
use serde_json::{json, Value};
use sqlx::PgPool;
use tokio::sync::Mutex as TokioMutex;
use tokio::time::{timeout, Duration};
use tracing::{info, warn};
use uuid::Uuid;

use super::bdp_sync::{
    BdpSyncService, OrderContext, ResolvedArticle, SyncLockGuard, BDP_SYNC_MARKET_ID, SYNC_LOCKS,
};
use crate::models::{ConfiguracionRestaurante, Venta, VentaLinea};
use crate::repositories::{ClienteRepository, VentaLineaRepository, VentaRepository};
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{
    BdpCreateOrderRequest, BdpGetArticleRequest, BdpGetOrderRequest, BdpGetPosArticlesRequest,
    BdpOrderIdentifier,
};
use crate::services::{ModoEfectivo, ServicioModoOperacion};

/// Errores clasificados para decidir si reintentar.
pub(crate) enum BdpSyncError {
    /// Rechazo conocido: no se aplicó una operación válida y no se reintenta.
    Rejected(String),
    /// Timeout, HTTP anómalo o JSON inválido: BDP pudo haber aplicado la orden.
    AmbiguousTransport(String),
}

pub(crate) enum OrderSendFailure {
    Rejected(String),
    Ambiguous(String),
}

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
        if config.bdp_sync_mode != "unidirectional" {
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
            "create_order",
            "venta",
            venta.id,
            "venta_id",
            &datos_enviados,
            snapshot_pre_id,
            idempotency_key,
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

    /// Envía una sola vez. Ante un fallo de transporte intenta reconciliar por
    /// `MarketplaceOrderId`; nunca repite `CreateOrder` a ciegas.
    async fn retry_send_order(
        client: &BdpWeblinkClient<'_>,
        config: &ConfiguracionRestaurante,
        venta: &Venta,
        article: &ResolvedArticle,
        lineas: Option<&[VentaLinea]>,
        line_article_ids: Option<&[i64]>,
        order_ctx: &OrderContext,
    ) -> Result<i64, OrderSendFailure> {
        match Self::send_order(
            client,
            config,
            venta,
            article,
            lineas,
            line_article_ids,
            order_ctx,
        )
        .await
        {
            Ok(order_id) => Ok(order_id),
            Err(BdpSyncError::Rejected(msg)) => Err(OrderSendFailure::Rejected(msg)),
            Err(BdpSyncError::AmbiguousTransport(msg)) => {
                let marketplace_id = Self::marketplace_order_id(venta.id);
                let request = BdpGetOrderRequest {
                    order_identifier: BdpOrderIdentifier::by_market(
                        BDP_SYNC_MARKET_ID,
                        marketplace_id.clone(),
                    ),
                };
                match client.get_order(&request).await {
                    Ok(response) => {
                        let order_id = response
                            .get("OrderId")
                            .and_then(Value::as_i64)
                            .or_else(|| response.get("Order")?.get("OrderId")?.as_i64());
                        order_id.filter(|id| *id > 0).ok_or_else(|| {
                            OrderSendFailure::Ambiguous(format!(
                                "{msg}; reconciliación sin OrderId para {marketplace_id}"
                            ))
                        })
                    }
                    Err(error) => Err(OrderSendFailure::Ambiguous(format!(
                        "{msg}; reconciliación falló para {marketplace_id}: {error}"
                    ))),
                }
            }
        }
    }

    /// Construye y envía una comanda a BDP para la venta dada.
    async fn send_order(
        client: &BdpWeblinkClient<'_>,
        config: &ConfiguracionRestaurante,
        venta: &Venta,
        article: &ResolvedArticle,
        lineas: Option<&[VentaLinea]>,
        line_article_ids: Option<&[i64]>,
        order_ctx: &OrderContext,
    ) -> Result<i64, BdpSyncError> {
        if let Some(lineas) = lineas {
            /* [028A-BDP-WRITE] Mantener la validación en la frontera que
             * precede al HTTP, incluso si aparece otro caller de send_order. */
            Self::validate_order_lines(lineas).map_err(|failure| match failure {
                OrderSendFailure::Rejected(message) => BdpSyncError::Rejected(message),
                OrderSendFailure::Ambiguous(message) => BdpSyncError::AmbiguousTransport(message),
            })?;
        }
        let order = Self::build_order(config, venta, article, lineas, line_article_ids, order_ctx);
        let response = client
            .create_order(&order)
            .await
            .map_err(|error| match error {
                crate::services::bdp_weblink::BdpWeblinkError::Remote(message) => {
                    BdpSyncError::Rejected(format!("BDP: {message}"))
                }
                crate::services::bdp_weblink::BdpWeblinkError::NotConfigured => {
                    BdpSyncError::Rejected("BDP no está configurado".to_string())
                }
                crate::services::bdp_weblink::BdpWeblinkError::InvalidBaseUrl(url) => {
                    BdpSyncError::Rejected(format!("URL BDP inválida: {url}"))
                }
                crate::services::bdp_weblink::BdpWeblinkError::WriteTargetDenied(url) => {
                    BdpSyncError::Rejected(format!("destino de escritura BDP no autorizado: {url}"))
                }
                crate::services::bdp_weblink::BdpWeblinkError::Http(message) => {
                    BdpSyncError::AmbiguousTransport(format!("error de transporte BDP: {message}"))
                }
                crate::services::bdp_weblink::BdpWeblinkError::Api { status, body } => {
                    BdpSyncError::AmbiguousTransport(format!("BDP respondió HTTP {status}: {body}"))
                }
                crate::services::bdp_weblink::BdpWeblinkError::Throttled(message) => {
                    /* [R3] Throttling es un rechazo temporal del TPV local; si lo
                     * tratamos como permanente perdemos comandas. Lo marcamos
                     * ambiguo para que el operador/operación de reconciliación
                     * lo vuelva a intentar. */
                    BdpSyncError::AmbiguousTransport(format!("BDP throttled: {message}"))
                }
            })?;

        /* Extraer OrderId y ErrorMessage de la respuesta */
        let order_id = response.get("OrderId").and_then(Value::as_i64).unwrap_or(0);
        let error_msg = response
            .get("ErrorMessage")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();

        if error_msg.is_empty() && order_id > 0 {
            Ok(order_id)
        } else if error_msg.is_empty() {
            Err(BdpSyncError::Rejected(
                "BDP devolvió OrderId=0 sin error".to_string(),
            ))
        } else {
            Err(BdpSyncError::Rejected(format!("BDP: {error_msg}")))
        }
    }

    #[must_use]
    pub fn marketplace_order_id(venta_id: Uuid) -> String {
        let venta_hex = venta_id.simple().to_string();
        format!("G{}", &venta_hex[..14])
    }

    /// Construye el payload BDP `CreateOrder` desde una venta Glory.
    /// [F2.7] Si hay líneas, genera un pedido multi-item. Si no, usa fallback legacy (1 artículo).
    /// `line_article_ids`: paralelo a `lineas`, con el ID BDP de cada artículo resuelto.
    /// [F3.1] `order_ctx`: tender, order type y customer resueltos.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::too_many_lines
    )]
    pub(crate) fn build_order(
        config: &ConfiguracionRestaurante,
        venta: &Venta,
        article: &ResolvedArticle,
        lineas: Option<&[VentaLinea]>,
        line_article_ids: Option<&[i64]>,
        order_ctx: &OrderContext,
    ) -> BdpCreateOrderRequest {
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        /* MarketplaceOrderId: estable por venta y max 15 chars.
         * Debe mantenerse idéntico entre reintentos para que BDP pueda deduplicar
         * una escritura que sí se aplicó pero cuya respuesta se perdió. */
        let marketplace_order_id = Self::marketplace_order_id(venta.id);

        /* [048A-10] El Total del Order debe coincidir con la suma de los Item.Total
         * (bruto por línea), que es el "teórico" que BDP valida — error 300033 cuando
         * no coincide. El IVA se calcula desde VatPct de cada item, no se suma aquí.
         * Evidencia: dry-run real (2026-06-01) pasó validación de totales con
         * Item.Total=Price y Order.Total=Price (sin IVA). En fallback legacy, el item
         * usa base+iva como precio y el total coincide con ese mismo valor. */
        let total: f64 = match lineas {
            Some(ls) => ls
                .iter()
                .map(|linea| {
                    Self::decimal_to_f64(&linea.precio_unitario)
                        * Self::decimal_to_f64(&linea.cantidad)
                        - Self::decimal_to_f64(&linea.descuento)
                })
                .sum(),
            None => {
                Self::decimal_to_f64(&venta.importe_base) + Self::decimal_to_f64(&venta.importe_iva)
            }
        };

        /* [F2.7] Construir Items array — multi-item si hay líneas, fallback si no */
        let items: Vec<Value> = if let Some(lineas) = lineas {
            lineas
                .iter()
                .enumerate()
                .map(|(i, linea)| {
                    let precio = Self::decimal_to_f64(&linea.precio_unitario);
                    let cantidad = Self::decimal_to_f64(&linea.cantidad);
                    let descuento = Self::decimal_to_f64(&linea.descuento);
                    /* [AUDIT-2.3] Validar que precio y cantidad sean positivos. */
                    if precio < 0.0 || cantidad <= 0.0 {
                        warn!(
                            "[BDP-SAFE] Línea '{}' tiene precio={} o cantidad={} inválidos; se envía a BDP tal cual",
                            linea.descripcion, precio, cantidad
                        );
                    }
                    let linea_total = (precio * cantidad) - descuento;
                    /* [F2.8] Usar artículo BDP mapeado por línea, o fallback al default */
                    let line_article_id = line_article_ids
                        .and_then(|ids| ids.get(i))
                        .copied()
                        .unwrap_or(article.id);
                    json!({
                        "Lin": (i + 1) as i32,
                        "Id": line_article_id,
                        "Name": linea.descripcion,
                        "Units": cantidad,
                        "Price": precio,
                        "Supplement": 0.0,
                        "Discount": descuento,
                        "DiscountPct": false,
                        "Total": linea_total,
                        "VatPct": Self::decimal_to_f64(&linea.iva_pct),
                        "Comments": [],
                        "Supplements": [],
                        "OrderItemType": 0,
                        "OrderItemTypeMetaInfo": "",
                        "TyC_D1": 0,
                        "TyC_D2": 0,
                        "TyC_D3": 0,
                        "OnSale": false
                    })
                })
                .collect()
        } else {
            /* Fallback legacy: 1 artículo genérico con el total de la venta */
            let description = if venta.descripcion.is_empty() {
                article.name.clone()
            } else {
                venta.descripcion.clone()
            };
            vec![json!({
                "Lin": 1,
                "Id": article.id,
                "Name": description,
                "Units": 1.0,
                "Price": total,
                "Supplement": 0.0,
                "Discount": 0.0,
                "DiscountPct": false,
                "Total": total,
                "VatPct": Self::decimal_to_f64(&venta.iva_porcentaje),
                "Comments": [],
                "Supplements": [],
                "OrderItemType": 0,
                "OrderItemTypeMetaInfo": "",
                "TyC_D1": 0,
                "TyC_D2": 0,
                "TyC_D3": 0,
                "OnSale": false
            })]
        };

        BdpCreateOrderRequest {
            employee_id: config.bdp_employee_id,
            items_profile_id: config.bdp_items_profile_id,
            order_end_type: 1, /* Pendiente de validación — no factura, no imprime ticket */
            order_operation_type: 0, /* Escritura real (0=CheckAndCreate) */
            invoice: Some(false),
            order: {
                /* [F3.1-3.3] Construir order JSON con campos opcionales */
                let mut order = json!({
                    "MarketplaceOrderId": &marketplace_order_id[..15.min(marketplace_order_id.len())],
                    "MarketId": BDP_SYNC_MARKET_ID,
                    "MarketName": "Glory",
                    "PreparationTime": now,
                    "OrderId": 0,
                    "PosId": config.bdp_pos_id,
                    "Type": order_ctx.order_type,
                    "RoomNumber": 0,
                    "TableNumber": 0,
                    "Items": items,
                    "Discount": 0.0,
                    "DiscountPct": false,
                    "Total": total,
                    "ExecutionTime": now,
                    "Status": 0,
                    "AlreadyInvoiced": false,
                    "Comments": format!("Glory venta {}", venta.id)
                });
                /* [F3.2] TenderId — mapeo de método de pago */
                if let Some(tender_id) = order_ctx.tender_id {
                    order["TenderId"] = json!(tender_id);
                }
                /* [F3.1] Customer — datos del cliente si existe */
                if let Some(ref name) = order_ctx.customer_name {
                    let mut customer = json!({ "Name": name });
                    if let Some(ref phone) = order_ctx.customer_phone {
                        if !phone.is_empty() {
                            customer["Phone"] = json!(phone);
                        }
                    }
                    if let Some(ref code) = order_ctx.customer_code {
                        if !code.is_empty() {
                            customer["Code"] = json!(code);
                        }
                    }
                    order["Customer"] = customer;
                }
                order
            },
        }
    }

    /// Resuelve qué artículo BDP usar. Intenta:
    /// 1. Si `bdp_default_article_code` es numérico, enriquecer con `GetArticle`
    /// 2. Si no, buscar el primer artículo del perfil
    /// 3. [128A-1/F2] Resolver desde el catálogo local (`bdp_article_map`) antes
    ///    del fallback genérico: si existe mapeo local, usar sus datos
    ///    (descripción, precio, IVA) en vez del artículo genérico con código 0.
    async fn resolve_article(
        client: &BdpWeblinkClient<'_>,
        pool: &PgPool,
        user_id: uuid::Uuid,
        config: &ConfiguracionRestaurante,
    ) -> ResolvedArticle {
        /* Intento 1: código configurado como número → intentar GetArticle para datos reales */
        if let Ok(code) = config.bdp_default_article_code.trim().parse::<i64>() {
            if code > 0 {
                /* [157A-9] F9.2: GetArticle enriquece nombre, precio e IVA desde BDP */
                match client
                    .get_article(&BdpGetArticleRequest { art_code: code })
                    .await
                {
                    Ok(value) => {
                        let article_data = value.get("ArticleData").unwrap_or(&value);
                        let name = article_data
                            .get("ArtDescription")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&config.bdp_default_article_name)
                            .to_string();
                        #[allow(clippy::cast_precision_loss)]
                        let price = article_data
                            .get("Price1")
                            .and_then(Value::as_f64)
                            .unwrap_or(0.0);
                        let vat_pct = article_data
                            .get("TAVPer")
                            .and_then(Value::as_f64)
                            .unwrap_or(10.0);
                        return ResolvedArticle {
                            id: code,
                            name,
                            price,
                            vat_pct,
                        };
                    }
                    Err(e) => {
                        warn!("[157A-9] GetArticle falló para código {code}, usando config: {e}");
                        return ResolvedArticle {
                            id: code,
                            name: config.bdp_default_article_name.clone(),
                            price: 0.0,
                            /* [R12] Usar iva_por_defecto de config en vez de 10.0 hardcodeado */
                            vat_pct: Self::decimal_to_f64(&config.iva_por_defecto),
                        };
                    }
                }
            }
        }

        /* Intento 2: buscar primer artículo del perfil */
        match client
            .get_pos_articles(&BdpGetPosArticlesRequest::first_page(
                config.bdp_items_profile_id,
                1,
            ))
            .await
        {
            Ok(value) => {
                let default_iva = Self::decimal_to_f64(&config.iva_por_defecto);
                if let Some(article) = Self::extract_first_article(&value, default_iva) {
                    return article;
                }
            }
            Err(e) => {
                warn!("[065A-5] Error buscando artículo BDP: {e}");
            }
        }

        /* Intento 3: [128A-1/F2] catálogo local. */
        if let Some(resolved) = Self::resolve_article_local(pool, user_id, config).await {
            return resolved;
        }

        /* Fallback: genérico */
        ResolvedArticle {
            id: 0,
            name: config.bdp_default_article_name.clone(),
            price: 0.0,
            /* [R12] Usar iva_por_defecto de config en vez de 10.0 hardcodeado */
            vat_pct: Self::decimal_to_f64(&config.iva_por_defecto),
        }
    }

    /// [128A-1/F2] Intento 3 de `resolve_article`: si
    /// `bdp_default_article_code` está configurado, busca el mapeo local con
    /// ese código Glory (numérico o alfanumérico) y usa sus datos
    /// (descripción, precio, IVA) en vez del fallback genérico. Respeta
    /// `activo`: un mapeo desactivado no se vende. El id BDP del artículo se
    /// toma del mapeo local cuando el código configurado no es numérico.
    pub(crate) async fn resolve_article_local(
        pool: &PgPool,
        user_id: uuid::Uuid,
        config: &ConfiguracionRestaurante,
    ) -> Option<ResolvedArticle> {
        let code = config.bdp_default_article_code.trim();
        if code.is_empty() || code.eq_ignore_ascii_case("GLORY") {
            return None;
        }
        let config_code_numeric = code.parse::<i64>().ok().filter(|id| *id > 0);
        let map = match crate::repositories::BdpArticleMapRepository::buscar_por_codigo(
            pool, user_id, code,
        )
        .await
        {
            Ok(Some(map)) => map,
            Ok(None) => return None,
            Err(e) => {
                warn!("[128A-1/F2] Error resolviendo artículo local '{code}': {e}");
                return None;
            }
        };
        if !map.activo {
            /* Mapeo local existe pero está desactivado → usar fallback
             * genérico (el artículo no se vende). */
            return None;
        }
        let name = if map.descripcion.is_empty() {
            map.articulo_bdp_nombre.clone()
        } else {
            map.descripcion.clone()
        };
        let default_iva = Self::decimal_to_f64(&config.iva_por_defecto);
        let iva_pct = if map.iva_pct > rust_decimal::Decimal::ZERO {
            Self::decimal_to_f64(&map.iva_pct)
        } else {
            default_iva
        };
        let price = Self::decimal_to_f64(&map.precio_tarifa1);
        /* [128A-1/F2] El id BDP puede venir del código configurado (numérico,
         * semántica clásica) o del código BDP del mapeo local cuando el código
         * configurado es alfanumérico. Si ninguno es numérico, no hay un
         * ArticleId BDP válido → fallback genérico. */
        let id = match config_code_numeric {
            Some(id) => id,
            None => match map.articulo_bdp_codigo.trim().parse::<i64>() {
                Ok(id) if id > 0 => id,
                _ => return None,
            },
        };
        Some(ResolvedArticle {
            id,
            name,
            price,
            vat_pct: iva_pct,
        })
    }

    /// [F2.8] Resuelve el artículo BDP para cada línea de venta consultando `bdp_article_map`.
    /// Devuelve un Vec<i64> paralelo a `lineas` con el ID del artículo BDP.
    /// Si una línea no tiene mapeo, usa `default_article_id`.
    async fn resolve_line_articles(
        pool: &PgPool,
        user_id: uuid::Uuid,
        lineas: &[VentaLinea],
        default_article_id: i64,
    ) -> Vec<i64> {
        let mut ids = Vec::with_capacity(lineas.len());
        for linea in lineas {
            let resolved = if linea.articulo_codigo.is_empty() {
                default_article_id
            } else {
                match crate::repositories::BdpArticleMapRepository::buscar_por_codigo(
                    pool,
                    user_id,
                    &linea.articulo_codigo,
                )
                .await
                {
                    Ok(Some(map)) => {
                        /* El código BDP puede ser numérico (ID directo) o texto (requeriría lookup).
                         * Por ahora solo soportamos códigos numéricos. */
                        match map.articulo_bdp_codigo.trim().parse::<i64>() {
                            Ok(code) if code > 0 => code,
                            _ => {
                                info!(
                                    "[F2.8] Código BDP '{}' no numérico para línea '{}', usando default",
                                    map.articulo_bdp_codigo, linea.descripcion
                                );
                                default_article_id
                            }
                        }
                    }
                    Ok(None) => {
                        /* Sin mapeo — usar artículo default */
                        default_article_id
                    }
                    Err(e) => {
                        warn!(
                            "[F2.8] Error buscando mapeo BDP para código '{}': {e}",
                            linea.articulo_codigo
                        );
                        default_article_id
                    }
                }
            };
            ids.push(resolved);
        }
        ids
    }

    /// [F3.1-3.3] Resuelve el contexto del pedido: tender, order type y datos del cliente.
    /// Se ejecuta una sola vez por `sync_venta` y el resultado se reutiliza en `build_order`.
    async fn resolve_order_context(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
    ) -> OrderContext {
        /* [F3.2] TenderId: mapear metodo_pago → ID BDP desde config.bdp_tender_map */
        let tender_id = Self::resolve_tender_id(venta, config);

        /* [F3.3] Order type: mapear canal → tipo BDP desde config.bdp_order_type_map */
        let order_type = Self::resolve_order_type(venta, config);

        /* [F3.1] Customer: lookup cliente si existe */
        let (customer_code, customer_name, customer_phone) =
            Self::resolve_customer(pool, venta, config).await;

        OrderContext {
            tender_id,
            order_type,
            customer_code,
            customer_name,
            customer_phone,
        }
    }

    /// [F3.2] Resuelve `TenderId` buscando `venta.metodo_pago` en `bdp_tender_map`.
    /// Ej: `{"efectivo": "1", "tarjeta": "2"}` → `metodo_pago`="Efectivo" → `tender_id=Some(1)`.
    pub(crate) fn resolve_tender_id(
        venta: &Venta,
        config: &ConfiguracionRestaurante,
    ) -> Option<i32> {
        let map = config.bdp_tender_map.as_object()?;
        let key = venta.metodo_pago.to_lowercase();
        let value = map.get(&key)?;
        let id = value
            .as_i64()
            .and_then(|id| i32::try_from(id).ok())
            .or_else(|| value.as_str()?.trim().parse::<i32>().ok())?;
        if id > 0 {
            Some(id)
        } else {
            None
        }
    }

    /// [F3.3] Resuelve el order type buscando venta.canal en `bdp_order_type_map`.
    /// Default: 0 (Barra/Takeaway) si no hay mapeo o el canal no está configurado.
    pub(crate) fn resolve_order_type(venta: &Venta, config: &ConfiguracionRestaurante) -> i32 {
        let Some(map) = config.bdp_order_type_map.as_object() else {
            return 0;
        };
        let key = venta.canal.to_lowercase();
        let Some(value) = map.get(&key) else {
            return 0;
        };
        match value
            .as_i64()
            .and_then(|value| i32::try_from(value).ok())
            .or_else(|| value.as_str().and_then(|s| s.trim().parse::<i32>().ok()))
        {
            Some(t) if t >= 0 => t,
            _ => 0,
        }
    }

    /// [F3.1] Resuelve datos del cliente si la venta tiene `cliente_id`.
    /// Devuelve (code, name, phone) — cada uno puede ser None.
    async fn resolve_customer(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let Some(cliente_id) = venta.cliente_id else {
            /* Sin cliente — usar default_customer_code si existe */
            let code = &config.bdp_default_customer_code;
            if code.is_empty() {
                return (None, None, None);
            }
            return (Some(code.clone()), None, None);
        };

        let user_id = venta.user_id;

        match ClienteRepository::find_by_id(pool, cliente_id, user_id).await {
            Ok(Some(cliente)) => {
                let name = {
                    let full = format!("{} {}", cliente.nombre, cliente.apellidos)
                        .trim()
                        .to_string();
                    if full.is_empty() {
                        None
                    } else {
                        Some(full)
                    }
                };
                let phone = if cliente.telefono.is_empty() {
                    None
                } else {
                    Some(cliente.telefono.clone())
                };
                /* [F7.5] Priorizar bdp_customer_code del cliente sobre default config.
                 * Si el cliente ya fue sincronizado con BDP, usamos su código real.
                 * Si no, fallback al código genérico de config. */
                let code = if let Some(bdp_code) = cliente.bdp_customer_code {
                    Some(bdp_code.to_string())
                } else {
                    let default = config.bdp_default_customer_code.clone();
                    if default.is_empty() {
                        None
                    } else {
                        Some(default)
                    }
                };
                (code, name, phone)
            }
            Ok(None) => {
                info!(
                    "[F3.1] Cliente {} no encontrado para venta {}",
                    cliente_id, venta.id
                );
                (None, None, None)
            }
            Err(e) => {
                warn!("[F3.1] Error buscando cliente {}: {e}", cliente_id);
                (None, None, None)
            }
        }
    }

    /* [R12] Parámetro default_iva_pct: IVA fallback del config, no 10.0 hardcodeado. */
    pub(crate) fn extract_first_article(
        value: &Value,
        default_iva_pct: f64,
    ) -> Option<ResolvedArticle> {
        let items = value
            .get("ArticlesListData")
            .or_else(|| value.get("ArticleListData"))
            .or_else(|| value.get("Articles"))
            .and_then(|v| v.as_array())?;

        let item = items.first()?;
        #[allow(clippy::cast_possible_truncation)]
        let id = item
            .get("ArtCode")
            .or_else(|| item.get("Id"))
            .or_else(|| item.get("Code"))
            .and_then(Value::as_i64)
            .or_else(|| {
                item.get("ArtCode")
                    .or_else(|| item.get("Id"))
                    .and_then(Value::as_f64)
                    .map(|f| f as i64)
            })?;

        let name = item
            .get("ArtDescription")
            .or_else(|| item.get("Description"))
            .or_else(|| item.get("Name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Artículo BDP")
            .to_string();

        #[allow(clippy::cast_precision_loss)]
        let price = item
            .get("Price1")
            .or_else(|| item.get("Price"))
            .and_then(Value::as_f64)
            .or_else(|| item.get("Price1").and_then(Value::as_i64).map(|i| i as f64))
            .unwrap_or(0.0);

        /* [R12] Usar default_iva_pct de config en vez de 10.0 hardcodeado */
        let vat_pct = item
            .get("TAVPer")
            .or_else(|| item.get("VatPct"))
            .and_then(Value::as_f64)
            .unwrap_or(default_iva_pct);

        (id > 0).then_some(ResolvedArticle {
            id,
            name,
            price,
            vat_pct,
        })
    }

    /* [R5] Fase HTTP de sync_venta: resolve artículo, contexto, líneas y envío.
     * Se extrae a una función para poder aplicar timeout global con
     * `tokio::time::timeout` sin perder claridad. */
    async fn run_http_phase(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
    ) -> Result<i64, OrderSendFailure> {
        let client = BdpWeblinkClient::new(config);
        let article = Self::resolve_article(&client, pool, venta.user_id, config).await;

        /* [F7.5] Si la política exige cliente BDP, la comanda solo continúa
         * cuando el cliente ya tiene un código local confirmado. */
        if config.bdp_auto_sync_customers {
            if let Some(cliente_id) = venta.cliente_id {
                if let Some(bdp_code) =
                    Self::ensure_cliente_bdp_synced(pool, cliente_id, venta.user_id, config).await
                {
                    info!(
                        "[F7.5] Cliente {} auto-sincronizado con BDP (code={bdp_code}) para venta {}",
                        cliente_id, venta.id
                    );
                } else {
                    let msg = if config.bdp_default_customer_code.is_empty() {
                        "Cliente sin código BDP confirmado. Asigne un código BDP al cliente o configure un cliente por defecto en Configuración → BDP.".to_string()
                    } else {
                        "Cliente sin código BDP confirmado. Se usará el cliente por defecto configurado.".to_string()
                    };
                    warn!(
                        "[BDP-SAFE] {msg} (cliente {cliente_id}, venta {})",
                        venta.id
                    );
                    return Err(OrderSendFailure::Rejected(msg));
                }
            }
        }

        /* [F3.1] Resolver contexto del pedido: tender, order type, customer. */
        let order_ctx = Self::resolve_order_context(pool, venta, config).await;

        /* [F2.6] Obtener líneas de venta para multi-item.
         * Si la venta tiene líneas en BD, se usan para construir un pedido multi-item.
         * Si no, se usa el comportamiento legacy (1 artículo genérico). */
        let lineas = match VentaLineaRepository::listar_por_venta(pool, venta.id).await {
            Ok(l) if !l.is_empty() => Some(l),
            Ok(_) => None,
            Err(e) => {
                /* [028A-BDP-WRITE] Una lectura incompleta no puede degradarse a
                 * una comanda genérica: hacerlo podría enviar un importe o una
                 * composición distinta de la venta real. El fallback legacy
                 * solo aplica cuando la consulta tuvo éxito y no hay líneas. */
                return Err(OrderSendFailure::Rejected(format!(
                    "Comanda BDP bloqueada: no se pudieron leer las líneas de la venta: {e}"
                )));
            }
        };

        /* [028A-BDP-WRITE] No enviar líneas financieramente imposibles al TPV.
         * La validación de la UI no es una frontera de seguridad: las ventas
         * pueden llegar desde jobs, imports o datos históricos. */
        if let Some(ref lineas) = lineas {
            Self::validate_order_lines(lineas)?;
        }

        /* [F2.8] Resolver artículo BDP por línea usando bdp_article_map. */
        let line_article_ids: Option<Vec<i64>> = if let Some(ref lineas) = lineas {
            Some(Self::resolve_line_articles(pool, venta.user_id, lineas, article.id).await)
        } else {
            None
        };

        Self::retry_send_order(
            &client,
            config,
            venta,
            &article,
            lineas.as_deref(),
            line_article_ids.as_deref(),
            &order_ctx,
        )
        .await
    }

    /* [028A-BDP-WRITE] Valida las líneas justo antes de construir el payload.
     * Precio cero sigue permitido para cortesías; precios negativos y cantidades
     * no positivas no representan una línea válida para una escritura BDP. */
    pub(crate) fn validate_order_lines(lineas: &[VentaLinea]) -> Result<(), OrderSendFailure> {
        for linea in lineas {
            if linea.cantidad <= Decimal::ZERO {
                return Err(OrderSendFailure::Rejected(format!(
                    "Línea BDP inválida '{}': la cantidad debe ser mayor que cero",
                    linea.descripcion
                )));
            }
            if linea.precio_unitario < Decimal::ZERO {
                return Err(OrderSendFailure::Rejected(format!(
                    "Línea BDP inválida '{}': el precio no puede ser negativo",
                    linea.descripcion
                )));
            }
            if linea.descuento < Decimal::ZERO {
                return Err(OrderSendFailure::Rejected(format!(
                    "Línea BDP inválida '{}': el descuento no puede ser negativo",
                    linea.descripcion
                )));
            }
            let bruto = linea.precio_unitario * linea.cantidad;
            if linea.descuento > bruto {
                return Err(OrderSendFailure::Rejected(format!(
                    "Línea BDP inválida '{}': el descuento supera el importe bruto",
                    linea.descripcion
                )));
            }
        }
        Ok(())
    }

    /* [028A-BDP-WRITE] Parseo estricto de importes remotos. Nunca convertir
     * null/formatos inválidos a cero: en dinero, cero es un dato válido y no
     * puede usarse como fallback de una respuesta corrupta. */
    pub(crate) fn parse_remote_money(value: &Value, field: &str) -> Result<Decimal, String> {
        let raw = match value {
            Value::Number(number) => number.to_string(),
            Value::String(text) => text.trim().to_string(),
            _ => return Err(format!("BDP devolvió {field} con formato inválido")),
        };
        let parsed = raw
            .parse::<Decimal>()
            .map_err(|_| format!("BDP devolvió {field} inválido: '{raw}'"))?;
        if parsed < Decimal::ZERO {
            return Err(format!("BDP devolvió {field} negativo"));
        }
        Ok(parsed)
    }

    /* PaymentId no tiene un tipo estable entre respuestas BDP. Un identificador
     * vacío equivale a ausencia de identidad y no debe usarse para deduplicar. */
    pub(crate) fn remote_payment_id(value: Option<&Value>) -> Option<String> {
        let value = value?;
        let id = match value {
            Value::String(text) => text.trim().to_string(),
            Value::Number(number) => number.to_string(),
            _ => return None,
        };
        (!id.is_empty()).then_some(id)
    }

    /* [028A-BDP-WRITE] Decide la contribución de una fila local a la unión
     * financiera. Cero significa que el mismo pago ya está en BDP; el importe
     * completo significa identidad ausente/desconocida; una discrepancia de
     * importe bloquea. */
    pub(crate) fn local_payment_contribution(
        remote_payment_amounts: &HashMap<String, Decimal>,
        payment_id: Option<&str>,
        amount: Decimal,
    ) -> Result<Decimal, String> {
        let Some(payment_id) = payment_id else {
            return Ok(amount);
        };
        match remote_payment_amounts.get(payment_id) {
            None => Ok(amount),
            Some(remote_amount) if remote_amount == &amount => Ok(Decimal::ZERO),
            Some(_) => Err(format!(
                "Pago bloqueado: PaymentId {payment_id} tiene importe local y remoto distinto"
            )),
        }
    }

    /* [028A-BDP-WRITE] Normaliza y agrega Payments en una función pura para
     * que la regla financiera tenga regresiones unitarias independientes de
     * PostgreSQL/HTTP. Un ID repetido con el mismo importe es una repetición
     * de respuesta; el mismo ID con importe distinto es inconsistente y bloquea. */
    pub(crate) fn parse_remote_payments(
        payments: &[Value],
    ) -> Result<(HashMap<String, Decimal>, Decimal), String> {
        let mut amounts = HashMap::<String, Decimal>::new();
        let mut total = Decimal::ZERO;
        for payment in payments {
            let paid = Self::parse_remote_money(
                payment.get("Amount").ok_or_else(|| {
                    "Pago bloqueado: un pago remoto no contiene Amount".to_string()
                })?,
                "Payments.Amount",
            )?;
            let Some(payment_id) = Self::remote_payment_id(payment.get("PaymentId")) else {
                total += paid;
                continue;
            };
            if let Some(previous) = amounts.get(&payment_id) {
                if previous != &paid {
                    return Err(format!(
                        "Pago bloqueado: PaymentId remoto {payment_id} tiene importes contradictorios"
                    ));
                }
                continue;
            }
            amounts.insert(payment_id, paid);
            total += paid;
        }
        Ok((amounts, total))
    }

    /* [R16] Conversión Decimal → f64 para serialización JSON a BDP.
     * Se convierte vía string para máxima precisión; es el enfoque más fiable
     * para Decimal→f64. El redondeo monetario se aplica en los call-sites
     * que lo necesiten, no aquí (vat_pct es un porcentaje, no moneda). */
    pub(crate) fn decimal_to_f64(d: &rust_decimal::Decimal) -> f64 {
        use std::str::FromStr;
        match f64::from_str(&d.to_string()) {
            Ok(v) => v,
            Err(e) => {
                warn!("[R16] Error convirtiendo Decimal '{d}' a f64: {e}");
                0.0
            }
        }
    }

    pub(crate) fn sanitize_error(raw: &str) -> String {
        if raw.contains("401") || raw.contains("403") {
            "Error de autenticación con BDP (401/403)".to_string()
        } else if raw.contains("300035") {
            "BDP: serie no válida (300035)".to_string()
        } else if raw.contains("300008") {
            "BDP: salón/mesa no válidos (300008)".to_string()
        } else if raw.contains("300009") {
            "BDP: delivery no soportado en este POS (300009)".to_string()
        } else if raw.contains("301011") {
            "BDP: MarketplaceOrderId demasiado largo (301011)".to_string()
        } else if raw.contains("301400") {
            "BDP: caja cerrada (301400)".to_string()
        } else {
            let truncated: String = raw.chars().take(200).collect();
            format!("Error BDP: {truncated}")
        }
    }

    pub(crate) fn cleanup_lock(venta_id: uuid::Uuid) {
        let mut map = SYNC_LOCKS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(entry) = map.get(&venta_id) {
            if Arc::strong_count(entry) <= 2 {
                map.remove(&venta_id);
            }
        }
        /* [AUDIT-N3] Sweep periódico: eliminar entradas huérfanas cuyo Arc
         * solo vive en el HashMap (strong_count == 1). Esto previene leak
         * de memoria cuando cleanup_lock no se llama por panic o early return. */
        if map.len() > 100 {
            map.retain(|_, arc| Arc::strong_count(arc) > 1);
        }
    }

    /// [F7.5] Auto-sync de cliente Glory → BDP.
    /// Devuelve únicamente un código BDP previamente confirmado. La creación
    /// automática quedó deliberadamente deshabilitada: `max + 1` y hashes no
    /// pueden garantizar ausencia de colisiones con otros escritores del TPV.
    pub async fn ensure_cliente_bdp_synced(
        pool: &PgPool,
        cliente_id: uuid::Uuid,
        user_id: uuid::Uuid,
        _config: &ConfiguracionRestaurante,
    ) -> Option<i32> {
        let Ok(Some(cliente)) = ClienteRepository::find_by_id(pool, cliente_id, user_id).await
        else {
            return None;
        };
        if let Some(code) = cliente.bdp_customer_code {
            return (code > 0).then_some(code);
        }
        let msg = "Creación automática BDP deshabilitada: asigne y verifique un código explícito desde la sincronización manual";
        let _ = ClienteRepository::update_bdp_sync(pool, cliente_id, None, false, Some(msg)).await;
        None
    }
}
