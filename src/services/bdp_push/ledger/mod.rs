// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [D26g] Respuesta perdida en stock. Antes del HTTP se guarda S (stock leído con
 * `GetStock`), V y S+V en `bdp_stock_ledger`, con el bloqueo por artículo y
 * almacén para que no haya dos movimientos a la vez. Si el envío falla, se
 * relee `GetStock`: S+V significa que sí se aplicó (no se reintenta), S que no
 * (un único reintento controlado) y cualquier otro valor se para a revisión.
 *
 * BDP no devuelve identificador de operación, y una venta de caja también mueve
 * stock: un valor que no cuadre con S ni con S+V no se puede atribuir sin más,
 * y por eso se para en vez de reintentar. */

mod modelo;
mod registro;
mod resolucion;

use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::ConfiguracionRestaurante;
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{BdpGetListStockRequest, BdpGetStockRequest};

use super::cola::BdpPushPendiente;
use super::constantes::{DOMINIO_STOCK, OPERACION_INVENTARIO};

pub use modelo::{clasificar, ReconciliacionStock};
pub use resolucion::{listar as listar_revisiones, resolver, Decision, FilaRevision, Resolucion};

use modelo::{decimal, lineas_del_payload, stock_desde_respuesta, variacion_de_linea};
use registro::{FilaLibro, FilaPendiente};

/// Pendientes de más de dos horas pasan a revisión; un proceso que murió entre
/// abrir y cerrar el libro no puede bloquear el artículo para siempre.
pub use registro::cerrar as cerrar_libro;

/// Tope de artículos por consulta de `GetListStock` (mismo que el sincronizador).
const ARTICULOS_POR_TANDA: usize = 200;

/// Lo que el envío dejó abierto en el libro y hace falta para reconciliarlo.
pub struct LibroAbierto {
    pub documento_id: Uuid,
    pub filas: Vec<FilaPendiente>,
}

/// Qué hacer con la fila de la cola tras reconciliar una respuesta perdida.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccionLibro {
    /// BDP ya lo tiene aplicado: cerrar como sincronizado sin reenviar.
    Aplicado,
    /// No se aplicó y toca el reintento único: dejar que el flush reintente.
    Reintentar,
    /// No se puede atribuir: parar en revisión manual.
    Revisar,
}

/// Preparado del libro antes de escribir: S leído, V calculada y las filas que
/// se crearán si el bloqueo del par artículo y almacén está libre.
pub struct PreparadoLibro {
    filas: Vec<FilaLibro>,
    documento_id: Uuid,
}

impl PreparadoLibro {
    /// Abre las filas `pendiente` y devuelve lo abierto. Se llama justo antes
    /// del HTTP: entre la lectura de S y el envío no debe haber otro movimiento
    /// del mismo par. Si el bloqueo está ocupado, no se abre ninguna.
    pub async fn abrir(
        mut self,
        pool: &PgPool,
        user_id: Uuid,
        operacion: &str,
    ) -> Result<LibroAbierto, String> {
        let documento_id = self.documento_id;
        registro::abrir(pool, user_id, operacion, &self.filas, documento_id).await?;
        Ok(LibroAbierto {
            documento_id,
            filas: self.filas.drain(..).map(FilaPendiente::desde).collect(),
        })
    }
}

/// Prepara el libro de un envío de stock. Devuelve `None` si la fila no es de
/// stock: el resto de dominios no mueve inventario.
pub async fn preparar(
    config: &ConfiguracionRestaurante,
    client: &BdpWeblinkClient<'_>,
    pendiente: &BdpPushPendiente,
    payload: &serde_json::Value,
) -> Result<Option<PreparadoLibro>, String> {
    if pendiente.dominio != DOMINIO_STOCK {
        return Ok(None);
    }
    /* Sin BDP configurado no hay lectura previa posible: el envío lo cortará
     * antes el cliente y aquí no hay libro que abrir. */
    if sin_credenciales(config) {
        return Ok(None);
    }
    let (almacen, almacen_destino) = destinos(config, &pendiente.operacion, &pendiente.entidad_id);
    let codigos = lineas_del_payload(&pendiente.operacion, payload)?;
    if codigos.is_empty() {
        return Ok(None);
    }
    let lecturas = leer_stock(client, almacen, codigos.iter().map(|(codigo, _)| *codigo)).await?;

    let mut filas = Vec::with_capacity(codigos.len());
    for (articulo, unidades) in &codigos {
        let previo = lecturas.get(articulo).copied().flatten();
        /* En inventario, `Units` es el stock final; V es la diferencia. */
        let Some(variacion) = variacion_de_linea(&pendiente.operacion, *unidades, previo) else {
            /* Sin S no se puede saber qué se espera ver. La línea se queda sin
             * libro (y sin bloqueo), no el lote entero: el resto sí se puede
             * reconciliar. */
            tracing::warn!(
                "[D26g] inventario sin stock previo legible del artículo {articulo}:                  la línea se envía sin libro"
            );
            continue;
        };
        filas.push(FilaLibro {
            /* `id_local` identifica el intento, no la fila de la cola: la misma
             * fila puede reintentar y necesita un libro nuevo (el del intento
             * anterior queda cerrado con su dictamen). [D26g] El vínculo
             * entre intentos es `documento_id`. */
            id_local: Uuid::new_v4().to_string(),
            articulo: *articulo,
            almacen,
            /* El traspaso mueve en origen; el destino queda anotado como dato
             * (no se lee su S, porque la reconciliación es sobre el origen). */
            almacen_destino,
            variacion,
            stock_previo: previo,
        });
    }
    Ok(Some(PreparadoLibro {
        filas,
        documento_id: pendiente.id,
    }))
}

