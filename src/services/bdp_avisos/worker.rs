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
        let documento = documento_de(&accion, &payload);
        let vinculo = comanda_de(&payload).zip(decision.pedido.as_ref());
        /* El resumen cuenta el estado con el que el aviso sale de verdad: el cruce
         * de documento puede degradar un `procesado` a incidencia. */
        let estado = aplicar(pool, id, user_id, &decision, documento, vinculo).await?;
        match estado {
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
        /* [D10] Al anular en caja, el TPV local borra la comanda: `GetOrder`
         * responde 301000 en vez de devolverla con `Status: 2` (comprobado el
         * 2026-10-21). Un aviso `cambiar-estado` con Estado=2 sobre una comanda
         * que ya no existe es una anulación coherente, no una incidencia: se
         * sintetiza el pedido cancelado para que la venta quede anulada igual
         * que si BDP la devolviera. */
        Consulta::NoExiste if accion == "cambiar-estado" && aviso_cancela(payload) => Decision {
            estado: "procesado",
            pedido: Some(serde_json::json!({ "Status": 2 })),
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

/// [D10] ¿El aviso declara `Estado: 2` (cancelada)? Es el único caso en que la
/// desaparición de la comanda significa «anulada» y no «no existe».
fn aviso_cancela(payload: &Value) -> bool {
    CamposAviso::desde_json(payload)
        .estado()
        .and_then(|texto| texto.trim().parse::<i64>().ok())
        == Some(2)
}

/// Comanda del aviso como número; None si no es numérica.
fn comanda_de(payload: &Value) -> Option<i64> {
    CamposAviso::desde_json(payload)
        .comanda()
        .and_then(|texto| texto.parse::<i64>().ok())
}

/// [D20] Documento que declara un aviso `finalizar`: la `Serie` del TPV más el
/// `Numero` a ocho dígitos, que es el formato del `InvoiceNumber` de BDP
/// (`00001TB` + `13` → `00001TB00000013`). `None` para el resto de avisos o si
/// falta cualquiera de los dos campos: sin ellos no hay documento que cruzar.
fn documento_de(accion: &str, payload: &Value) -> Option<String> {
    if accion != "finalizar" {
        return None;
    }
    let campos = CamposAviso::desde_json(payload);
    let serie = campos.serie()?.trim();
    let numero = campos.numero()?.trim();
    if serie.is_empty() || numero.is_empty() {
        return None;
    }
    Some(format!("{serie}{numero:0>8}"))
}

/// [D20] Cruza el documento del aviso con el de la venta de esa comanda.
/// Devuelve la nota de incidencia si hay dos documentos distintos para una sola
/// venta; `None` cuando el enlace queda bien o cuando no hay nada que cruzar.
async fn cruzar_documento(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    factura: &str,
    comanda: i64,
) -> Result<Option<String>, String> {
    let guardada: Option<String> = sqlx::query_scalar(
        "SELECT COALESCE(bdp_invoice_number, '') FROM ventas WHERE user_id = $1 AND bdp_order_id = $2",
    )
    .bind(user_id)
    .bind(comanda)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| format!("No se pudo leer la factura de la venta: {error}"))?;

    match guardada.as_deref() {
        /* Sin venta local no hay nada que enlazar: la comanda pudo crearse en otro TPV. */
        None => Ok(None),
        Some(numero) if numero == factura => Ok(None),
        Some(numero) if !numero.is_empty() => Ok(Some(format!(
            "El aviso finalizar trae {factura} y la venta de la comanda {comanda} tiene {numero}."
        ))),
        /* La venta aún no tenía número: el aviso es el documento recién emitido, así
         * que es su fuente. Si el número ya estuviera en otra venta, el índice único
         * lo rechaza y el aviso acaba en revisión en vez de duplicar el documento. */
        Some(_) => {
            /* El `NOT EXISTS` hace el trabajo del índice único sin romper la
             * transacción: un choque deja 0 filas y se convierte en incidencia. */
            let filas = sqlx::query(
                "UPDATE ventas SET bdp_invoice_number = $3, updated_at = NOW() \
                 WHERE user_id = $1 AND bdp_order_id = $2 \
                   AND NOT EXISTS (SELECT 1 FROM ventas otra \
                                    WHERE otra.user_id = $1 AND otra.bdp_invoice_number = $3)",
            )
            .bind(user_id)
            .bind(comanda)
            .bind(factura)
            .execute(&mut **tx)
            .await
            .map_err(|error| {
                format!("No se pudo enlazar el documento {factura} con la venta de la comanda {comanda}: {error}")
            })?
            .rows_affected();
            if filas == 0 {
                return Ok(Some(format!(
                    "El documento {factura} del aviso ya consta en otra venta: la comanda {comanda} queda sin enlazar."
                )));
            }
            Ok(None)
        }
    }
}

/// [28AA-6] Lee las líneas locales de la venta de esa comanda y las compara con el
/// pedido de BDP. Devuelve la nota de conflicto, o `None` si no hay venta local.
async fn conflicto_con_venta(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    comanda: i64,
    pedido: &Value,
) -> Result<Option<String>, String> {
    let (lineas, unidades): (i64, f64) = sqlx::query_as(
        r"SELECT COUNT(l.id), COALESCE(SUM(l.cantidad), 0)::float8
            FROM venta_lineas l
            JOIN ventas v ON v.id = l.venta_id
           WHERE v.user_id = $1 AND v.bdp_order_id = $2",
    )
    .bind(user_id)
    .bind(comanda)
    .fetch_one(&mut **tx)
    .await
    .map_err(|error| format!("No se pudieron leer las líneas de la venta: {error}"))?;
    Ok(conflicto_lineas(comanda, pedido, lineas, unidades))
}

/// [28AA-6] Compara las unidades del pedido de BDP con las de la venta local. Solo
/// unidades, no número de líneas: BDP puede partir una línea de cantidad 2 en dos
/// ítems y eso no es un cambio. Sin líneas locales (venta legacy) o sin ítems en el
/// pedido no hay nada que contradecir, así que no hay conflicto.
fn conflicto_lineas(
    comanda: i64,
    pedido: &Value,
    lineas_locales: i64,
    unidades_locales: f64,
) -> Option<String> {
    if lineas_locales == 0 {
        return None;
    }
    let items = pedido
        .get("Order")
        .unwrap_or(pedido)
        .get("Items")?
        .as_array()?;
    let unidades_bdp: f64 = items
        .iter()
        .map(|item| item.get("Units").and_then(Value::as_f64).unwrap_or(0.0))
        .sum();
    if (unidades_bdp - unidades_locales).abs() < 0.001 {
        return None;
    }
    Some(format!(
        "Conflicto web–TPV en la comanda {comanda}: el TPV tiene {unidades_bdp} uds y la venta local {unidades_locales}. La venta no se modifica; revisar cuál es la correcta."
    ))
}

/// Escribe la decisión y, si hay pedido, enlaza su venta. Va en una transacción:
/// el aviso nunca queda `procesado` sin que la venta de su comanda esté al día.
/// `procesado_en` solo se marca cuando el aviso sale de la cola.
///
/// [D20] Si el aviso declara un documento (`finalizar`) y la venta tiene otro
/// distinto, el aviso pasa a `requiere_revision` y la venta no se toca: enlazar
/// dos documentos diferentes sería justo la incidencia que D20 debe detectar.
///
/// Devuelve el estado con el que el aviso queda de verdad, para que el resumen del
/// lote cuente la incidencia como incidencia y no como éxito.
async fn aplicar(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
    decision: &Decision,
    documento: Option<String>,
    vinculo: Option<(i64, &Value)>,
) -> Result<&'static str, String> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| format!("No se pudo abrir la transacción del aviso: {error}"))?;
    /* El cruce va antes de escribir el estado: decide si el aviso puede darse por
     * procesado o si lo que hay es una incidencia. */
    let incidencia = match (decision.estado, documento.as_deref(), vinculo) {
        ("procesado", Some(factura), Some((comanda, _))) => {
            cruzar_documento(&mut tx, user_id, factura, comanda).await?
        }
        _ => None,
    };
    /* [28AA-6] Sin incidencia de documento, se compara el pedido con las líneas de la
     * venta: si el TPV cambió las líneas, el aviso queda en revisión y la venta no se toca. */
    let incidencia = match (incidencia, decision.estado, vinculo) {
        (None, "procesado", Some((comanda, pedido))) => {
            conflicto_con_venta(&mut tx, user_id, comanda, pedido).await?
        }
        (nota, _, _) => nota,
    };
    let (estado, error, enlazar) = match incidencia {
        Some(nota) => ("requiere_revision", Some(nota), false),
        None => (decision.estado, decision.error.clone(), true),
    };
    sqlx::query(
        r"UPDATE bdp_avisos_inbox
             SET estado = $2::text,
                 ultimo_error = $3,
                 pedido_bdp = COALESCE($4, pedido_bdp),
                 procesado_en = CASE WHEN $2::text = 'recibido' THEN NULL ELSE NOW() END
           WHERE id = $1",
    )
    .bind(id)
    .bind(estado)
    .bind(error.as_deref())
    .bind(decision.pedido.as_ref())
    .execute(&mut *tx)
    .await
    .map_err(|error| format!("No se pudo guardar el resultado del aviso: {error}"))?;
    /* Con incidencia no se enlaza: la venta se queda como estaba hasta que una
     * persona la resuelva, en vez de quedar marcada con un documento que no es el suyo. */
    if enlazar {
        if let Some((comanda, pedido)) = vinculo {
            enlazar_venta(&mut tx, user_id, comanda, pedido).await?;
        }
    }
    tx.commit()
        .await
        .map_err(|error| format!("No se pudo confirmar el aviso: {error}"))?;
    Ok(estado)
}

