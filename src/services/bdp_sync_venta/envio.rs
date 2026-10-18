/* [267A-9/F4.1] Envío de la comanda a BDP: reintento con reconciliación por
 * MarketplaceOrderId y fase HTTP de sync_venta. */

use serde_json::Value;
use sqlx::PgPool;
use tracing::{info, warn};

use super::{BdpSyncError, OrderSendFailure};
use crate::models::{ConfiguracionRestaurante, Venta, VentaLinea};
use crate::repositories::VentaLineaRepository;
use crate::services::bdp_sync::{
    BdpSyncService, OrderContext, ResolvedArticle, BDP_SYNC_MARKET_ID,
};
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{BdpGetOrderRequest, BdpOrderIdentifier};

impl BdpSyncService {
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

    /* [R5] Fase HTTP de sync_venta: resolve artículo, contexto, líneas y envío.
     * Se extrae a una función para poder aplicar timeout global con
     * `tokio::time::timeout` sin perder claridad. */
    pub(super) async fn run_http_phase(
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
}
