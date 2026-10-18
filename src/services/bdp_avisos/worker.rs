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

/// Intentos antes de dar el aviso por roto y mandarlo a revisión manual.
pub const MAX_INTENTOS: i32 = 5;
/// Tope por lote: un worker no se come el buzón entero de golpe.
pub const LIMITE_LOTE_MAX: i64 = 50;

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

/// Procesa hasta `limite` avisos `recibido` de la cuenta. Reclama las filas con
/// `FOR UPDATE SKIP LOCKED` para que dos lotes solapados no dupliquen trabajo.
pub async fn procesar_lote(
    pool: &PgPool,
    user_id: Uuid,
    consultor: &impl ConsultaPedido,
    limite: i64,
) -> Result<ResumenLote, String> {
    let reclamados: Vec<(Uuid, String, Value, i32)> = sqlx::query_as(
        r"UPDATE bdp_avisos_inbox
             SET estado = 'procesando', intentos = intentos + 1
           WHERE id IN (
                 SELECT id FROM bdp_avisos_inbox
                  WHERE user_id = $1 AND estado = 'recibido'
                  ORDER BY recibido_en
                  LIMIT $2
                  FOR UPDATE SKIP LOCKED)
          RETURNING id, accion, payload, intentos",
    )
    .bind(user_id)
    .bind(limite.clamp(1, LIMITE_LOTE_MAX))
    .fetch_all(pool)
    .await
    .map_err(|error| format!("No se pudieron reclamar los avisos: {error}"))?;

    let mut resumen = ResumenLote::default();
    for (id, accion, payload, intentos) in reclamados {
        let decision = decidir(&accion, &payload, intentos, consultor).await;
        aplicar(pool, id, &decision).await?;
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
    let comanda = CamposAviso::desde_json(payload)
        .comanda()
        .and_then(|texto| texto.parse::<i64>().ok());
    let Some(comanda) = comanda else {
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

/// Escribe la decisión. `procesado_en` solo se marca cuando el aviso sale de la cola.
async fn aplicar(pool: &PgPool, id: Uuid, decision: &Decision) -> Result<(), String> {
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
    .execute(pool)
    .await
    .map_err(|error| format!("No se pudo guardar el resultado del aviso: {error}"))?;
    Ok(())
}
