// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [D26g] Revisión manual de una operación de stock ambigua.
 *
 * Cuando la relectura de `GetStock` no permite atribuir el movimiento (otro
 * valor, lectura fallida o el reintento ya gastado), la auditoría se queda
 * `ambiguo` y el guard de escritura bloquea ese artículo para siempre: no hay
 * forma de reintentar ni de dejarlo de intentar. Esta pieza cierra ese
 * circuito. La persona que ha mirado el BDP dice qué pasó y aquí se traduce a
 * una decisión: cerrar la operación como aplicada (no se reenvía) o
 * reencolar el envío con el contador a cero.
 *
 * La auditoría se cierra como `exito` o `error`, nunca se borra: el guard lee
 * `pendiente`/`ambiguo` y deja de bloquear en cuanto hay dictamen. */

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::super::constantes::{
    DOMINIO_STOCK, ESTADO_ERROR, ESTADO_PENDIENTE, ESTADO_SINCRONIZADO, OPERACION_INVENTARIO,
};
use crate::services::bdp_write_guard::entidad_uuid;

/// Una fila del libro esperando dictamen, tal como la enseña la pantalla.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct FilaRevision {
    pub id_local: String,
    pub articulo_codigo: String,
    pub almacen: i32,
    pub operacion: String,
    pub variacion: rust_decimal::Decimal,
    pub stock_previo: Option<rust_decimal::Decimal>,
    pub stock_esperado: Option<rust_decimal::Decimal>,
    pub stock_posterior: Option<rust_decimal::Decimal>,
    pub nota: Option<String>,
    pub documento_id: Option<String>,
    pub creado_en: chrono::DateTime<chrono::Utc>,
    /// Estado actual de la fila de la cola que la originó (`None` si ya no
    /// existe): es lo que se va a mover al decidir.
    pub estado_cola: Option<String>,
}

/// Qué ha dicho la persona tras mirar el BDP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// BDP ya tiene el movimiento: se cierra como sincronizado, sin reenviar.
    Aplicado,
    /// No está: el envío vuelve a la cola con los reintentos a cero.
    NoAplicado,
}

impl Decision {
    /// Texto del cuerpo del endpoint. Cualquier otro valor es un error de
    /// validación, no un «no aplicado» silencioso.
    pub fn desde_texto(texto: &str) -> Result<Self, String> {
        match texto.trim() {
            "aplicado" => Ok(Self::Aplicado),
            "no_aplicado" => Ok(Self::NoAplicado),
            otro => Err(format!(
                "decisión desconocida: {otro} (admite 'aplicado' o 'no_aplicado')"
            )),
        }
    }
}

/// Resultado de la decisión, para contestar al front con qué se hizo.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Resolucion {
    pub id_local: String,
    pub aplicado: bool,
    /// La fila de la cola quedó así: `sincronizado` o `pendiente`.
    pub estado_cola: String,
}

