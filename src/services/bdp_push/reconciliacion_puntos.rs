// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [198A-1/D12c] Antes de reenviar un AddPoints cuya respuesta se perdió (auditoría
 * `ambiguo` con el mismo payload), se compara el saldo de BDP con el ledger local
 * hasta esa operación. Si BDP ya lo tiene aplicado, no se reenvía (evita duplicar
 * el +1); si no lo tiene, se reenvía; si no cuadra con ninguno, se para a revisión. */

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{BdpAddPointsRequest, BdpGetPointsRequest};
use crate::services::{entidad_uuid, BdpBackupService};

use super::cola::{BdpPushPendiente, BdpPushService, MarcarResultado};
use super::constantes::{ESTADO_ERROR, ESTADO_SINCRONIZADO};

/// Resultado de comparar el saldo de BDP con el ledger local hasta la operación.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reconciliacion {
    /// BDP ya contiene el movimiento: no reenviar.
    YaAplicado,
    /// BDP no contiene el movimiento: reenviar.
    Pendiente,
    /// No cuadra con ninguno de los dos casos: no reenviar, revisar a mano.
    Discordante,
}

/// `saldo_local` incluye el `delta` si la operación ya estaba aplicada en BDP.
pub fn clasificar_saldo(
    saldo_bdp: Decimal,
    saldo_local: Decimal,
    delta: Decimal,
) -> Reconciliacion {
    if saldo_bdp == saldo_local {
        Reconciliacion::YaAplicado
    } else if saldo_bdp == saldo_local - delta {
        Reconciliacion::Pendiente
    } else {
        Reconciliacion::Discordante
    }
}

/// Lee `Points` de la respuesta de GetPoints; un `ErrorMessage` no vacío es fallo.
pub fn saldo_desde_respuesta(valor: &Value) -> Result<Decimal, String> {
    if let Some(error) = valor.get("ErrorMessage").and_then(Value::as_str) {
        if !error.is_empty() {
            return Err(format!("GetPoints: {error}"));
        }
    }
    let puntos = valor
        .get("Points")
        .ok_or_else(|| "GetPoints sin campo Points".to_string())?;
    /* Puede venir como número o como texto; se quitan las comillas del texto. */
    let texto = puntos.to_string();
    Decimal::from_str_exact(texto.trim_matches('"'))
        .map_err(|error| format!("Saldo BDP no numérico ({texto}): {error}"))
}

/// Si hay un intento previo ambiguo de esta fila, decide sin reenviar cuando ya
/// está aplicado o no cuadra. `None` = no hay intento previo o BDP no lo tiene:
/// el flujo normal reenvía. `Some(estado)` = la fila ya quedó resuelta.
pub async fn resolver_intento_previo(
    pool: &PgPool,
    user_id: Uuid,
    client: &BdpWeblinkClient<'_>,
    pendiente: &BdpPushPendiente,
) -> Result<Option<&'static str>, String> {
    let entidad = entidad_uuid(&pendiente.dominio, &pendiente.entidad_id);
    let Some((audit_id, creado_en)) =
        ultimo_ambiguo(pool, user_id, entidad, &pendiente.payload).await?
    else {
        return Ok(None);
    };

    let puntos: BdpAddPointsRequest = serde_json::from_value(pendiente.payload.clone())
        .map_err(|error| format!("payload de puntos inválido: {error}"))?;
    let cliente_id = Uuid::parse_str(&pendiente.entidad_id)
        .map_err(|error| format!("entidad de puntos no es un UUID: {error}"))?;
    /* Ledger hasta el envío ambiguo: los movimientos posteriores no cuentan. */
    let saldo_local = saldo_hasta(pool, user_id, cliente_id, creado_en).await?;

    let consulta = client
        .get_points(&BdpGetPointsRequest {
            customer: puntos.customer,
        })
        .await
        .map_err(|error| error.to_string())
        .and_then(|valor| saldo_desde_respuesta(&valor));
    let saldo_bdp = match consulta {
        Ok(saldo) => saldo,
        /* Sin saldo no se sabe si está aplicado: no se reenvía; cuenta como reintento. */
        Err(error) => {
            return marcar(pool, user_id, pendiente, ESTADO_ERROR, Some(&error), true)
                .await
                .map(Some)
        }
    };

    match clasificar_saldo(saldo_bdp, saldo_local, puntos.points_added) {
        Reconciliacion::Pendiente => Ok(None),
        Reconciliacion::YaAplicado => {
            let respuesta = json!({ "reconciliado": true, "saldo_bdp": saldo_bdp.to_string() });
            BdpBackupService::actualizar_resultado(pool, audit_id, "exito", Some(&respuesta), None)
                .await?;
            marcar(pool, user_id, pendiente, ESTADO_SINCRONIZADO, None, false)
                .await
                .map(Some)
        }
        Reconciliacion::Discordante => {
            let error = format!(
                "Saldo BDP {saldo_bdp} no cuadra con el local {saldo_local} (movimiento {}): no se reenvía",
                puntos.points_added
            );
            marcar(pool, user_id, pendiente, ESTADO_ERROR, Some(&error), false)
                .await
                .map(Some)
        }
    }
}

