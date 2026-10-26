// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
//! [26AA-2] Lectura programada de formas de pago y empleados del BDP.
//!
//! Antes solo se consultaban a mano. El barrido decide *cuándo* (cadencia por
//! cuenta en `bdp_lectura_programada`, apagada de fábrica); aquí se decide *qué
//! se guarda*: los registros se relacionan por identificador BDP, los que BDP
//! deja de publicar pasan a inactivos y nunca se borran ni se tocan los mapeos.
//! Solo lee del TPV: no escribe en BDP ni pide armado.

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::services::bdp_weblink_catalog::{BdpGetEmployeesRequest, BdpGetPosTendersRequest};
use crate::services::{
    BdpWeblinkClient, ConfiguracionService, ModoEfectivo, ServicioModoOperacion,
};

/// Misma cadencia recomendada que el resto de maestros.
pub const SEGUNDOS_RECOMENDADOS: i32 = 900;
/// Cada cuánto se mira qué cuenta está vencida (no llama a BDP).
pub const GRANO_BUCLE_SEGUNDOS: u64 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Maestro {
    FormasPago,
    Empleados,
}

impl Maestro {
    pub const TODOS: [Self; 2] = [Self::FormasPago, Self::Empleados];

    /// Valor guardado en `bdp_lectura_programada.maestro`.
    #[must_use]
    pub const fn clave(self) -> &'static str {
        match self {
            Self::FormasPago => "formas_pago",
            Self::Empleados => "empleados",
        }
    }

    #[must_use]
    pub fn desde_clave(clave: &str) -> Option<Self> {
        Self::TODOS.into_iter().find(|m| m.clave() == clave)
    }

    /// Tabla destino: nombres fijos de este módulo, nunca de la entrada.
    #[must_use]
    pub const fn tabla(self) -> &'static str {
        match self {
            Self::FormasPago => "bdp_formas_pago",
            Self::Empleados => "bdp_empleados",
        }
    }
}

/// Un registro leído de BDP, reducido a lo que se guarda.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistroBdp {
    pub id: i64,
    pub nombre: String,
    pub tipo: Option<i64>,
}

/// Entero o decimal (`12.0`) → entero, igual que en el resto de maestros.
fn entero(valor: Option<&Value>) -> Option<i64> {
    let texto = match valor? {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        _ => return None,
    };
    texto.trim().split('.').next()?.parse().ok()
}

/// Interpreta la respuesta de BDP. Un `ErrorMessage` no vacío es un fallo aunque
/// el HTTP sea 200, y una lista vacía también: dejaría a todos como inactivos
/// por un fallo transitorio, así que no se acepta como «BDP no tiene ninguno».
pub fn registros_de(
    valor: &Value,
    colecciones: &[&str],
    ids: &[&str],
) -> Result<Vec<RegistroBdp>, String> {
    if let Some(mensaje) = valor
        .get("ErrorMessage")
        .and_then(Value::as_str)
        .filter(|m| !m.trim().is_empty())
    {
        return Err(format!("BDP respondió: {mensaje}"));
    }
    let items = colecciones
        .iter()
        .find_map(|clave| valor.get(*clave).and_then(Value::as_array))
        .ok_or_else(|| "La respuesta del BDP no trae la lista esperada".to_string())?;
    let registros: Vec<RegistroBdp> = items
        .iter()
        .filter_map(|item| {
            let id = ids.iter().find_map(|clave| entero(item.get(*clave)))?;
            let nombre = ["Name", "Nombre", "Description"]
                .iter()
                .find_map(|clave| item.get(*clave).and_then(Value::as_str))
                .unwrap_or("")
                .trim()
                .to_string();
            Some(RegistroBdp {
                id,
                nombre,
                tipo: entero(item.get("Type")),
            })
        })
        .collect();
    if registros.is_empty() {
        return Err("BDP devolvió una lista vacía; no se desactiva nada".to_string());
    }
    Ok(registros)
}

/// Guarda la lectura en una transacción: da de alta o reactiva los vistos y
/// desactiva los ausentes. Devuelve (nuevos, desactivados).
async fn guardar(
    pool: &PgPool,
    user_id: Uuid,
    maestro: Maestro,
    registros: &[RegistroBdp],
) -> Result<(u64, u64), sqlx::Error> {
    let tabla = maestro.tabla();
    let mut tx = pool.begin().await?;
    let mut nuevos = 0;
    for r in registros {
        /* `xmax = 0` solo en las filas insertadas: distingue alta de cambio. */
        let alta: bool = sqlx::query_scalar(&format!(
            "INSERT INTO {tabla} (user_id, bdp_id, nombre, tipo, activo, visto_en) \
             VALUES ($1, $2, $3, $4, TRUE, NOW()) \
             ON CONFLICT (user_id, bdp_id) DO UPDATE \
             SET nombre = EXCLUDED.nombre, tipo = EXCLUDED.tipo, activo = TRUE, visto_en = NOW() \
             RETURNING (xmax = 0)"
        ))
        .bind(user_id)
        .bind(r.id)
        .bind(&r.nombre)
        .bind(r.tipo)
        .fetch_one(&mut *tx)
        .await?;
        if alta {
            nuevos += 1;
        }
    }
    let vistos: Vec<i64> = registros.iter().map(|r| r.id).collect();
    let desactivados = sqlx::query(&format!(
        "UPDATE {tabla} SET activo = FALSE \
         WHERE user_id = $1 AND activo AND bdp_id <> ALL($2)"
    ))
    .bind(user_id)
    .bind(&vistos)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    tx.commit().await?;
    Ok((nuevos, desactivados))
}