/// Copia el estado BDP del pedido a la venta con ese `bdp_order_id`. Sin venta local
/// no es error (la comanda pudo crearse en otro TPV). Sin `Status` legible no toca la venta.
///
/// [D10] Si BDP dice `cancelled`, la venta queda anulada también aquí: es un hecho
/// del TPV, no una acción de la web, así que no pasa por los guards de
/// `VentaService::anular` (M9 impide anular una facturada). Se marca una sola vez
/// —`anulada = false` en el WHERE— para que un aviso repetido no pise el motivo
/// ni la fecha de la anulación original.
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
    let anulada_remota = estado == "cancelled";
    sqlx::query(
        r"UPDATE ventas
             SET bdp_order_status = $3::text,
                 bdp_invoiced = ($3::text = 'invoiced'),
                 anulada = CASE WHEN $4 AND NOT anulada THEN TRUE ELSE anulada END,
                 anulada_at = CASE WHEN $4 AND NOT anulada THEN NOW() ELSE anulada_at END,
                 anulacion_motivo = CASE WHEN $4 AND NOT anulada
                                         THEN 'Anulada en el TPV (aviso de BDP)'
                                         ELSE anulacion_motivo END,
                 updated_at = NOW()
           WHERE user_id = $1 AND bdp_order_id = $2",
    )
    .bind(user_id)
    .bind(comanda)
    .bind(estado)
    .bind(anulada_remota)
    .execute(&mut **tx)
    .await
    .map_err(|error| format!("No se pudo actualizar la venta de la comanda: {error}"))?;
    Ok(())
}

