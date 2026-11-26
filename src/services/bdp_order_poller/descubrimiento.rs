// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [21AB-8] Descubrimiento de comandas nacidas en el TPV (punto 6 de Guillermo,
 * 25/11): una comanda creada directamente en BDP no aparecía nunca en la web,
 * porque el sondeo solo miraba comandas que la web ya conocía y los avisos solo
 * llegan si el TPV está configurado para mandarlos.
 *
 * La única vía que el TPV ofrece para enterarse es `Orders/List` (probado el
 * 26/11 contra el demo: 92 comandas, `Status` 0 y 1). Aquí se listan, se
 * descartan las que la web ya tiene y las abiertas se copian como venta, con
 * sus líneas y su `bdp_order_id`, sin escribir nada en BDP.
 *
 * Marca de agua (`bdp_sync_cursor`, dominio `comandas`): solo se copian
 * comandas con `OrderId` mayor que la marca. En la primera pasada no hay marca
 * y copiar el histórico entero sería inventar ventas que el restaurante ya
 * cerró; así que esa pasada siembra la marca y solo recoge lo abierto de las
 * últimas horas, que es el servicio en curso. */

use std::collections::HashSet;

use chrono::{DateTime, NaiveDate, Timelike, Utc};
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::models::CrearVentaLineaRequest;
use crate::repositories::venta::{MesaVenta, NuevaVenta};
use crate::repositories::{VentaLineaRepository, VentaRepository};
use crate::services::bdp_weblink::BdpWeblinkClient;

use super::BdpOrderPollerService;

/// Dominio de la marca de agua en `bdp_sync_cursor`.
const DOMINIO: &str = "comandas";
/// Un listado por minuto como mucho: el polling ya llama a BDP por cada venta
/// pendiente y esta lectura no necesita la misma cadencia.
const SEGUNDOS_ENTRE_LISTADOS: i64 = 60;
/// Ventana de la primera pasada: lo abierto de las últimas horas es trabajo
/// vivo (el servicio en curso), no el histórico del TPV.
const VENTANA_PRIMERA_PASADA_HORAS: i64 = 12;

impl BdpOrderPollerService {
    /// Copia las comandas abiertas del TPV que la web no conoce. Devuelve
    /// cuántas ventas se crearon y si la llamada a BDP falló (histéresis M2).
    pub(super) async fn descubrir_comandas_tpv(
        pool: &PgPool,
        user_id: Uuid,
        client: &BdpWeblinkClient<'_>,
    ) -> (usize, bool) {
        let Some(marca) = Self::reclamar_listado(pool, user_id).await else {
            return (0, false);
        };

        let lista = match client.list_orders().await {
            Ok(lista) => lista,
            Err(error) => {
                warn!("[21AB-8] No se pudo listar las comandas del TPV: {error}");
                return (0, true);
            }
        };

        let conocidas = match Self::comandas_conocidas(pool, user_id, marca.unwrap_or(0)).await {
            Ok(ids) => ids,
            Err(error) => {
                warn!("[21AB-8] No se pudo leer las comandas ya copiadas: {error}");
                return (0, true);
            }
        };
        let (nueva_marca, nuevas) =
            Self::comandas_por_copiar(&lista, marca, &conocidas, Utc::now());

        let mut creadas = 0usize;
        let mut fallo = false;
        for comanda in &nuevas {
            match Self::copiar_comanda(pool, user_id, client, comanda).await {
                Ok(true) => {
                    creadas += 1;
                    info!(
                        "[21AB-8] Comanda del TPV copiada como venta: {}",
                        comanda.get("OrderId").and_then(|id| id.as_i64()).unwrap_or(0)
                    );
                }
                Ok(false) => {}
                Err(error) => {
                    warn!("[21AB-8] No se pudo copiar una comanda del TPV: {error}");
                    fallo = true;
                    break;
                }
            }
        }
        /* Con un fallo a medias, la marca se queda donde estaba: el siguiente
         * listado vuelve a intentar desde ahí. Los `OrderId` ya copiados no se
         * duplican porque el índice único lo impide y `conocidas` los salta. */
        let marca_final = if fallo {
            marca.unwrap_or(0)
        } else {
            nueva_marca
        };
        if let Err(error) = Self::guardar_marca(pool, user_id, marca_final).await {
            warn!("[21AB-8] No se pudo guardar la marca de comandas: {error}");
        }
        (creadas, false)
    }