/// Pide la lista a BDP y la guarda. Devuelve el resumen para la pantalla.
async fn leer(
    pool: &PgPool,
    user_id: Uuid,
    client: &BdpWeblinkClient<'_>,
    pos_id: i32,
    maestro: Maestro,
) -> Result<String, String> {
    let registros = match maestro {
        Maestro::FormasPago => {
            let valor = client
                .get_pos_tenders(&BdpGetPosTendersRequest { pos_id })
                .await
                .map_err(|e| e.to_string())?;
            registros_de(&valor, &["TenderList", "Tenders"], &["Id", "TenderId"])?
        }
        Maestro::Empleados => {
            let valor = client
                .get_employees(&BdpGetEmployeesRequest {
                    ids: Vec::new(),
                    only_salespeople: None,
                })
                .await
                .map_err(|e| e.to_string())?;
            registros_de(&valor, &["Employees", "EmployeeList"], &["Id"])?
        }
    };
    let (nuevos, desactivados) = guardar(pool, user_id, maestro, &registros)
        .await
        .map_err(|e| format!("no se pudo guardar: {e}"))?;
    Ok(format!(
        "{} leídos, {nuevos} nuevos, {desactivados} inactivos",
        registros.len()
    ))
}

/// Reclama el turno en la misma sentencia que lo comprueba (dos instancias no
/// leen a la vez). La marca va antes de leer: si falla, se espera una cadencia.
async fn reclamar(pool: &PgPool, user_id: Uuid, maestro: Maestro) -> Result<bool, sqlx::Error> {
    sqlx::query(
        "UPDATE bdp_lectura_programada SET ultima_lectura = NOW() \
         WHERE user_id = $1 AND maestro = $2 AND segundos > 0 \
           AND (ultima_lectura IS NULL \
                OR ultima_lectura <= NOW() - segundos * INTERVAL '1 second')",
    )
    .bind(user_id)
    .bind(maestro.clave())
    .execute(pool)
    .await
    .map(|r| r.rows_affected() > 0)
}

/// Anota el resultado: la pantalla enseña la última lectura buena y el último error.
async fn anotar(
    pool: &PgPool,
    user_id: Uuid,
    maestro: Maestro,
    resultado: &Result<String, String>,
) {
    let (ok, error, resumen) = match resultado {
        Ok(r) => (true, None, Some(r.as_str())),
        Err(e) => (false, Some(e.as_str()), None),
    };
    let consulta = sqlx::query(
        "UPDATE bdp_lectura_programada \
         SET ultima_ok = CASE WHEN $3 THEN NOW() ELSE ultima_ok END, \
             ultimo_error = $4, resumen = COALESCE($5, resumen) \
         WHERE user_id = $1 AND maestro = $2",
    )
    .bind(user_id)
    .bind(maestro.clave())
    .bind(ok)
    .bind(error)
    .bind(resumen)
    .execute(pool)
    .await;
    if let Err(e) = consulta {
        tracing::warn!(
            "[26AA-2] No se pudo anotar la lectura de {}: {e}",
            maestro.clave()
        );
    }
}

/// Estado de la lectura de un maestro, para la pantalla.
pub struct Estado {
    pub segundos: i32,
    pub ultima_lectura: Option<DateTime<Utc>>,
    pub ultima_ok: Option<DateTime<Utc>>,
    pub ultimo_error: Option<String>,
    pub resumen: Option<String>,
    pub activos: i64,
    pub inactivos: i64,
}

/// Cadencia, última lectura y recuento. Sin fila, la lectura está apagada.
pub async fn estado(pool: &PgPool, user_id: Uuid, maestro: Maestro) -> Result<Estado, sqlx::Error> {
    let fila = sqlx::query(
        "SELECT segundos, ultima_lectura, ultima_ok, ultimo_error, resumen \
         FROM bdp_lectura_programada WHERE user_id = $1 AND maestro = $2",
    )
    .bind(user_id)
    .bind(maestro.clave())
    .fetch_optional(pool)
    .await?;
    /* La tabla destino es un nombre fijo del enum, no una entrada externa. */
    let cuentas = sqlx::query(&format!(
        "SELECT COUNT(*) FILTER (WHERE activo) AS activos, \
                COUNT(*) FILTER (WHERE NOT activo) AS inactivos \
         FROM {} WHERE user_id = $1",
        maestro.tabla()
    ))
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    let f = fila.as_ref();
    Ok(Estado {
        segundos: f.and_then(|f| f.try_get("segundos").ok()).unwrap_or(0),
        ultima_lectura: f.and_then(|f| f.try_get("ultima_lectura").ok()),
        ultima_ok: f.and_then(|f| f.try_get("ultima_ok").ok()),
        ultimo_error: f.and_then(|f| f.try_get("ultimo_error").ok()),
        resumen: f.and_then(|f| f.try_get("resumen").ok()),
        activos: cuentas.try_get("activos")?,
        inactivos: cuentas.try_get("inactivos")?,
    })
}