/// Filas de stock que esperan dictamen humano, la más reciente primero.
pub async fn listar(pool: &PgPool, user_id: Uuid) -> Result<Vec<FilaRevision>, String> {
    sqlx::query_as::<_, FilaRevision>(
        "SELECT l.id_local, l.articulo_codigo, l.almacen, l.operacion, l.variacion, \
                l.stock_previo, l.stock_esperado, l.stock_posterior, l.nota, \
                l.documento_id, l.creado_en, p.estado AS estado_cola \
         FROM bdp_stock_ledger l \
         LEFT JOIN bdp_push_pendientes p \
                ON p.id::text = l.documento_id AND p.user_id = l.user_id \
         WHERE l.user_id = $1 AND l.estado = 'requiere_revision' \
         ORDER BY l.creado_en DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|error| format!("No se pudo leer el libro de stock: {error}"))
}

/// Registra el dictamen de una fila en revisión: cierra la auditoría de la
/// entidad (lo que desbloquea el guard) y deja la cola lista para lo que
/// toque. Todo en una transacción: o queda cerrado, o no se toca nada.
pub async fn resolver(
    pool: &PgPool,
    user_id: Uuid,
    id_local: &str,
    decision: Decision,
    motivo: &str,
) -> Result<Resolucion, String> {
    let motivo = motivo.trim();
    /* Un dictamen sin motivo no se puede auditar después: la pantalla lo exige
     * y aquí no se acepta en blanco. */
    if motivo.chars().count() < 5 {
        return Err("el motivo de la revisión debe tener al menos 5 caracteres".into());
    }
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| format!("No se pudo abrir la revisión de stock: {error}"))?;
    let revision = fila_en_revision(&mut tx, user_id, id_local).await?;

    let nueva_nota = format!("revisión manual: {motivo}");
    /* La auditoría primero: si falla, la transacción no deja un libro cerrado
     * con el guard aún bloqueando. */
    cerrar_auditorias(&mut tx, user_id, &revision, decision, &nueva_nota).await?;
    /* La fila del libro sale del estado de revisión con el dictamen: si no
     * estaba aplicada, `rechazado` deja claro que ese movimiento no salió. */
    let estado_libro = match decision {
        Decision::Aplicado => "confirmado",
        Decision::NoAplicado => "rechazado",
    };
    super::registro::cerrar_tx(&mut tx, user_id, id_local, estado_libro, &nueva_nota).await?;
    let estado_cola = mover_cola(&mut tx, user_id, &revision, decision, &nueva_nota).await?;
    tx.commit()
        .await
        .map_err(|error| format!("No se pudo confirmar la revisión de stock: {error}"))?;
    tracing::info!(
        "[D26g] revisión manual de stock {id_local} ({}/{}): {decision:?}, cola {estado_cola}",
        revision.articulo,
        revision.almacen
    );
    Ok(Resolucion {
        id_local: id_local.to_string(),
        aplicado: decision == Decision::Aplicado,
        estado_cola,
    })
}