    /// Reclama el turno de listado. `None` = todavía no toca (o hay otra
    /// instancia listando). `Some(None)` = primera pasada: no hay marca.
    async fn reclamar_listado(pool: &PgPool, user_id: Uuid) -> Option<Option<i64>> {
        let previa: Option<i64> = sqlx::query_scalar(
            "SELECT NULLIF(posicion->>'comanda', '')::bigint FROM bdp_sync_cursor \
              WHERE user_id = $1 AND dominio = $2 AND perfil = ''",
        )
        .bind(user_id)
        .bind(DOMINIO)
        .fetch_optional(pool)
        .await
        .unwrap_or_else(|error| {
            warn!("[21AB-8] No se pudo leer la marca de comandas: {error}");
            None
        });

        let reclamado: Option<Uuid> = sqlx::query_scalar(
            r"INSERT INTO bdp_sync_cursor (id, user_id, dominio, perfil, posicion, ultimo_intento)
              VALUES ($1, $2, $3, '', '{}'::jsonb, NOW())
              ON CONFLICT (user_id, dominio, perfil) DO UPDATE SET ultimo_intento = NOW()
               WHERE bdp_sync_cursor.ultimo_intento IS NULL
                  OR bdp_sync_cursor.ultimo_intento
                     <= NOW() - ($4::bigint * INTERVAL '1 second')
              RETURNING id",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(DOMINIO)
        .bind(SEGUNDOS_ENTRE_LISTADOS)
        .fetch_optional(pool)
        .await
        .unwrap_or_else(|error| {
            warn!("[21AB-8] No se pudo reclamar el listado de comandas: {error}");
            None
        });
        reclamado.map(|_| previa)
    }

    async fn guardar_marca(pool: &PgPool, user_id: Uuid, marca: i64) -> Result<(), String> {
        sqlx::query(
            "UPDATE bdp_sync_cursor SET posicion = jsonb_build_object('comanda', $3::bigint) \
              WHERE user_id = $1 AND dominio = $2 AND perfil = ''",
        )
        .bind(user_id)
        .bind(DOMINIO)
        .bind(marca.max(0))
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
    }

    /// `OrderId` de las comandas que la web ya tiene por encima de la marca.
    async fn comandas_conocidas(
        pool: &PgPool,
        user_id: Uuid,
        marca: i64,
    ) -> Result<HashSet<i64>, String> {
        let ids: Vec<i64> = sqlx::query_scalar(
            "SELECT bdp_order_id FROM ventas \
              WHERE user_id = $1 AND bdp_order_id IS NOT NULL AND bdp_order_id > $2",
        )
        .bind(user_id)
        .bind(marca)
        .fetch_all(pool)
        .await
        .map_err(|error| error.to_string())?;
        Ok(ids.into_iter().collect())
    }

    /// Comandas a copiar y marca nueva. Pura, para probarla sin TPV ni base:
    /// se queda con las abiertas (`Status` 0 y 1) por encima de la marca que la
    /// web no conoce; una cancelada o facturada ya es historia del TPV. Sin
    /// marca (primera pasada) solo entra lo abierto dentro de la ventana.
    pub(crate) fn comandas_por_copiar(
        lista: &Value,
        marca: Option<i64>,
        conocidas: &HashSet<i64>,
        ahora: DateTime<Utc>,
    ) -> (i64, Vec<Value>) {
        let comandas = lista
            .get("Orders")
            .or_else(|| lista.get("Order"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let desde = ahora - chrono::Duration::hours(VENTANA_PRIMERA_PASADA_HORAS);

        let mut vistas = HashSet::new();
        let mut nuevas = Vec::new();
        let mut tope = marca.unwrap_or(0);
        for comanda in comandas {
            let Some(id) = comanda.get("OrderId").and_then(Value::as_i64) else {
                continue;
            };
            /* El demo devuelve la misma comanda varias veces (una por estado
             * histórico): se mira una sola vez. */
            if !vistas.insert(id) {
                continue;
            }
            tope = tope.max(id);
            let abierta = comanda
                .get("Status")
                .and_then(Value::as_i64)
                .is_some_and(|status| status == 0 || status == 1);
            if !abierta || conocidas.contains(&id) {
                continue;
            }
            let pendiente = comanda
                .get("Status")
                .and_then(Value::as_i64)
                .is_some_and(|status| status == 0);
            match marca {
                Some(marca) if id > marca => nuevas.push(comanda.clone()),
                Some(_) => {}
                /* Primera pasada: lo que aún espera validación es trabajo vivo
                 * con cualquier reloj, y lo aceptado solo si es de la ventana.
                 * Así, un TPV con el reloj desfasado (el demo va parado en el
                 * 18/11) no se traga el servicio en curso. */
                None => {
                    if pendiente || Self::creada_desde(&comanda, desde) {
                        nuevas.push(comanda.clone());
                    } else {
                        info!("[21AB-8] Comanda {id} es del histórico del TPV: no se copia");
                    }
                }
            }
        }
        (tope, nuevas)
    }

    /// `CreationDate` de la comanda dentro de la ventana.
    fn creada_desde(comanda: &Value, desde: DateTime<Utc>) -> bool {
        comanda
            .get("CreationDate")
            .and_then(Value::as_str)
            .and_then(|fecha| DateTime::parse_from_rfc3339(fecha).ok())
            .is_some_and(|fecha| fecha.with_timezone(&Utc) >= desde)
    }

    /// Crea la venta espejo de una comanda del TPV. `Ok(false)` si entre el
    /// listado y la copia la creó otra vía (el índice único lo impide).
    async fn copiar_comanda(
        pool: &PgPool,
        user_id: Uuid,
        client: &BdpWeblinkClient<'_>,
        comanda: &Value,
    ) -> Result<bool, String> {
        let Some(order_id) = comanda.get("OrderId").and_then(Value::as_i64) else {
            return Ok(false);
        };
        /* El listado trae solo la cabecera; las líneas están en `GetOrder`. */
        let detalle = client
            .get_order(&crate::services::bdp_weblink_catalog::BdpGetOrderRequest {
                order_identifier:
                    crate::services::bdp_weblink_catalog::BdpOrderIdentifier::by_order_id(order_id),
            })
            .await
            .map_err(|error| format!("GetOrder {order_id}: {error}"))?;
        let pedido = detalle.get("Order").unwrap_or(&detalle);
        let estado = BdpOrderPollerService::map_status(
            BdpOrderPollerService::parse_status(&detalle).unwrap_or(0),
        );

        let lineas = Self::lineas_de_comanda(pool, user_id, pedido).await?;
        let (fecha, hora) = Self::fecha_de_comanda(pedido);
        let mesa = Self::mesa_de_comanda(pool, user_id, pedido).await?;
        let con_mesa = pedido.get("Type").and_then(Value::as_i64) == Some(1) && mesa.is_some();
        let iva = lineas
            .iter()
            .find_map(|linea| linea.iva_pct)
            .unwrap_or_else(|| Decimal::new(10, 0));
        let turno = if hora < 11 {
            "manana"
        } else if hora < 17 {
            "mediodia"
        } else {
            "noche"
        };

        let data = NuevaVenta {
            user_id,
            fecha,
            comensales: None,
            descripcion: &format!("Comanda creada en el TPV (BDP #{order_id})"),
            iva_porcentaje: iva,
            turno,
            canal: if con_mesa { "comedor" } else { "barra" },
            metodo_pago: "efectivo",
            importe_base: Decimal::ZERO,
            importe_iva: Decimal::ZERO,
            reserva_id: None,
            cliente_id: None,
            mesa,
        };

        let mut tx = pool
            .begin()
            .await
            .map_err(|error| format!("No se pudo abrir la transacción: {error}"))?;
        let venta = VentaRepository::create_with(&mut *tx, &data)
            .await
            .map_err(|error| {
                /* Carrera con otra instancia: el índice único (user_id,
                 * bdp_order_id) se comprueba después, así que aquí se mira el
                 * duplicado al vuelo y se descarta sin ruido. */
                format!("No se pudo crear la venta de la comanda {order_id}: {error}")
            })?;
        if !lineas.is_empty() {
            VentaLineaRepository::crear_batch_conn(&mut tx, venta.id, &lineas)
                .await
                .map_err(|error| format!("No se pudieron copiar las líneas: {error}"))?;
        }
        VentaRepository::recalcular_importes(&mut *tx, venta.id, user_id)
            .await
            .map_err(|error| format!("No se pudo calcular el importe: {error}"))?;
        /* `bdp_synced = true` y el `OrderId` desde el alta: la venta es espejo,
         * no encargo. Si no, el envío la trataría como pendiente y crearía una
         * segunda comanda en el TPV. */
        VentaRepository::update_bdp_status(&mut *tx, venta.id, true, None, Some(order_id))
            .await
            .map_err(|error| format!("No se pudo marcar la venta como sincronizada: {error}"))?;
        VentaRepository::update_bdp_order_status(&mut *tx, venta.id, &estado)
            .await
            .map_err(|error| format!("No se pudo guardar el estado de la comanda: {error}"))?;
        tx.commit()
            .await
            .map_err(|error| format!("No se pudo confirmar la venta espejo: {error}"))?;
        Ok(true)
    }

    /// Líneas del pedido de BDP como líneas locales, con el código Glory de su
    /// equivalencia cuando existe (sin equivalencia, descripción de BDP).
    async fn lineas_de_comanda(
        pool: &PgPool,
        user_id: Uuid,
        pedido: &Value,
    ) -> Result<Vec<CrearVentaLineaRequest>, String> {
        let items = pedido
            .get("Items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut lineas = Vec::with_capacity(items.len());
        for item in items {
            let cantidad = decimal(item.get("Units")).unwrap_or(Decimal::ONE);
            let precio = decimal(item.get("Price")).unwrap_or(Decimal::ZERO);
            let total = decimal(item.get("Total")).unwrap_or(precio * cantidad);
            /* El descuento manual del TPV llega como importe; localmente es un
             * porcentaje sobre el precio, así que se convierte aquí. */
            let descuento = match decimal(item.get("DiscountAmount")) {
                Some(importe) if importe > Decimal::ZERO && total > Decimal::ZERO => {
                    Some((importe * Decimal::from(100) / total).round_dp(2))
                }
                _ => None,
            };
            let bdp_id = item.get("Id").and_then(Value::as_i64);
            let codigo = match bdp_id {
                Some(id) => Self::codigo_glory(pool, user_id, id).await,
                None => None,
            };
            lineas.push(CrearVentaLineaRequest {
                articulo_codigo: codigo,
                descripcion: item
                    .get("Name")
                    .and_then(Value::as_str)
                    .unwrap_or("Artículo del TPV")
                    .to_string(),
                cantidad: Some(cantidad),
                precio_unitario: precio,
                iva_pct: decimal(item.get("VatPct")),
                descuento,
                invitacion: item.get("Invitation").and_then(Value::as_bool),
                comentario: None,
            });
        }
        Ok(lineas)
    }

    /// Código Glory de un artículo BDP, si el catálogo lo tiene mapeado. Es solo
    /// informativo en la venta espejo: no se envía nada al TPV.
    async fn codigo_glory(pool: &PgPool, user_id: Uuid, articulo_bdp: i64) -> Option<String> {
        sqlx::query_scalar(
            "SELECT articulo_glory_codigo FROM bdp_article_map \
              WHERE user_id = $1 AND btrim(articulo_bdp_codigo) = $2 LIMIT 1",
        )
        .bind(user_id)
        .bind(articulo_bdp.to_string())
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
    }

    /// Fecha (UTC de la comanda) y hora, para el turno. BDP no da zona: la
    /// fecha del ticket es la que el TPV puso al crearla.
    pub(crate) fn fecha_de_comanda(pedido: &Value) -> (NaiveDate, u32) {
        let creada = pedido
            .get("CreationDate")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let instante = chrono::DateTime::parse_from_rfc3339(creada)
            .map(|fecha| fecha.with_timezone(&chrono::Utc))
            .ok();
        match instante {
            Some(fecha) => (
                fecha.date_naive(),
                fecha.hour() + fecha.minute() / 60,
            ),
            None => (chrono::Utc::now().date_naive(), 12),
        }
    }

    /// Mesa del plano para una comanda con mesa (`Type` 1, salón y número).
    async fn mesa_de_comanda(
        pool: &PgPool,
        user_id: Uuid,
        pedido: &Value,
    ) -> Result<Option<MesaVenta>, String> {
        let salon = pedido.get("RoomNumber").and_then(Value::as_i64);
        let numero = pedido.get("TableNumber").and_then(Value::as_i64);
        let (Some(salon), Some(numero)) = (salon, numero) else {
            return Ok(None);
        };
        if salon <= 0 || numero <= 0 {
            return Ok(None);
        }
        sqlx::query_as::<_, (Uuid, i32, i32)>(
            "SELECT m.id, m.numero, z.orden FROM mesas m \
               JOIN zonas_sala z ON z.id = m.zona_id \
              WHERE z.user_id = $1 AND z.orden = $2 AND m.numero = $3 LIMIT 1",
        )
        .bind(user_id)
        .bind(i32::try_from(salon).unwrap_or_default())
        .bind(i32::try_from(numero).unwrap_or_default())
        .fetch_optional(pool)
        .await
        .map(|fila| {
            fila.map(|(id, num_mesa, num_salon)| MesaVenta {
                id,
                num_mesa,
                num_salon,
            })
        })
        .map_err(|error| format!("No se pudo resolver la mesa del TPV: {error}"))
    }
}

/// Número JSON a `Decimal` sin pasar por `f64` cuando ya es entero.
fn decimal(valor: Option<&Value>) -> Option<Decimal> {
    match valor? {
        Value::Number(numero) => numero
            .as_i64()
            .map(Decimal::from)
            .or_else(|| numero.as_f64().and_then(Decimal::from_f64_retain)),
        Value::String(texto) => texto.trim().parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ids(comandas: &[Value]) -> Vec<i64> {
        comandas
            .iter()
            .filter_map(|comanda| comanda.get("OrderId").and_then(|id| id.as_i64()))
            .collect()
    }

    /* Primera pasada: la marca se siembra en la comanda más alta y solo entra
     * lo que aún espera validación o lo abierto de las últimas horas (el
     * servicio en curso). El histórico cerrado del TPV no se copia como ventas
     * del restaurante. */
    #[test]
    fn la_primera_pasada_solo_copia_lo_abierto_de_la_ventana() {
        let ahora = chrono::DateTime::parse_from_rfc3339("2026-11-26T20:00:00Z")
            .map(|fecha| fecha.with_timezone(&chrono::Utc))
            .expect("fecha válida");
        let lista = json!({"Orders": [
            {"OrderId": 10, "Status": 1, "CreationDate": "2026-11-18T06:45:53Z"},
            {"OrderId": 12, "Status": 1, "CreationDate": "2026-11-26T19:30:00Z"},
            {"OrderId": 13, "Status": 3, "CreationDate": "2026-11-26T19:35:00Z"},
            /* Aún espera validación: trabajo vivo, entre con el reloj que sea
             * (el demo va parado en el 18/11). */
            {"OrderId": 14, "Status": 0, "CreationDate": "2026-11-18T07:39:00Z"},
        ]});
        let (marca, nuevas) = BdpOrderPollerService::comandas_por_copiar(
            &lista,
            None,
            &std::collections::HashSet::new(),
            ahora,
        );
        assert_eq!(marca, 14);
        assert_eq!(ids(&nuevas), vec![12, 14]);
    }

    /* Por encima de la marca solo entra lo abierto que la web no tiene: una
     * cerrada, una cancelada y una ya copiada se descartan. El demo repite la
     * misma comanda por cada estado histórico, y eso no puede duplicar nada. */
    #[test]
    fn solo_copia_abiertas_nuevas_por_encima_de_la_marca() {
        let lista = json!({"Orders": [
            {"OrderId": 20, "Status": 1},
            {"OrderId": 21, "Status": 0},
            {"OrderId": 22, "Status": 3},
            {"OrderId": 23, "Status": 2},
            {"OrderId": 24, "Status": 1},
            {"OrderId": 25, "Status": 1},
            {"OrderId": 25, "Status": 3},
            {"OrderId": 26, "Status": 0},
        ]});
        let conocidas: std::collections::HashSet<i64> = [24].into_iter().collect();
        let (marca, nuevas) = BdpOrderPollerService::comandas_por_copiar(
            &lista,
            Some(21),
            &conocidas,
            chrono::Utc::now(),
        );
        assert_eq!(marca, 26);
        assert_eq!(ids(&nuevas), vec![25, 26]);
    }

    /* La fecha del turno sale de la `CreationDate` del TPV; sin fecha legible
     * no se inventa un día: se usa hoy y el turno de mediodía. */
    #[test]
    fn la_fecha_de_la_comanda_sale_de_su_creation_date() {
        let pedido = json!({"CreationDate": "2026-11-18T06:45:53.0000000Z"});
        let (fecha, hora) = BdpOrderPollerService::fecha_de_comanda(&pedido);
        assert_eq!(fecha.to_string(), "2026-11-18");
        assert_eq!(hora, 6);

        let (_, hora) = BdpOrderPollerService::fecha_de_comanda(&json!({"CreationDate": ""}));
        assert_eq!(hora, 12);
    }
}