/// Enciende, cambia o apaga la cadencia (0 = apagada). Al encender se borra la
/// marca: la primera lectura ocurre en la siguiente pasada, no una cadencia después.
pub async fn fijar_cadencia(
    pool: &PgPool,
    user_id: Uuid,
    maestro: Maestro,
    segundos: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO bdp_lectura_programada (user_id, maestro, segundos) VALUES ($1, $2, $3) \
         ON CONFLICT (user_id, maestro) DO UPDATE SET segundos = EXCLUDED.segundos, \
             ultima_lectura = CASE WHEN EXCLUDED.segundos > 0 \
                 AND bdp_lectura_programada.segundos = 0 THEN NULL \
                 ELSE bdp_lectura_programada.ultima_lectura END",
    )
    .bind(user_id)
    .bind(maestro.clave())
    .bind(segundos)
    .execute(pool)
    .await?;
    Ok(())
}

/// Lee las cuentas vencidas de un maestro. Un fallo de una cuenta se anota y no
/// corta a las demás. Devuelve cuántas se leyeron bien.
pub async fn sincronizar_vencidas(
    pool: &PgPool,
    modo: &ServicioModoOperacion,
    maestro: Maestro,
) -> usize {
    let usuarios: Vec<Uuid> = match sqlx::query_scalar(
        "SELECT user_id FROM bdp_lectura_programada WHERE maestro = $1 AND segundos > 0",
    )
    .bind(maestro.clave())
    .fetch_all(pool)
    .await
    {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!(
                "[26AA-2] No se pudieron listar cuentas de {}: {e}",
                maestro.clave()
            );
            return 0;
        }
    };
    let mut leidas = 0;
    for user_id in usuarios {
        if !matches!(
            modo.modo_efectivo(pool, user_id).await,
            Ok(ModoEfectivo::Bdp)
        ) {
            continue;
        }
        if !matches!(reclamar(pool, user_id, maestro).await, Ok(true)) {
            continue;
        }
        let Ok(config) = ConfiguracionService::obtener(pool, user_id).await else {
            continue;
        };
        if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
            continue;
        }
        let client = BdpWeblinkClient::new(&config);
        let resultado = match client.login().await {
            Ok(_) => leer(pool, user_id, &client, config.bdp_pos_id, maestro).await,
            Err(e) => Err(format!("login: {e}")),
        };
        match &resultado {
            Ok(_) => leidas += 1,
            Err(e) => tracing::warn!(
                "[26AA-2] Lectura de {} fallida para {user_id}: {e}",
                maestro.clave()
            ),
        }
        anotar(pool, user_id, maestro, &resultado).await;
    }
    leidas
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lee_formas_de_pago_con_ids_decimales() {
        let v = json!({"TenderList": [
            {"Id": 1.0, "Name": " EFECTIVO ", "Type": 1},
            {"TenderId": "2", "Name": "TARJETA"}
        ]});
        let r = registros_de(&v, &["TenderList"], &["Id", "TenderId"]).unwrap();
        assert_eq!(
            r,
            vec![
                RegistroBdp {
                    id: 1,
                    nombre: "EFECTIVO".into(),
                    tipo: Some(1)
                },
                RegistroBdp {
                    id: 2,
                    nombre: "TARJETA".into(),
                    tipo: None
                },
            ]
        );
    }

    #[test]
    fn error_remoto_y_lista_vacia_no_desactivan_nada() {
        let con_error = json!({"Employees": [{"Id": 1}], "ErrorMessage": "sin licencia"});
        assert_eq!(
            registros_de(&con_error, &["Employees"], &["Id"]),
            Err("BDP respondió: sin licencia".to_string())
        );
        let vacia = json!({"Employees": []});
        assert!(registros_de(&vacia, &["Employees"], &["Id"])
            .unwrap_err()
            .contains("vacía"));
        assert!(registros_de(&json!({}), &["Employees"], &["Id"]).is_err());
    }

    #[test]
    fn claves_de_maestro_se_reconocen() {
        for m in Maestro::TODOS {
            assert_eq!(Maestro::desde_clave(m.clave()), Some(m));
        }
        assert_eq!(Maestro::desde_clave("otro"), None);
    }
}