fn sin_credenciales(config: &ConfiguracionRestaurante) -> bool {
    config.bdp_base_url.trim().is_empty()
        || config.bdp_login.trim().is_empty()
        || config.bdp_password.trim().is_empty()
        || config.bdp_integrator_code.trim().is_empty()
}

/// Almacén del movimiento (y destino si es traspaso) a partir de la entidad de
/// la cola. La entidad es la que decidió el handler: `artículo:almacén` en un
/// ajuste, `artículo:origen>destino:uuid` en un traspaso y `conteo:almacén` en
/// un inventario.
fn destinos(
    config: &ConfiguracionRestaurante,
    operacion: &str,
    entidad: &str,
) -> (i32, Option<i32>) {
    if operacion == OPERACION_INVENTARIO {
        if let Some(almacen) = entidad.strip_prefix("conteo:").and_then(numero) {
            return (almacen, None);
        }
        return (config.bdp_almacen_default, None);
    }
    if let Some((origen, destino)) = entidad
        .split_once(':')
        .and_then(|(_, resto)| resto.split_once('>'))
    {
        if let (Some(origen), Some(destino)) =
            (numero(origen), destino.split(':').next().and_then(numero))
        {
            return (origen, Some(destino));
        }
    }
    if let Some(almacen) = entidad.split(':').nth(1).and_then(numero) {
        return (almacen, None);
    }
    (config.bdp_almacen_default, None)
}

fn numero(texto: &str) -> Option<i32> {
    texto.trim().parse().ok()
}

/// S de cada artículo en el almacén. Un artículo que el BDP no conoce queda en
/// `None` (no hay S que comparar) en lugar de tumbar el envío entero.
async fn leer_stock(
    client: &BdpWeblinkClient<'_>,
    almacen: i32,
    codigos: impl Iterator<Item = i64>,
) -> Result<std::collections::HashMap<i64, Option<Decimal>>, String> {
    let codigos: Vec<i64> = codigos.collect();
    let mut lecturas = std::collections::HashMap::new();
    for tanda in codigos.chunks(ARTICULOS_POR_TANDA) {
        let articles = tanda
            .iter()
            .map(
                |&article| crate::services::bdp_weblink_catalog::BdpListStockItemRequest {
                    article,
                    altern: 0,
                },
            )
            .collect();
        let valor = client
            .get_list_stock(&BdpGetListStockRequest {
                store: almacen,
                articles,
            })
            .await
            .map_err(|error| format!("GetListStock del almacén {almacen}: {error}"))?;
        for (codigo, stock) in lineas_leidas(&valor, tanda) {
            lecturas.insert(codigo, stock);
        }
    }
    Ok(lecturas)
}

/// Pares `(artículo, stock)` de una respuesta de `GetListStock`, con `None` en
/// las líneas que traen su propio error.
fn lineas_leidas(valor: &serde_json::Value, pedidos: &[i64]) -> Vec<(i64, Option<Decimal>)> {
    let Some(lista) = valor.get("Stock").and_then(serde_json::Value::as_array) else {
        return pedidos.iter().map(|&codigo| (codigo, None)).collect();
    };
    let mut leidas = Vec::new();
    for linea in lista {
        let Some(articulo) = linea.get("Article").and_then(serde_json::Value::as_i64) else {
            continue;
        };
        if !pedidos.contains(&articulo) {
            continue;
        }
        let con_error = linea
            .get("ErrorMessage")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|mensaje| !mensaje.trim().is_empty());
        let stock = if con_error {
            None
        } else {
            linea.get("Units").and_then(decimal)
        };
        leidas.push((articulo, stock));
    }
    /* Un artículo que no vino en la respuesta no tiene S conocido. */
    for &codigo in pedidos {
        if !leidas.iter().any(|(articulo, _)| *articulo == codigo) {
            leidas.push((codigo, None));
        }
    }
    leidas
}