/// La fila de la cola que originó la operación. Si el dictamen es «aplicado»,
/// la parte que ya consta en BDP se descuenta del acumulado: la fila puede
/// haber sumado más ajustes del mismo artículo mientras esperaba dictamen, y
/// cerrarla entera perdería esos sin enviar.
async fn mover_cola(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    revision: &Revision,
    decision: Decision,
    nota: &str,
) -> Result<String, String> {
    let Some(id) = revision.documento_id else {
        /* Sin fila de cola (p. ej. la purgó el barrido) no hay nada que mover:
         * el dictamen del libro ya destrabó el artículo. */
        return Ok("sin_fila".to_string());
    };
    if decision == Decision::NoAplicado {
        /* No consta en BDP: vuelve a la cola con los reintentos a cero. Es un
         * envío nuevo autorizado por una persona, no un reintento automático. */
        sqlx::query(
            "UPDATE bdp_push_pendientes \
             SET estado = $3, ultimo_error = $4, updated_at = NOW(), reintentos = 0 \
             WHERE id = $1 AND user_id = $2 AND dominio = $5",
        )
        .bind(id)
        .bind(user_id)
        .bind(ESTADO_PENDIENTE)
        .bind(nota)
        .bind(DOMINIO_STOCK)
        .execute(&mut **tx)
        .await
        .map_err(|error| format!("No se pudo reencolar la operación de stock: {error}"))?;
        return Ok(ESTADO_PENDIENTE.to_string());
    }
    /* Un inventario es un valor absoluto, no un delta acumulable: si consta, la
     * fila entera está hecha. */
    if revision.operacion == self::OPERACION_INVENTARIO {
        cerrar_cola(tx, user_id, id, nota).await?;
        return Ok(ESTADO_SINCRONIZADO.to_string());
    }
    let acumulado: Option<String> = sqlx::query_scalar(
        "SELECT payload_json ->> 'Units' FROM bdp_push_pendientes \
         WHERE id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| format!("No se pudo leer el ajuste pendiente: {error}"))?;
    let restante = acumulado
        .and_then(|texto| texto.parse::<rust_decimal::Decimal>().ok())
        .map(|unidades| unidades - revision.variacion)
        .unwrap_or_default();
    if restante.is_zero() {
        cerrar_cola(tx, user_id, id, nota).await?;
        return Ok(ESTADO_SINCRONIZADO.to_string());
    }
    /* Queda algo sin enviar: la fila sigue viva con el resto, y su nota dice
     * cuánto ya consta. */
    let nota_resto = format!(
        "{nota}; quedan {restante} sin enviar del artículo {}",
        revision.articulo
    );
    sqlx::query(
        "UPDATE bdp_push_pendientes \
         SET payload_json = jsonb_set(payload_json, '{Units}', to_jsonb($3::text)), \
             estado = $4, ultimo_error = $5, updated_at = NOW(), reintentos = 0 \
         WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .bind(restante.to_string())
    .bind(ESTADO_PENDIENTE)
    .bind(&nota_resto)
    .execute(&mut **tx)
    .await
    .map_err(|error| format!("No se pudo descontar lo aplicado del ajuste: {error}"))?;
    Ok(ESTADO_PENDIENTE.to_string())
}

async fn cerrar_cola(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    id: Uuid,
    nota: &str,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE bdp_push_pendientes \
         SET estado = $3, ultimo_error = $4, updated_at = NOW() \
         WHERE id = $1 AND user_id = $2 AND dominio = $5",
    )
    .bind(id)
    .bind(user_id)
    .bind(ESTADO_SINCRONIZADO)
    .bind(nota)
    .bind(DOMINIO_STOCK)
    .execute(&mut **tx)
    .await
    .map_err(|error| format!("No se pudo cerrar la operación de stock: {error}"))?;
    Ok(())
}

/// Lo que hace falta para cerrar una fila en revisión: el par artículo y
/// almacén (para la nota), la variación aplicada (lo que hay que descontar de
/// la cola si el dictamen es «aplicado») y, sobre todo, la entidad de la cola,
/// que es la que dio su UUID a la auditoría y la que el guard compara.
struct Revision {
    articulo: String,
    almacen: i32,
    operacion: String,
    variacion: rust_decimal::Decimal,
    documento_id: Option<Uuid>,
    entidad_id: Option<String>,
}

/// Localiza la fila en revisión y su fila de cola, y bloquea el libro para que
/// dos revisiones simultáneas no se pisen.
/// Columnas del libro que hacen falta para cerrar la revisión, tal cual salen
/// del SELECT: artículo, almacén, operación, variación, documento y entidad.
type FilaEnRevision = (
    String,
    i32,
    String,
    rust_decimal::Decimal,
    Option<String>,
    Option<String>,
);

async fn fila_en_revision(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    id_local: &str,
) -> Result<Revision, String> {
    let fila: Option<FilaEnRevision> = sqlx::query_as(
        "SELECT l.articulo_codigo, l.almacen, l.operacion, l.variacion, l.documento_id, \
                p.entidad_id \
         FROM bdp_stock_ledger l \
         LEFT JOIN bdp_push_pendientes p \
                ON p.id::text = l.documento_id AND p.user_id = l.user_id \
         WHERE l.user_id = $1 AND l.id_local = $2 AND l.estado = 'requiere_revision' \
         FOR UPDATE OF l",
    )
    .bind(user_id)
    .bind(id_local)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| format!("No se pudo leer el libro de stock: {error}"))?;
    let (articulo, almacen, operacion, variacion, documento, entidad_id) =
        fila.ok_or_else(|| format!("No hay una operación de stock en revisión con id {id_local}"))?;
    Ok(Revision {
        articulo,
        almacen,
        operacion,
        variacion,
        documento_id: documento.and_then(|texto| Uuid::parse_str(&texto).ok()),
        entidad_id,
    })
}

