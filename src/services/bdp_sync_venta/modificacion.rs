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
            let clave = format!(
                "{}|{}|{}|{}|{}",
                linea.articulo_codigo,
                linea.descripcion,
                linea.precio_unitario.normalize(),
                linea.iva_pct.normalize(),
                linea.descuento.normalize()
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

impl BdpSyncService {
    /* [02AB-2] Releer antes de escribir: si el TPV cambió la comanda (total
     * distinto del que la web conoce) no se sobrescribe nada. */
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
        let total_bdp = orden.get("Total").and_then(Value::as_f64);
        let total_web = Self::total_pedido(venta, Some(antes));
        if total_bdp.is_none_or(|t| (t - total_web).abs() > 0.005) {
            return Err(format!(
                "Conflicto: la comanda {order_id} ya no coincide con la web (total en BDP {}, en la web {total_web:.2}). Se cambió en el TPV; revísela allí o concilie antes de modificar.",
                total_bdp.map_or_else(|| "desconocido".into(), |t| format!("{t:.2}"))
            ));
        }
        Ok(())
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

        let client = BdpWeblinkClient::new(config);
        Self::comprobar_sin_cambios_en_tpv(&client, venta, antes, order_id).await?;

        let article = Self::resolve_article(&client, pool, venta.user_id, config).await;
        let ids = Self::resolve_line_articles(pool, venta.user_id, &ajustes, article.id).await;
        let items = Self::items_de_lineas(&ajustes, Some(&ids), &article);
        let total_final = Self::total_pedido(venta, Some(despues));
        let order_ctx = Self::resolve_order_context(pool, venta, config).await;
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

        /* Misma auditoría y armado que la creación (alcance `create_order`). */
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
                scope: "create_order",
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
            created_at: Utc::now(),
        }
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

    #[test]
    fn delta_rechaza_descuento() {
        let antes = [linea("Coca", "1", "1.05", "0.10")];
        let despues = [linea("Coca", "2", "1.05", "0.10")];
        assert!(delta_lineas(&antes, &despues).is_err());
    }
}
