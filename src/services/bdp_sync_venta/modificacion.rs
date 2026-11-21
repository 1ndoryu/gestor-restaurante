/* [02AB-1] Modificar en BDP una comanda ya creada. Regla probada en el demo
 * (Agente/documentacion/bdp/sondas-modificar-comanda-y-mesas-2026-11-02.md):
 * CreateOrder con el `OrderId` existente añade solo las líneas enviadas y
 * `Order.Total` es el total FINAL; cambiar o quitar = línea con unidades
 * negativas/positivas de ajuste. */

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde_json::Value;

use crate::models::{ConfiguracionRestaurante, Venta, VentaLinea};
use crate::services::bdp_sync::BdpSyncService;
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{
    BdpCreateOrderRequest, BdpGetOrderRequest, BdpOrderIdentifier,
};
use crate::services::{EscrituraAutorizar, ModoEfectivo, ServicioModoOperacion};

/// Líneas de ajuste (cantidad con signo) que llevan `antes` a `despues`.
/// Se agrupa por artículo, texto, precio e IVA; una línea con descuento no
/// se puede ajustar por diferencia y se rechaza.
pub(crate) fn delta_lineas(
    antes: &[VentaLinea],
    despues: &[VentaLinea],
) -> Result<Vec<VentaLinea>, String> {
    let mut mapa: BTreeMap<String, (VentaLinea, Decimal)> = BTreeMap::new();
    for (lineas, signo) in [(antes, -Decimal::ONE), (despues, Decimal::ONE)] {
        for linea in lineas {
            /* [18AB-4] La invitación entra en la clave: una línea invitada y
             * otra cobrada del mismo artículo no pueden fundirse en un ajuste,
             * porque BDP las trata distinto (`Invitation` y total 0). */
            let clave = format!(
                "{}|{}|{}|{}|{}|{}",
                linea.articulo_codigo,
                linea.descripcion,
                linea.precio_unitario.normalize(),
                linea.iva_pct.normalize(),
                linea.descuento.normalize(),
                linea.invitacion
            );
            let entrada = mapa
                .entry(clave)
                .or_insert_with(|| (linea.clone(), Decimal::ZERO));
            entrada.1 += linea.cantidad * signo;
        }
    }
    let mut ajustes = Vec::new();
    for (mut linea, diferencia) in mapa.into_values() {
        if diferencia.is_zero() {
            continue;
        }
        if !linea.descuento.is_zero() {
            return Err(format!(
                "La línea '{}' lleva descuento y no se puede ajustar en BDP por diferencia; modifíquela en el TPV.",
                linea.descripcion
            ));
        }
        linea.cantidad = diferencia;
        ajustes.push(linea);
    }
    Ok(ajustes)
}

/* [21AB-2] Nombre de la primera línea que saldría con `Units: 0`, si hay.
 * Solo el cero: un ajuste negativo es la contrapartida de retirar producto y
 * BDP la acepta (doc 20/11, ticket 00001TB00000051). Pura para poder probarla
 * sin TPV: es la última puerta antes del `CreateOrder` de una modificación. */
pub(crate) fn items_con_cero_uds(items: &[Value]) -> Option<&str> {
    items
        .iter()
        .find(|item| {
            item.get("Units")
                .and_then(Value::as_f64)
                .is_some_and(|units| units == 0.0)
        })
        .map(|item| {
            item.get("Name")
                .and_then(Value::as_str)
                .unwrap_or("sin nombre")
        })
}

/* [02AB-2] Texto del conflicto cuando el total de BDP no es el que la web
 * conoce (el TPV cambió la comanda); `None` si coinciden. Lo comparten la
 * modificación, el cobro y el sondeo periódico. */
pub(crate) fn mensaje_conflicto(
    order_id: i64,
    total_bdp: Option<f64>,
    total_web: f64,
) -> Option<String> {
    if total_bdp.is_some_and(|t| (t - total_web).abs() <= 0.005) {
        return None;
    }
    Some(format!(
        "Conflicto: la comanda {order_id} ya no coincide con la web (total en BDP {}, en la web {total_web:.2}). Se cambió en el TPV; revísela allí o concilie antes de continuar.",
        total_bdp.map_or_else(|| "desconocido".into(), |t| format!("{t:.2}"))
    ))
}

/* [18AA-8] Mesa de la comanda en BDP según `GetOrder`. `None` si la respuesta
 * no la trae (no se puede afirmar que difiera). */
pub(crate) fn mesa_en_bdp(orden: &Value) -> Option<(i64, i64)> {
    Some((
        orden.get("RoomNumber")?.as_i64()?,
        orden.get("TableNumber")?.as_i64()?,
    ))
}