/// Último intento de AddPoints con resultado ambiguo y el mismo payload.
async fn ultimo_ambiguo(
    pool: &PgPool,
    user_id: Uuid,
    entidad: Uuid,
    payload: &Value,
) -> Result<Option<(Uuid, DateTime<Utc>)>, String> {
    sqlx::query_as::<_, (Uuid, DateTime<Utc>)>(
        "SELECT id, created_at FROM bdp_audit_log \
         WHERE user_id = $1 AND operacion = 'add_points' AND resultado = 'ambiguo' \
           AND target_entity_id = $2 AND datos_enviados = $3 \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user_id)
    .bind(entidad)
    .bind(payload)
    .fetch_optional(pool)
    .await
    .map_err(|error| format!("No se pudo consultar la auditoría BDP: {error}"))
}

/// Suma del ledger del cliente con `created_at` hasta `hasta`.
async fn saldo_hasta(
    pool: &PgPool,
    user_id: Uuid,
    cliente_id: Uuid,
    hasta: DateTime<Utc>,
) -> Result<Decimal, String> {
    sqlx::query_scalar::<_, Decimal>(
        "SELECT COALESCE(SUM(points_added), 0) FROM bdp_puntos_cliente \
         WHERE user_id = $1 AND cliente_id = $2 AND created_at <= $3",
    )
    .bind(user_id)
    .bind(cliente_id)
    .bind(hasta)
    .fetch_one(pool)
    .await
    .map_err(|error| format!("No se pudo calcular el saldo local: {error}"))
}

async fn marcar(
    pool: &PgPool,
    user_id: Uuid,
    pendiente: &BdpPushPendiente,
    estado: &'static str,
    error: Option<&str>,
    incrementar_reintento: bool,
) -> Result<&'static str, String> {
    BdpPushService::marcar_resultado(
        pool,
        user_id,
        MarcarResultado {
            dominio: &pendiente.dominio,
            entidad_id: &pendiente.entidad_id,
            operacion: &pendiente.operacion,
            estado,
            error,
            incrementar_reintento,
        },
    )
    .await?;
    Ok(estado)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    fn d(texto: &str) -> Decimal {
        Decimal::from_str(texto).expect("decimal de prueba")
    }

    #[test]
    fn saldo_igual_al_local_significa_ya_aplicado() {
        assert_eq!(
            clasificar_saldo(d("4"), d("4"), d("1")),
            Reconciliacion::YaAplicado
        );
    }

    #[test]
    fn saldo_un_movimiento_menor_significa_pendiente() {
        assert_eq!(
            clasificar_saldo(d("3"), d("4"), d("1")),
            Reconciliacion::Pendiente
        );
    }

    #[test]
    fn saldo_que_no_cuadra_es_discordante() {
        assert_eq!(
            clasificar_saldo(d("10"), d("4"), d("1")),
            Reconciliacion::Discordante
        );
    }

    #[test]
    fn lee_points_numerico_y_textual() {
        assert_eq!(
            saldo_desde_respuesta(&json!({"Points": 4, "ErrorMessage": ""})),
            Ok(d("4"))
        );
        assert_eq!(
            saldo_desde_respuesta(&json!({"Points": "3.50", "ErrorMessage": ""})),
            Ok(d("3.50"))
        );
    }

    #[test]
    fn error_message_no_vacio_es_fallo() {
        assert!(
            saldo_desde_respuesta(&json!({"Points": 0, "ErrorMessage": "cliente no existe"}))
                .is_err()
        );
        assert!(saldo_desde_respuesta(&json!({"ErrorMessage": ""})).is_err());
    }
}
