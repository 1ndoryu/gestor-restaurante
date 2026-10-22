/* [198A-1/F1] Flush manual de la cola de push Glory -> BDP (D1/D2).
 *
 * POST /api/bdp/push/flush — dispara `BdpPushFlushService::flush` con
 * `forzar_manual=true`. Es el botón "Sincronizar a BDP":
 *   - D1: el botón manual existe siempre (aunque `push_modalidad=automatico`).
 *   - D2: el reintento tras bloqueo por suscripción es SOLO manual; este
 *     endpoint procesa también las filas `pendiente_suscripcion`.
 * [159A-2/F1] Query opcional `?dominios=a&dominios=b` para el botón "Exportar
 * al BDP" por sección (p. ej. solo departamento/familia/articulo). Ausente o
 * vacío = flush global (comportamiento histórico).
 *
 * [208A-2/C4] Visibilidad de la cola (decisión D5):
 *   GET /api/bdp/push/pendientes  — listar filas (estado, reintentos, error).
 *   POST /api/bdp/push/:id/reintentar — reintento individual de una fila.
 *
 * En standalone el worker no envía nada (no-op) y devuelve el resumen con
 * `omitidos_standalone`; el fail-closed se conserva porque cada fila pasa por
 * `armar_push` (arming) → backup → auditoría antes de cualquier HTTP. */

use axum::extract::{Path, RawQuery, State};
use axum::routing::get;
use axum::{Json, Router};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::UserRole;
use crate::services::{BdpPushFila, BdpPushFlushResumen, BdpPushFlushService, BdpPushService};
use crate::AppState;

use super::rate_limit::{limitador_escrituras, limitar};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/bdp/push/flush", axum::routing::post(flush_manual))
        .route("/bdp/push/pendientes", get(listar_pendientes))
        .route(
            "/bdp/push/:id/reintentar",
            axum::routing::post(reintentar_fila),
        )
        /* Tope por IP de las escrituras de este módulo (ver rate_limit.rs). */
        .layer(axum::middleware::from_fn_with_state(
            limitador_escrituras(),
            limitar,
        ))
}

/* [10AA-3-D34] El filtro por dominio se lee a mano de la query.
 *
 * Antes era `Query<FlushQuery>` con `Option<Vec<String>>`, y `serde_urlencoded`
 * no sabe deserializar una clave repetida en una secuencia: toda llamada con
 * `?dominios=propina` respondía 400 «invalid type: string, expected a
 * sequence». El botón «Exportar por sección» del front enviaba exactamente eso,
 * así que estaba roto desde que existe el filtro. Se aceptan las dos formas
 * (`dominios=a&dominios=b` y `dominios=a,b`). */
fn dominios_de_query(cruda: Option<&str>) -> Option<Vec<String>> {
    let mut dominios = Vec::new();
    for (clave, valor) in form_urlencoded_pares(cruda.unwrap_or_default()) {
        /* `dominios[]` es la forma que usan algunos clientes HTTP. */
        if clave != "dominios" && clave != "dominios[]" {
            continue;
        }
        dominios.extend(
            valor
                .split(',')
                .map(str::trim)
                .filter(|dominio| !dominio.is_empty())
                .map(str::to_string),
        );
    }
    (!dominios.is_empty()).then_some(dominios)
}

/// Pares `clave=valor` de una query, decodificados. Vacío si no hay query.
fn form_urlencoded_pares(cruda: &str) -> Vec<(String, String)> {
    cruda
        .split('&')
        .filter(|par| !par.is_empty())
        .map(|par| {
            let (clave, valor) = par.split_once('=').unwrap_or((par, ""));
            (decodificar(clave), decodificar(valor))
        })
        .collect()
}

/// Decodifica `%XX` y `+` (el espacio de una query). Un `%` suelto se deja tal cual.
fn decodificar(texto: &str) -> String {
    let bytes = texto.as_bytes();
    let mut salida = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => salida.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                match u8::from_str_radix(&texto[i + 1..i + 3], 16) {
                    Ok(byte) => {
                        salida.push(byte);
                        i += 3;
                        continue;
                    }
                    /* No es hexadecimal: se copia el `%` literal. */
                    Err(_) => salida.push(b'%'),
                }
            }
            byte => salida.push(byte),
        }
        i += 1;
    }
    /* `from_utf8_lossy` evita un error por un byte roto: el dominio se compara
     * contra constantes ASCII y uno inválido simplemente no casa. */
    String::from_utf8_lossy(&salida).into_owned()
}

pub async fn flush_manual(
    State(state): State<AppState>,
    auth: AuthUser,
    RawQuery(query): RawQuery,
) -> Result<Json<BdpPushFlushResumen>, AppError> {
    /* Escritura BDP: disparar el push (armado + envío) es acción de Admin. */
    auth.require_role(&[UserRole::Admin])?;
    let dominios = dominios_de_query(query.as_deref());
    let resumen = BdpPushFlushService::flush_con_dominios(
        &state.pool,
        auth.user_id,
        true,
        dominios.as_deref(),
    )
    .await
    .map_err(AppError::Internal)?;
    Ok(Json(resumen))
}

#[cfg(test)]
mod tests {
    use super::dominios_de_query;

    /* [10AA-3-D34] La forma que envía el front: clave repetida. */
    #[test]
    fn acepta_clave_repetida() {
        assert_eq!(
            dominios_de_query(Some("dominios=articulo&dominios=stock")),
            Some(vec!["articulo".to_string(), "stock".to_string()])
        );
    }

    /* Y la forma con corchetes o con coma, por comodidad de otros clientes. */
    #[test]
    fn acepta_corchetes_y_coma() {
        assert_eq!(
            dominios_de_query(Some("dominios[]=propina")),
            Some(vec!["propina".to_string()])
        );
        assert_eq!(
            dominios_de_query(Some("dominios=articulo, stock")),
            Some(vec!["articulo".to_string(), "stock".to_string()])
        );
    }

    /* Sin filtro la cola entera se procesa: `None`, nunca una lista vacía. */
    #[test]
    fn sin_dominios_es_none() {
        assert_eq!(dominios_de_query(None), None);
        assert_eq!(dominios_de_query(Some("")), None);
        assert_eq!(dominios_de_query(Some("dominios=")), None);
    }

    #[test]
    fn decodifica_porcentajes() {
        assert_eq!(
            dominios_de_query(Some("dominios=cliente%5Fpuntos")),
            Some(vec!["cliente_puntos".to_string()])
        );
    }
}

/// Listar las filas de la cola de sincronización (visibilidad D5).
pub async fn listar_pendientes(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<BdpPushFila>>, AppError> {
    /* Acción de sincronización BDP: solo Admin (misma política que flush). */
    auth.require_role(&[UserRole::Admin])?;
    let filas = BdpPushService::listar_filas(&state.pool, auth.user_id, 100)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(filas))
}

/// Reintentar individualmente una fila de la cola (decisión D5, regla D2).
pub async fn reintentar_fila(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<BdpPushFlushResumen>, AppError> {
    auth.require_role(&[UserRole::Admin])?;
    let fila = BdpPushService::obtener_fila(&state.pool, auth.user_id, id)
        .await
        .map_err(AppError::Internal)?;
    let Some(fila) = fila else {
        return Err(AppError::NotFound(
            "Fila de sincronización no encontrada".into(),
        ));
    };
    if matches!(fila.estado.as_str(), "sincronizado" | "descartado") {
        return Err(AppError::Validation(
            "La fila ya está sincronizada o descartada".into(),
        ));
    }
    let resumen = BdpPushFlushService::reintentar_uno(&state.pool, auth.user_id, id)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(resumen))
}