/* [18AA-8] Comprobado el 18/11/2026 en el demo 36.2: reenviar `Orders/Create`
 * con un `OrderId` colocado en otra mesa **no** la mueve; deja la comanda en
 * las dos y, si el total cuadra con la ocupante, fusiona las líneas en ella.
 * Por eso no se modifica una comanda que BDP tiene en una mesa distinta de la
 * de la venta. Sin mesa en la venta (barra) no se compara: su `RoomNumber`
 * sale del tipo de comanda, no de un salón. */
pub(crate) fn mensaje_mesa_distinta(venta: &Venta, orden: &Value) -> Option<String> {
    let (Some(salon), Some(mesa)) = (venta.num_salon, venta.num_mesa) else {
        return None;
    };
    match mesa_en_bdp(orden) {
        Some((salon_bdp, mesa_bdp)) if salon_bdp != i64::from(salon) || mesa_bdp != i64::from(mesa) => {
            Some(format!(
                "La comanda está en BDP en el salón {salon_bdp}, mesa {mesa_bdp}, y la venta en el salón {salon}, mesa {mesa}. Modificarla así la dejaría colocada en las dos mesas; muévala en el TPV y vuelva a intentarlo."
            ))
        }
        _ => None,
    }
}

impl BdpSyncService {
    /* [02AB-2] Releer antes de escribir: si el TPV cambió la comanda (total
     * distinto del que la web conoce) no se sobrescribe nada. [18AA-8] Tampoco
     * si la tiene en otra mesa. */
    async fn comprobar_sin_cambios_en_tpv(
        client: &BdpWeblinkClient<'_>,
        venta: &Venta,
        antes: &[VentaLinea],
        order_id: i64,
    ) -> Result<(), String> {
        let respuesta = client
            .get_order(&BdpGetOrderRequest {
                order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            })
            .await
            .map_err(|e| format!("No se pudo releer la comanda {order_id} en BDP: {e}"))?;
        let orden = respuesta.get("Order").unwrap_or(&respuesta);
        if let Some(m) = mensaje_mesa_distinta(venta, orden) {
            return Err(m);
        }
        let total_bdp = orden.get("Total").and_then(Value::as_f64);
        match mensaje_conflicto(order_id, total_bdp, Self::total_pedido(venta, Some(antes))) {
            Some(m) => Err(m),
            None => Ok(()),
        }
    }

