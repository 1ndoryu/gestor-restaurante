// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [10AA-3/F2] Worker de avisos: completa cada aviso `recibido` con el pedido real
 * que devuelve BDP (`GetOrder`) y lo deja `procesado`, o lo reintenta o lo manda
 * a revisión. Solo lee de BDP y escribe en `bdp_avisos_inbox`: nunca toca el TPV
 * ni las ventas. La consulta va detrás de un trait para poder probarlo sin red. */

use std::future::Future;

use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use super::campos::CamposAviso;
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{BdpGetOrderRequest, BdpOrderIdentifier};
use crate::services::{
    BdpOrderPollerService, ConfiguracionService, ModoEfectivo, ServicioModoOperacion,
};

/// Intentos antes de dar el aviso por roto y mandarlo a revisión manual.
pub const MAX_INTENTOS: i32 = 5;
/// Tope por lote: un worker no se come el buzón entero de golpe.
pub const LIMITE_LOTE_MAX: i64 = 50;
/* Un aviso en `procesando` más de este tiempo se da por abandonado (el worker cayó
 * tras reclamarlo) y se retoma. Con más margen que cualquier consulta a BDP. */
const CADUCIDAD_RECLAMO_SEG: f64 = 600.0;

/// Lo que devuelve BDP al pedir una comanda, ya clasificado.
#[derive(Debug, Clone, PartialEq)]
pub enum Consulta {
    Encontrado(Value),
    /// BDP responde `301000`: la comanda no existe (o ya no).
    NoExiste,
    /// Cualquier otro fallo, transitorio o no: se reintenta hasta `MAX_INTENTOS`.
    Fallo(String),
}

/// Fuente del pedido real. En producción es el cliente `WebLink`; en pruebas, un doble.
pub trait ConsultaPedido: Sync {
    fn pedido(&self, comanda: i64) -> impl Future<Output = Consulta> + Send;
}

impl ConsultaPedido for BdpWeblinkClient<'_> {
    async fn pedido(&self, comanda: i64) -> Consulta {
        let req = BdpGetOrderRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(comanda),
        };
        match self.get_order(&req).await {
            Ok(valor) => Consulta::Encontrado(valor),
            /* El código de error viene en el texto de la respuesta; se busca por
             * su número porque el cliente no tipa los errores remotos. */
            Err(error) if error.to_string().contains("301000") => Consulta::NoExiste,
            Err(error) => Consulta::Fallo(error.to_string()),
        }
    }
}

/// Recuento de un lote, para la respuesta de la API y para los tests.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct ResumenLote {
    pub procesados: u32,
    pub reintentos: u32,
    pub revision: u32,
}

/// Procesa hasta `limite` avisos de la cuenta: los `recibido` y los `procesando` cuyo
/// reclamo caducó. Reclama las filas con `FOR UPDATE SKIP LOCKED` para que dos lotes
/// solapados no dupliquen trabajo.
pub async fn procesar_lote(
    pool: &PgPool,
    user_id: Uuid,
    consultor: &impl ConsultaPedido,
    limite: i64,
) -> Result<ResumenLote, String> {
    let reclamados: Vec<(Uuid, String, Value, i32)> = sqlx::query_as(
        r"UPDATE bdp_avisos_inbox
             SET estado = 'procesando', intentos = intentos + 1, reclamado_en = NOW()
           WHERE id IN (
                 SELECT id FROM bdp_avisos_inbox
                  WHERE user_id = $1
                    AND (estado = 'recibido'
                         OR (estado = 'procesando'
                             AND reclamado_en < NOW() - ($3::float8 * INTERVAL '1 second')))
                  ORDER BY recibido_en
                  LIMIT $2
                  FOR UPDATE SKIP LOCKED)
          RETURNING id, accion, payload, intentos",
    )
    .bind(user_id)
    .bind(limite.clamp(1, LIMITE_LOTE_MAX))
    .bind(CADUCIDAD_RECLAMO_SEG)
    .fetch_all(pool)
    .await
    .map_err(|error| format!("No se pudieron reclamar los avisos: {error}"))?;

    let mut resumen = ResumenLote::default();
    for (id, accion, payload, intentos) in reclamados {
        let decision = decidir(&accion, &payload, intentos, consultor).await;
        let vinculo = comanda_de(&payload).zip(decision.pedido.as_ref());
        aplicar(pool, id, user_id, &decision, vinculo).await?;
        match decision.estado {
            "procesado" => resumen.procesados += 1,
            "recibido" => resumen.reintentos += 1,
            _ => resumen.revision += 1,
        }
    }
    Ok(resumen)
}

/// Lo que se hace con un aviso tras consultar BDP.
#[derive(Debug, PartialEq)]
struct Decision {
    estado: &'static str,
    pedido: Option<Value>,
    error: Option<String>,
}