/// Tras un fallo del envío: relee el stock y decide. Escribe el resultado en el
/// libro (y cierra las filas como `sin_movimiento`, `confirmado`, `rechazado`
/// o `requiere_revision`), pero no toca la fila de la cola.
pub async fn tras_error(
    pool: &PgPool,
    user_id: Uuid,
    client: &BdpWeblinkClient<'_>,
    libro: &LibroAbierto,
    error: &str,
) -> Result<(AccionLibro, String), String> {
    if libro.filas.is_empty() {
        /* De stock pero sin libro (no se pudo leer S de ninguna línea): no hay
         * forma de saber qué pasó, así que no se reintenta a ciegas. */
        let nota = format!("{error}; el envío no dejó libro de stock: no se reintenta");
        return Ok((AccionLibro::Revisar, nota));
    }
    let intentos = registro::intentos_sin_movimiento(pool, user_id, libro.documento_id).await?;

    let mut decisiones = Vec::with_capacity(libro.filas.len());
    for fila in &libro.filas {
        let consulta = client
            .get_stock(&BdpGetStockRequest {
                article: fila.articulo,
                altern: 0,
                store: fila.almacen,
            })
            .await
            .map_err(|error| error.to_string())
            .and_then(|valor| stock_desde_respuesta(&valor));
        let posterior = match consulta {
            Ok(valor) => valor,
            /* Sin lectura no se sabe qué pasó: no se reintenta a ciegas. */
            Err(motivo) => {
                let nota = format!("{error}; no se pudo releer el stock: {motivo}");
                cerrar(pool, user_id, fila, "requiere_revision", &nota, None).await?;
                return Ok((AccionLibro::Revisar, nota));
            }
        };
        let decision = clasificar(fila.stock_previo, fila.variacion, posterior);
        decisiones.push((fila, decision, posterior));
    }

    /* Un lote con cualquier línea discordante no se reintenta: reintentar
     * duplicaría las que sí se aplicaron. */
    if let Some((fila, _, posterior)) = decisiones
        .iter()
        .find(|(_, decision, _)| *decision == ReconciliacionStock::Discordante)
    {
        let nota = format!(
            "{error}; el stock del artículo {} en el almacén {} quedó en {posterior}, \
             que no es ni S ({}) ni S+V ({}): no se reintenta",
            fila.articulo,
            fila.almacen,
            fila.stock_previo
                .map_or_else(|| "desconocido".to_string(), |previo| previo.to_string()),
            fila.stock_previo.map_or_else(
                || "desconocido".to_string(),
                |previo| (previo + fila.variacion).to_string(),
            ),
        );
        for (fila, _, posterior) in &decisiones {
            cerrar(
                pool,
                user_id,
                fila,
                "requiere_revision",
                &nota,
                Some(*posterior),
            )
            .await?;
        }
        return Ok((AccionLibro::Revisar, nota));
    }

    if decisiones
        .iter()
        .all(|(_, decision, _)| *decision == ReconciliacionStock::Aplicado)
    {
        let nota = format!("{error}; el stock ya muestra S+V: el movimiento sí se aplicó");
        for (fila, _, posterior) in &decisiones {
            cerrar(pool, user_id, fila, "confirmado", &nota, Some(*posterior)).await?;
        }
        return Ok((AccionLibro::Aplicado, nota));
    }

    /* Se ve S: el movimiento no llegó. Solo se reintenta si este envío no ha
     * gastado ya su reintento: el libro cuenta los cierres sin movimiento. */
    if intentos >= 1 {
        let nota = format!(
            "{error}; el stock sigue en S, pero este envío ya reintentó una vez \
             (D26g): no se reintenta más"
        );
        for (fila, _, posterior) in &decisiones {
            cerrar(
                pool,
                user_id,
                fila,
                "requiere_revision",
                &nota,
                Some(*posterior),
            )
            .await?;
        }
        return Ok((AccionLibro::Revisar, nota));
    }
    let nota = format!("{error}; el stock sigue en S: se reintenta una vez");
    for (fila, _, posterior) in &decisiones {
        cerrar(
            pool,
            user_id,
            fila,
            "sin_movimiento",
            &nota,
            Some(*posterior),
        )
        .await?;
    }
    Ok((AccionLibro::Reintentar, nota))
}

/// Tras un envío aceptado: las filas quedan `confirmado` y liberan el bloqueo.
pub async fn tras_exito(pool: &PgPool, user_id: Uuid, libro: &LibroAbierto) -> Result<(), String> {
    for fila in &libro.filas {
        cerrar(
            pool,
            user_id,
            fila,
            "confirmado",
            "envío aceptado por BDP",
            None,
        )
        .await?;
    }
    Ok(())
}

async fn cerrar(
    pool: &PgPool,
    user_id: Uuid,
    fila: &FilaPendiente,
    estado: &str,
    nota: &str,
    stock_posterior: Option<Decimal>,
) -> Result<(), String> {
    registro::cerrar(pool, user_id, &fila.id_local, estado, nota, stock_posterior).await
}