    /// Envía a BDP la diferencia entre las líneas guardadas y las nuevas de una
    /// venta ya sincronizada. Devuelve `Ok(())` si no hay nada que enviar o si
    /// BDP aceptó el cambio; cualquier `Err` deja la venta local sin tocar.
    pub async fn modificar_comanda(
        pool: &sqlx::PgPool,
        venta: &Venta,
        antes: &[VentaLinea],
        despues: &[VentaLinea],
        config: &ConfiguracionRestaurante,
    ) -> Result<(), String> {
        let Some(order_id) = venta.bdp_order_id.filter(|id| *id > 0) else {
            return Err("La venta figura sincronizada pero no tiene OrderId de BDP; concilie la comanda antes de modificarla.".into());
        };
        if ServicioModoOperacion::modo_efectivo_desde_config(config) != ModoEfectivo::Bdp
            || !crate::services::BdpWriteGuard::modo_permite_escritura(&config.bdp_sync_mode)
        {
            return Err(
                "BDP está en solo lectura: la modificación no se puede enviar al TPV.".into(),
            );
        }
        let ajustes = delta_lineas(antes, despues)?;
        if ajustes.is_empty() {
            return Ok(());
        }
        if ajustes.iter().any(|l| l.precio_unitario < Decimal::ZERO) {
            return Err("Hay una línea con precio negativo; no se envía a BDP.".into());
        }
        /* [21AB-5] Misma puerta de descuentos que en el alta, antes de tocar la
         * red: un ajuste de una línea invitada cambiaría el importe del ticket. */
        Self::exigir_etapa_descuentos(pool, venta.user_id, &ajustes).await?;

        let client = BdpWeblinkClient::new(config);
        Self::comprobar_sin_cambios_en_tpv(&client, venta, antes, order_id).await?;

        let article = Self::resolve_article(&client, pool, venta.user_id, config).await;
        let ids = Self::resolve_line_articles(pool, venta.user_id, &ajustes, article.id).await;
        let items = Self::items_de_lineas(&ajustes, Some(&ids), &article);
        /* [21AB-2] Última puerta antes del HTTP, igual que en el alta: un ajuste
         * de 0 uds dejaría una línea que BDP no puede facturar (`Units: 0`
         * revienta el cobro con «Intento de dividir por cero»). `delta_lineas`
         * ya descarta los deltas nulos, pero la comprobación no depende de eso. */
        if let Some(malo) = items_con_cero_uds(&items) {
            return Err(format!(
                "No se envía la modificación a BDP: una línea queda con 0 unidades ({malo})"
            ));
        }
        let total_final = Self::total_pedido(venta, Some(despues));
        /* [21AB-6] Mismo corte que en el alta: un canal de domicilio no se
         * envía a un TPV que no admite `Type=2`. */
        let order_ctx = Self::resolve_order_context(pool, venta, config).await?;
        let mut pedido_json =
            Self::json_pedido(config, venta, &order_ctx, &items, total_final, false);
        if let Some(mapa) = pedido_json.as_object_mut() {
            mapa.insert("OrderId".into(), order_id.into());
            /* Comanda abierta: sin forma de pago ni propina en la modificación. */
            mapa.remove("TenderId");
            mapa.remove("Tip");
        }
        let peticion = BdpCreateOrderRequest {
            employee_id: config.bdp_employee_id,
            items_profile_id: config.bdp_items_profile_id,
            order_end_type: config.bdp_order_end_type,
            order_operation_type: 0,
            invoice: Some(false),
            order: pedido_json,
        };

        /* [21AB-5] Auditoría y armado propios (`modify_order`): modificar una
         * comanda viva se enciende aparte de crearla. Con `create_order` la
         * etapa de comandas gobernaba también las modificaciones, y el índice
         * `uq_bdp_audit_create_vivo` habría bloqueado la modificación como si
         * fuera una segunda creación de la misma venta. */
        let datos = serde_json::json!({
            "venta_id": venta.id, "operacion": "update_order",
            "order_id": order_id, "lineas_ajuste": ajustes.len(), "total_final": total_final
        });
        let snapshot = crate::services::BdpBackupService::preparar_snapshot_escritura(
            pool,
            venta.user_id,
            "update_order",
            config,
            Some(order_id),
        )
        .await
        .map_err(|e| format!("Pre-write audit BDP falló; escritura bloqueada: {e}"))?;
        let audit_id = crate::services::BdpWriteGuard::authorize(
            pool,
            venta.user_id,
            config,
            EscrituraAutorizar {
                scope: "modify_order",
                target_entity_type: "venta",
                target_entity_id: venta.id,
                entity_json_field: "venta_id",
                datos_enviados: &datos,
                snapshot_pre_id: snapshot,
                idempotency_key: None,
            },
        )
        .await
        .map_err(|e| e.clone())?;

        let resultado = client.create_order(&peticion).await;
        let (estado, mensaje) = match &resultado {
            Ok(r) => match r.get("ErrorMessage").and_then(Value::as_str).map(str::trim) {
                Some(m) if !m.is_empty() => ("error", Some(m.to_string())),
                _ => ("exito", None),
            },
            /* Un corte tras enviar puede haberse aplicado: queda ambiguo. */
            Err(e) => ("ambiguo", Some(e.to_string())),
        };
        let _ = crate::services::BdpBackupService::actualizar_resultado(
            pool,
            audit_id,
            estado,
            resultado.as_ref().ok(),
            mensaje.as_deref(),
        )
        .await;
        match (estado, mensaje) {
            ("exito", _) => Ok(()),
            (_, m) => Err(format!(
                "BDP no aceptó la modificación: {}",
                Self::sanitize_error(&m.unwrap_or_default())
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::str::FromStr;

    fn linea(desc: &str, cant: &str, precio: &str, descuento: &str) -> VentaLinea {
        VentaLinea {
            id: uuid::Uuid::new_v4(),
            venta_id: uuid::Uuid::nil(),
            articulo_codigo: "1001".into(),
            descripcion: desc.into(),
            cantidad: Decimal::from_str(cant).unwrap(),
            precio_unitario: Decimal::from_str(precio).unwrap(),
            iva_pct: Decimal::from(10),
            descuento: Decimal::from_str(descuento).unwrap(),
            invitacion: false,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn conflicto_solo_si_los_totales_difieren() {
        assert!(mensaje_conflicto(1, Some(8.70), 8.70).is_none());
        assert!(mensaje_conflicto(1, Some(9.70), 8.70)
            .unwrap()
            .contains("9.70"));
        assert!(mensaje_conflicto(1, None, 8.70)
            .unwrap()
            .contains("desconocido"));
    }

    #[test]
    fn delta_sin_cambios_es_vacio() {
        let a = [linea("Coca", "1", "1.05", "0")];
        assert!(delta_lineas(&a, &a).unwrap().is_empty());
    }

    #[test]
    fn delta_anade_cambia_y_quita() {
        let antes = [
            linea("Coca", "2", "1.05", "0"),
            linea("Agua", "1", "1.00", "0"),
        ];
        let despues = [
            linea("Coca", "3", "1.05", "0"),
            linea("Cafe", "1", "1.20", "0"),
        ];
        let d = delta_lineas(&antes, &despues).unwrap();
        let por = |n: &str| d.iter().find(|l| l.descripcion == n).unwrap().cantidad;
        assert_eq!(por("Coca"), Decimal::ONE);
        assert_eq!(por("Agua"), -Decimal::ONE);
        assert_eq!(por("Cafe"), Decimal::ONE);
        assert_eq!(d.len(), 3);
    }

    /* Venta mínima: solo importan la mesa de la venta y el total de `antes`. */
    fn venta_en(salon: Option<i32>, mesa: Option<i32>) -> Venta {
        Venta {
            id: uuid::Uuid::nil(),
            user_id: uuid::Uuid::nil(),
            fecha: chrono::NaiveDate::from_ymd_opt(2026, 11, 18).expect("fecha"),
            comensales: None,
            descripcion: String::new(),
            iva_porcentaje: Decimal::from(10),
            turno: "cena".into(),
            canal: "comedor".into(),
            metodo_pago: "efectivo".into(),
            importe_base: Decimal::ZERO,
            importe_iva: Decimal::ZERO,
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
            mesa_id: None,
            num_mesa: mesa,
            num_salon: salon,
        }
    }

    fn orden_en(salon: i64, mesa: i64) -> Value {
        serde_json::json!({ "OrderId": 1, "RoomNumber": salon, "TableNumber": mesa, "Total": 1.05 })
    }

    /* [18AA-8] Reenviar con la comanda en otra mesa la duplica o fusiona líneas. */
    #[test]
    fn mesa_distinta_bloquea_la_modificacion() {
        let venta = venta_en(Some(1), Some(8));
        let mensaje = mensaje_mesa_distinta(&venta, &orden_en(1, 5)).expect("debe bloquear");
        assert!(mensaje.contains("salón 1, mesa 5"), "{mensaje}");
        assert!(mensaje.contains("salón 1, mesa 8"), "{mensaje}");
    }

    #[test]
    fn misma_mesa_o_barra_no_bloquea() {
        let venta = venta_en(Some(1), Some(8));
        assert!(mensaje_mesa_distinta(&venta, &orden_en(1, 8)).is_none());
        /* Barra: la venta no tiene mesa y BDP devuelve 0/0. */
        let barra = venta_en(None, None);
        assert!(mensaje_mesa_distinta(&barra, &orden_en(0, 0)).is_none());
        /* Respuesta sin mesa: no se puede afirmar que difiera. */
        assert!(mensaje_mesa_distinta(&venta, &serde_json::json!({ "OrderId": 1 })).is_none());
    }

    #[test]
    fn delta_rechaza_descuento() {
        let antes = [linea("Coca", "1", "1.05", "0.10")];
        let despues = [linea("Coca", "2", "1.05", "0.10")];
        assert!(delta_lineas(&antes, &despues).is_err());
    }

    /* [21AB-2] Una línea de 0 uds en un `CreateOrder` de modificación revienta
     * el cobro posterior en el TPV con «Intento de dividir por cero». */
    #[test]
    fn una_linea_de_cero_uds_se_detecta() {
        let items = serde_json::json!([
            { "Name": "COCA-COLA", "Units": 1.0 },
            { "Name": "SCHWEPPES LIMÓN", "Units": 0.0 }
        ]);
        assert_eq!(
            items_con_cero_uds(items.as_array().expect("array")),
            Some("SCHWEPPES LIMÓN")
        );
    }

    /* Un ajuste negativo es válido: es cómo se quita producto de una comanda. */
    #[test]
    fn ajustes_negativos_pasan_la_puerta() {
        let items = serde_json::json!([
            { "Name": "COCA-COLA", "Units": -2.0 },
            { "Name": "SCHWEPPES LIMÓN", "Units": 0.5 }
        ]);
        assert_eq!(items_con_cero_uds(items.as_array().expect("array")), None);
    }

    /* `delta_lineas` no produce 0 uds, pero la puerta tiene que verlo igual. */
    #[test]
    fn delta_no_genera_cero_uds() {
        let antes = [linea("Coca", "2", "1.05", "0")];
        let despues = [linea("Coca", "2", "1.05", "0")];
        /* Sin diferencias: no hay ajustes y por tanto no hay línea que enviar. */
        assert!(delta_lineas(&antes, &despues).unwrap().is_empty());
    }
}