/// Lotes máximos por cuenta en cada pasada: acota lo que se le pide a BDP de golpe.
const MAX_LOTES_PASADA: u32 = 10;

/// Segundos entre pasadas del procesador del buzón. `0` lo desactiva.
const SEGUNDOS_AUTO_POR_DEFECTO: u64 = 5;

/// Segundos entre pasadas, leídos de `BDP_AVISOS_AUTO_SEGUNDOS`.
#[must_use]
pub fn segundos_automaticos() -> u64 {
    parsear_segundos(std::env::var("BDP_AVISOS_AUTO_SEGUNDOS").ok().as_deref())
}

fn parsear_segundos(valor: Option<&str>) -> u64 {
    match valor.map(str::trim) {
        None | Some("") => SEGUNDOS_AUTO_POR_DEFECTO,
        Some(texto) => texto.parse::<u64>().unwrap_or_else(|_| {
            tracing::warn!("BDP_AVISOS_AUTO_SEGUNDOS no es un número: se usa el por defecto");
            SEGUNDOS_AUTO_POR_DEFECTO
        }),
    }
}

/// [D29] Cuentas con algo que hacer: avisos `recibido` y avisos `procesando` cuyo
/// reclamo caducó. Lo segundo importa porque, si el worker cae entre el reclamo y
/// el commit (o el proceso se reinicia por deploy), esa fila no vuelve sola a
/// `recibido`; el bucle periódico se quedaba sin verla nunca y el aviso quedaba
/// varado para siempre. Es la misma condición con la que reclama `procesar_lote`.
pub async fn cuentas_con_trabajo(pool: &PgPool) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT DISTINCT user_id FROM bdp_avisos_inbox \
          WHERE estado = 'recibido' \
             OR (estado = 'procesando' \
                 AND reclamado_en < NOW() - ($1::float8 * INTERVAL '1 second'))",
    )
    .bind(CADUCIDAD_RECLAMO_SEG)
    .fetch_all(pool)
    .await
}

