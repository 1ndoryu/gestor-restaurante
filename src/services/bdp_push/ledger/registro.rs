// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [D26g] Escritura del libro de stock (`bdp_stock_ledger`): abrir (S antes de
 * escribir, con el bloqueo por artículo y almacén), cerrar y barrer huérfanas. */

use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

/// Fila que se abre en el libro. `id_local` la identifica dentro del usuario.
pub struct FilaLibro {
    pub id_local: String,
    pub articulo: i64,
    pub almacen: i32,
    pub almacen_destino: Option<i32>,
    pub variacion: Decimal,
    /// S; `None` si la lectura previa falló (entonces no se podrá reconciliar).
    pub stock_previo: Option<Decimal>,
}

/// Filas del libro que sirven para reconciliar: qué artículo, en qué almacén y
/// qué se esperaba ver.
pub struct FilaPendiente {
    pub id_local: String,
    pub articulo: i64,
    pub almacen: i32,
    pub stock_previo: Option<Decimal>,
    pub variacion: Decimal,
}

impl FilaPendiente {
    /// Lo que el libro guarda de una fila abierta es justo lo que hace falta
    /// para reconciliarla; no hay que volver a leerla.
    pub fn desde(fila: FilaLibro) -> Self {
        Self {
            id_local: fila.id_local,
            articulo: fila.articulo,
            almacen: fila.almacen,
            stock_previo: fila.stock_previo,
            variacion: fila.variacion,
        }
    }
}

/// Minutos tras los que una fila `pendiente` se da por abandonada. Ningún envío
/// dura tanto (el timeout mayor del cliente es de 60 s): si sigue pendiente, el
/// proceso murió entre abrir y cerrar el libro.
const HORAS_HUERFANA: i32 = 2;

/// Abre en una transacción todas las filas del movimiento. Si el par artículo y
/// almacén ya tiene una operación pendiente, `uq_bdp_stock_ledger_bloqueo`
/// aborta la transacción entera: no se envía nada y no queda media operación.
pub async fn abrir(
    pool: &PgPool,
    user_id: Uuid,
    operacion: &str,
    filas: &[FilaLibro],
    documento_id: Uuid,
) -> Result<(), String> {
    liberar_huerfanas(pool, user_id).await?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| format!("No se pudo abrir el libro de stock: {error}"))?;
    for fila in filas {
        let esperado = fila.stock_previo.map(|previo| previo + fila.variacion);
        sqlx::query(
            "INSERT INTO bdp_stock_ledger \
             (id, user_id, articulo_codigo, almacen, almacen_destino, operacion, variacion, \
              stock_previo, stock_esperado, id_local, origen, documento_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'web', $11)",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(fila.articulo.to_string())
        .bind(fila.almacen)
        .bind(fila.almacen_destino)
        .bind(operacion)
        .bind(fila.variacion)
        .bind(fila.stock_previo)
        .bind(esperado)
        .bind(&fila.id_local)
        .bind(documento_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|error| conflicto(&error, fila))?;
    }
    tx.commit()
        .await
        .map_err(|error| format!("No se pudo abrir el libro de stock: {error}"))
}

fn conflicto(error: &sqlx::Error, fila: &FilaLibro) -> String {
    let bloqueo = error
        .as_database_error()
        .and_then(|db| db.constraint())
        .is_some_and(|nombre| nombre == "uq_bdp_stock_ledger_bloqueo");
    if bloqueo {
        return format!(
            "Hay otra operación de stock pendiente del artículo {} en el almacén {}: \
             no se envía hasta que se resuelva",
            fila.articulo, fila.almacen
        );
    }
    format!("No se pudo abrir el libro de stock: {error}")
}

/// Cierra una fila pendiente. Solo toca las que siguen `pendiente`: un cierre
/// repetido no pisa un estado final.
pub async fn cerrar(
    pool: &PgPool,
    user_id: Uuid,
    id_local: &str,
    estado: &str,
    nota: &str,
    stock_posterior: Option<Decimal>,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE bdp_stock_ledger \
         SET estado = $3, nota = $4, stock_posterior = $5, confirmado_en = NOW() \
         WHERE user_id = $1 AND id_local = $2 AND estado = 'pendiente'",
    )
    .bind(user_id)
    .bind(id_local)
    .bind(estado)
    .bind(nota)
    .bind(stock_posterior)
    .execute(pool)
    .await
    .map_err(|error| format!("No se pudo cerrar el libro de stock: {error}"))?;
    Ok(())
}

/// Igual que `cerrar`, dentro de la transacción de una revisión manual: el
/// dictamen tiene que caer junto con el cierre de la auditoría y de la cola.
pub async fn cerrar_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    id_local: &str,
    estado: &str,
    nota: &str,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE bdp_stock_ledger \
         SET estado = $3, nota = $4, confirmado_en = NOW() \
         WHERE user_id = $1 AND id_local = $2 AND estado = 'requiere_revision'",
    )
    .bind(user_id)
    .bind(id_local)
    .bind(estado)
    .bind(nota)
    .execute(&mut **tx)
    .await
    .map_err(|error| format!("No se pudo cerrar el libro de stock: {error}"))?;
    Ok(())
}

/// Intentos de este envío que ya cerraron sin movimiento. Sirve para que al
/// moverse el stock solo se reintente una vez (D26g).
pub async fn intentos_sin_movimiento(
    pool: &PgPool,
    user_id: Uuid,
    documento_id: Uuid,
) -> Result<i64, String> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM bdp_stock_ledger \
         WHERE user_id = $1 AND documento_id = $2 AND estado = 'sin_movimiento'",
    )
    .bind(user_id)
    .bind(documento_id.to_string())
    .fetch_one(pool)
    .await
    .map_err(|error| format!("No se pudo contar el libro de stock: {error}"))
}

/// Las pendientes de más de dos horas pasan a revisión: si no, un proceso que
/// murió a mitad bloquearía ese artículo y almacén para siempre.
async fn liberar_huerfanas(pool: &PgPool, user_id: Uuid) -> Result<(), String> {
    sqlx::query(
        "UPDATE bdp_stock_ledger SET estado = 'requiere_revision', confirmado_en = NOW(), \
         nota = COALESCE(nota, 'operación abandonada: el proceso murió con el libro abierto') \
         WHERE user_id = $1 AND estado = 'pendiente' \
           AND creado_en < NOW() - make_interval(hours => $2)",
    )
    .bind(user_id)
    .bind(HORAS_HUERFANA)
    .execute(pool)
    .await
    .map_err(|error| format!("No se pudo liberar el libro de stock: {error}"))?;
    Ok(())
}