/// Decide el estado a partir de la consulta. Pura salvo por la llamada a BDP.
async fn decidir(
    accion: &str,
    payload: &Value,
    intentos: i32,
    consultor: &impl ConsultaPedido,
) -> Decision {
    /* Sin comanda numérica no hay nada que consultar: se guarda para revisión
     * y no se llama a BDP con un identificador inventado. */
    let Some(comanda) = comanda_de(payload) else {
        return Decision {
            estado: "requiere_revision",
            pedido: None,
            error: Some("El aviso no trae un IdentificadorComanda numérico.".to_string()),
        };
    };

    match consultor.pedido(comanda).await {
        Consulta::Encontrado(pedido) => Decision {
            estado: "procesado",
            pedido: Some(pedido),
            error: None,
        },
        /* Tras un borrado es lo esperado que la comanda ya no exista. */
        Consulta::NoExiste if accion == "borrar" => Decision {
            estado: "procesado",
            pedido: None,
            error: None,
        },
        Consulta::NoExiste => Decision {
            estado: "requiere_revision",
            pedido: None,
            error: Some("BDP no conoce la comanda (301000).".to_string()),
        },
        Consulta::Fallo(error) if intentos >= MAX_INTENTOS => Decision {
            estado: "requiere_revision",
            pedido: None,
            error: Some(format!("Agotados {MAX_INTENTOS} intentos: {error}")),
        },
        Consulta::Fallo(error) => Decision {
            estado: "recibido",
            pedido: None,
            error: Some(error),
        },
    }
}

/// Comanda del aviso como número; None si no es numérica.
fn comanda_de(payload: &Value) -> Option<i64> {
    CamposAviso::desde_json(payload)
        .comanda()
        .and_then(|texto| texto.parse::<i64>().ok())
}

/// Escribe la decisión y, si hay pedido, enlaza su venta. Va en una transacción:
/// el aviso nunca queda `procesado` sin que la venta de su comanda esté al día.
/// `procesado_en` solo se marca cuando el aviso sale de la cola.
async fn aplicar(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
    decision: &Decision,
    vinculo: Option<(i64, &Value)>,
) -> Result<(), String> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| format!("No se pudo abrir la transacción del aviso: {error}"))?;
    sqlx::query(
        r"UPDATE bdp_avisos_inbox
             SET estado = $2::text,
                 ultimo_error = $3,
                 pedido_bdp = COALESCE($4, pedido_bdp),
                 procesado_en = CASE WHEN $2::text = 'recibido' THEN NULL ELSE NOW() END
           WHERE id = $1",
    )
    .bind(id)
    .bind(decision.estado)
    .bind(decision.error.as_deref())
    .bind(decision.pedido.as_ref())
    .execute(&mut *tx)
    .await
    .map_err(|error| format!("No se pudo guardar el resultado del aviso: {error}"))?;
    if let Some((comanda, pedido)) = vinculo {
        enlazar_venta(&mut tx, user_id, comanda, pedido).await?;
    }
    tx.commit()
        .await
        .map_err(|error| format!("No se pudo confirmar el aviso: {error}"))
}

/// Copia el estado BDP del pedido a la venta con ese `bdp_order_id`. Sin venta local
/// no es error (la comanda pudo crearse en otro TPV). Sin `Status` legible no toca la venta.
async fn enlazar_venta(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    comanda: i64,
    pedido: &Value,
) -> Result<(), String> {
    let Ok(codigo) = BdpOrderPollerService::parse_status(pedido) else {
        return Ok(());
    };
    let estado = BdpOrderPollerService::map_status(codigo);
    sqlx::query(
        r"UPDATE ventas
             SET bdp_order_status = $3::text,
                 bdp_invoiced = ($3::text = 'invoiced'),
                 updated_at = NOW()
           WHERE user_id = $1 AND bdp_order_id = $2",
    )
    .bind(user_id)
    .bind(comanda)
    .bind(estado)
    .execute(&mut **tx)
    .await
    .map_err(|error| format!("No se pudo actualizar la venta de la comanda: {error}"))?;
    Ok(())
}

/// Lotes máximos por cuenta al arrancar: acota el tiempo de arranque si el buzón es grande.
const MAX_LOTES_ARRANQUE: u32 = 10;

/// [D23a] Al arrancar el backend procesa los avisos `recibido` que esperaron en el
/// buzón mientras estaba parado. Solo cuentas en modo BDP y con BDP configurado,
/// igual que el endpoint manual; no escribe en el TPV. Devuelve los avisos procesados.
pub async fn procesar_pendientes_al_arrancar(pool: &PgPool, modo: &ServicioModoOperacion) -> u32 {
    let usuarios: Vec<Uuid> = match sqlx::query_scalar(
        "SELECT DISTINCT user_id FROM bdp_avisos_inbox WHERE estado = 'recibido'",
    )
    .fetch_all(pool)
    .await
    {
        Ok(usuarios) => usuarios,
        Err(error) => {
            tracing::warn!("[D23a] No se pudo listar el buzón al arrancar: {error}");
            return 0;
        }
    };

    let mut total = 0;
    for user_id in usuarios {
        if !matches!(
            modo.modo_efectivo(pool, user_id).await,
            Ok(ModoEfectivo::Bdp)
        ) {
            continue;
        }
        let Ok(config) = ConfiguracionService::obtener(pool, user_id).await else {
            continue;
        };
        if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
            continue;
        }
        let cliente = BdpWeblinkClient::new(&config);
        for _ in 0..MAX_LOTES_ARRANQUE {
            match procesar_lote(pool, user_id, &cliente, LIMITE_LOTE_MAX).await {
                Ok(resumen) if resumen.procesados > 0 => total += resumen.procesados,
                Ok(_) => break,
                Err(error) => {
                    tracing::warn!("[D23a] Lote de arranque fallido para {user_id}: {error}");
                    break;
                }
            }
        }
    }
    total
}