/// [D23a] Procesa una pasada de avisos `recibido`. Solo cuentas en modo BDP y con
/// BDP configurado, igual que el endpoint manual; no escribe en el TPV. Devuelve
/// los avisos procesados.
///
/// [10AA-3] Nació como pasada única al arrancar, para los avisos que esperaron con
/// el backend parado; pero el TPV avisa con el backend en marcha, así que una sola
/// pasada al arrancar dejaba el `finalizar` en el buzón hasta el siguiente reinicio
/// (la venta nunca se marcaba facturada). Ahora la llama el bucle periódico de
/// `main.rs`, y el caso de arranque es su primera pasada.
pub async fn procesar_pendientes(pool: &PgPool, modo: &ServicioModoOperacion) -> u32 {
    let usuarios = match cuentas_con_trabajo(pool).await {
        Ok(usuarios) => usuarios,
        Err(error) => {
            tracing::warn!("[D23a] No se pudo listar el buzón: {error}");
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
        for _ in 0..MAX_LOTES_PASADA {
            match procesar_lote(pool, user_id, &cliente, LIMITE_LOTE_MAX).await {
                Ok(resumen) if resumen.procesados > 0 => total += resumen.procesados,
                Ok(_) => break,
                Err(error) => {
                    tracing::warn!("[D23a] Lote del buzón fallido para {user_id}: {error}");
                    break;
                }
            }
        }
    }
    total
}

#[cfg(test)]
mod pruebas_conflicto_lineas {
    use super::conflicto_lineas;
    use serde_json::json;

    fn pedido(unidades: &[f64]) -> serde_json::Value {
        let items: Vec<_> = unidades.iter().map(|u| json!({ "Units": u })).collect();
        json!({ "Order": { "Items": items } })
    }

    #[test]
    fn cuadrar_unidades_no_es_conflicto() {
        assert_eq!(conflicto_lineas(7, &pedido(&[1.0, 2.0]), 2, 3.0), None);
    }

    #[test]
    fn partir_una_linea_en_items_no_es_conflicto() {
        assert_eq!(conflicto_lineas(7, &pedido(&[1.0, 1.0]), 1, 2.0), None);
    }

    #[test]
    fn linea_añadida_en_el_tpv_es_conflicto() {
        let nota = conflicto_lineas(7, &pedido(&[1.0, 2.0, 1.0]), 2, 3.0).unwrap();
        assert!(nota.contains("comanda 7") && nota.contains("4 uds"));
    }

    #[test]
    fn venta_sin_lineas_locales_no_se_compara() {
        assert_eq!(conflicto_lineas(7, &pedido(&[5.0]), 0, 0.0), None);
    }

    #[test]
    fn pedido_sin_items_no_se_compara() {
        assert_eq!(conflicto_lineas(7, &json!({ "Status": 2 }), 2, 3.0), None);
    }
}

#[cfg(test)]
mod pruebas_intervalo {
    use super::parsear_segundos;

    #[test]
    fn segundos_automaticos_usa_por_defecto_y_acepta_cero() {
        assert_eq!(parsear_segundos(None), super::SEGUNDOS_AUTO_POR_DEFECTO);
        assert_eq!(
            parsear_segundos(Some("  ")),
            super::SEGUNDOS_AUTO_POR_DEFECTO
        );
        assert_eq!(parsear_segundos(Some("0")), 0);
        assert_eq!(parsear_segundos(Some("30")), 30);
        assert_eq!(
            parsear_segundos(Some("cinco")),
            super::SEGUNDOS_AUTO_POR_DEFECTO
        );
    }
}