/// Cierra las auditorías ambiguas de esa entidad de stock: es lo que hace que
/// `comprobar_sin_pendiente` deje pasar la próxima escritura. La identidad es
/// la de la fila de la cola (`entidad_uuid` del dominio de stock), no la del
/// payload de BDP, que solo lleva los campos del `WebLink`.
async fn cerrar_auditorias(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    revision: &Revision,
    decision: Decision,
    nota: &str,
) -> Result<(), String> {
    let Some(entidad_id) = &revision.entidad_id else {
        return Err(format!(
            "La fila de la cola de la operación de stock {}/{} ya no existe: \
             la auditoría no se puede cerrar desde aquí",
            revision.articulo, revision.almacen
        ));
    };
    let resultado = match decision {
        Decision::Aplicado => "exito",
        Decision::NoAplicado => ESTADO_ERROR,
    };
    let cerradas = sqlx::query(
        "UPDATE bdp_audit_log \
         SET resultado = $2, error_mensaje = $3, updated_at = NOW() \
         WHERE user_id = $1 AND resultado IN ('pendiente', 'ambiguo') \
           AND target_entity_type = $4 AND target_entity_id = $5",
    )
    .bind(user_id)
    .bind(resultado)
    .bind(nota)
    .bind(DOMINIO_STOCK)
    .bind(entidad_uuid(DOMINIO_STOCK, entidad_id))
    .execute(&mut **tx)
    .await
    .map_err(|error| format!("No se pudo cerrar la auditoría BDP: {error}"))?;
    /* Cero auditorías cerradas es un fallo de coherencia: el libro dice que hay
     * que revisar y la auditoría ya no está bloqueando. Se para para que no
     * parezca que la revisión surtió efecto. */
    if cerradas.rows_affected() == 0 {
        return Err(format!(
            "La auditoría de la operación de stock {} no estaba pendiente: revisa la cola",
            revision.articulo
        ));
    }
    Ok(())
}

/// [D-auto-push] Auditorías de stock que ninguna revisión automática puede
/// cerrar: no existe fila en el libro (el envío falló antes de abrirlo) ni
/// operación del poller que las cubra, así que mientras estén `pendiente` o
/// `ambiguo` ese artículo y almacén no admiten otro movimiento.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct AuditoriaBloqueada {
    pub id: Uuid,
    pub operacion: String,
    pub resultado: String,
    pub error_mensaje: Option<String>,
    pub creada_en: chrono::DateTime<chrono::Utc>,
}

/// Las que bloquean, la más reciente primero.
pub async fn listar_auditorias_bloqueadas(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<AuditoriaBloqueada>, String> {
    sqlx::query_as::<_, AuditoriaBloqueada>(
        "SELECT id, operacion, resultado, error_mensaje, created_at AS creada_en \
         FROM bdp_audit_log \
         WHERE user_id = $1 AND target_entity_type = $2 \
           AND resultado IN ('pendiente', 'ambiguo') \
         ORDER BY created_at DESC",
    )
    .bind(user_id)
    .bind(DOMINIO_STOCK)
    .fetch_all(pool)
    .await
    .map_err(|error| format!("No se pudo leer las auditorías de stock bloqueadas: {error}"))
}

/// Cierra una auditoría de stock que el restaurante da por no aplicada. El
/// motivo es obligatorio: sin él, dentro de un mes nadie sabe por qué se
/// destrabó un artículo. No se borra la fila, se cierra como `error`.
pub async fn descartar_auditoria(
    pool: &PgPool,
    user_id: Uuid,
    audit_id: Uuid,
    motivo: &str,
) -> Result<(), String> {
    let motivo = motivo.trim();
    if motivo.chars().count() < 5 {
        return Err("el motivo del descarte debe tener al menos 5 caracteres".into());
    }
    let nota = format!("descartada por el restaurante: {motivo}");
    /* El filtro por estado va en el UPDATE: si otra revisión la cerró entre la
     * lectura y esto, no se pisa su dictamen. */
    let cerradas = sqlx::query(
        "UPDATE bdp_audit_log \
         SET resultado = 'error', error_mensaje = $3, updated_at = NOW() \
         WHERE id = $1 AND user_id = $2 AND target_entity_type = $4 \
           AND resultado IN ('pendiente', 'ambiguo')",
    )
    .bind(audit_id)
    .bind(user_id)
    .bind(&nota)
    .bind(DOMINIO_STOCK)
    .execute(pool)
    .await
    .map_err(|error| format!("No se pudo descartar la auditoría de stock: {error}"))?;
    if cerradas.rows_affected() == 0 {
        return Err("Esa auditoría ya no estaba bloqueando: recarga la lista".into());
    }
    tracing::info!("[D-auto-push] auditoría de stock {audit_id} descartada: {motivo}");
    Ok(())
}
