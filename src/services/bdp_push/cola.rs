// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use rust_decimal::Decimal;

use super::constantes::{
    DOMINIO_CLIENTE_PUNTOS, DOMINIO_STOCK, ESTADOS_ACTIVOS, ESTADO_DESCARTADO, ESTADO_PENDIENTE,
    OPERACION_PUNTOS, OPERACION_REGULARIZAR,
};

/// Fila pendiente de push (proyección mínima para el worker).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BdpPushPendiente {
    pub id: Uuid,
    pub dominio: String,
    pub entidad_id: String,
    pub operacion: String,
    #[sqlx(rename = "payload_json")]
    pub payload: Value,
    pub estado: String,
    pub reintentos: i32,
}

/// Datos de la fila a actualizar tras procesarla. Agrupados para que
/// `marcar_resultado` no tenga más parámetros de los permitidos.
pub struct MarcarResultado<'a> {
    pub dominio: &'a str,
    pub entidad_id: &'a str,
    pub operacion: &'a str,
    pub estado: &'a str,
    pub error: Option<&'a str>,
    pub incrementar_reintento: bool,
}

pub struct BdpPushService;

impl BdpPushService {
    /// Encola un ajuste de stock (`regularizar`) SUMANDO su delta a la fila activa
    /// del mismo artículo. Los ajustes son deltas, no estado: con `encolar` (que
    /// reemplaza el payload) dos ajustes +1 y −1 dejaban solo el último. Si el
    /// neto queda en 0, la fila pasa a `descartado` y no se envía nada.
    pub async fn acumular_regularizacion(
        pool: &PgPool,
        user_id: Uuid,
        entidad_id: &str,
        delta: Decimal,
        payload: &Value,
    ) -> Result<(), String> {
        /* Una sola sentencia: leer, sumar y escribir sin carreras entre ajustes. */
        let actualizadas = sqlx::query(
            "WITH f AS ( \
                SELECT id, (payload_json->>'Units')::numeric + $3::text::numeric AS nuevo \
                FROM bdp_push_pendientes \
                WHERE user_id = $1 AND dominio = 'stock' AND entidad_id = $2 \
                  AND operacion = 'regularizar' AND estado = ANY($4) \
             ) \
             UPDATE bdp_push_pendientes p \
             SET payload_json = jsonb_set(p.payload_json, '{Units}', to_jsonb(f.nuevo::text)), \
                 estado = CASE WHEN f.nuevo = 0 THEN $5 ELSE $6 END, \
                 reintentos = 0, ultimo_error = NULL, updated_at = NOW() \
             FROM f WHERE p.id = f.id",
        )
        .bind(user_id)
        .bind(entidad_id)
        .bind(delta.to_string())
        .bind(ESTADOS_ACTIVOS)
        .bind(ESTADO_DESCARTADO)
        .bind(ESTADO_PENDIENTE)
        .execute(pool)
        .await
        .map_err(|error| format!("No se pudo acumular ajuste de stock: {error}"))?;

        /* Sin fila activa previa: primer ajuste del artículo, se inserta normal. */
        if actualizadas.rows_affected() > 0 {
            return Ok(());
        }
        Self::encolar(
            pool,
            user_id,
            DOMINIO_STOCK,
            entidad_id,
            OPERACION_REGULARIZAR,
            payload,
        )
        .await
    }

    /// Encola un movimiento de puntos SUMANDO `PointsAdded` a la fila activa del
    /// mismo cliente. Los puntos son deltas: con `encolar` (que reemplaza) dos
    /// movimientos seguidos dejaban solo el último y BDP perdía puntos (D12c).
    /// El motivo se toma del movimiento más reciente. Neto 0 → `descartado`.
    pub async fn acumular_puntos(
        pool: &PgPool,
        user_id: Uuid,
        entidad_id: &str,
        delta: Decimal,
        payload: &Value,
    ) -> Result<(), String> {
        /* Una sola sentencia: leer, sumar y escribir sin carreras entre movimientos. */
        let actualizadas = sqlx::query(
            "WITH f AS ( \
                SELECT id, (payload_json->>'PointsAdded')::numeric + $3::text::numeric AS nuevo \
                FROM bdp_push_pendientes \
                WHERE user_id = $1 AND dominio = 'cliente_puntos' AND entidad_id = $2 \
                  AND operacion = 'puntos' AND estado = ANY($4) \
             ) \
             UPDATE bdp_push_pendientes p \
             SET payload_json = jsonb_set(\
                     jsonb_set(p.payload_json, '{PointsAdded}', to_jsonb(f.nuevo::text)), \
                     '{Reason}', $7::jsonb), \
                 estado = CASE WHEN f.nuevo = 0 THEN $5 ELSE $6 END, \
                 reintentos = 0, ultimo_error = NULL, updated_at = NOW() \
             FROM f WHERE p.id = f.id",
        )
        .bind(user_id)
        .bind(entidad_id)
        .bind(delta.to_string())
        .bind(ESTADOS_ACTIVOS)
        .bind(ESTADO_DESCARTADO)
        .bind(ESTADO_PENDIENTE)
        .bind(payload.get("Reason").cloned().unwrap_or(Value::Null))
        .execute(pool)
        .await
        .map_err(|error| format!("No se pudo acumular puntos pendientes: {error}"))?;

        /* Sin fila activa previa: primer movimiento del cliente, se inserta normal. */
        if actualizadas.rows_affected() > 0 {
            return Ok(());
        }
        Self::encolar(
            pool,
            user_id,
            DOMINIO_CLIENTE_PUNTOS,
            entidad_id,
            OPERACION_PUNTOS,
            payload,
        )
        .await
    }

    /// Encola (o refresca) una operación pendiente. Upsert sobre la fila activa
    /// (M19): si ya existe una fila activa para la misma entidad+operación, se
    /// actualiza el payload y se reinicia a 'pendiente'; si no, se inserta.
    pub async fn encolar(
        pool: &PgPool,
        user_id: Uuid,
        dominio: &str,
        entidad_id: &str,
        operacion: &str,
        payload: &Value,
    ) -> Result<(), String> {
        let actualizadas = sqlx::query(
            "UPDATE bdp_push_pendientes \
             SET payload_json = $5, estado = $6, reintentos = 0, \
                 ultimo_error = NULL, updated_at = NOW() \
             WHERE user_id = $1 AND dominio = $2 AND entidad_id = $3 \
               AND operacion = $4 AND estado = ANY($7)",
        )
        .bind(user_id)
        .bind(dominio)
        .bind(entidad_id)
        .bind(operacion)
        .bind(payload)
        .bind(ESTADO_PENDIENTE)
        .bind(ESTADOS_ACTIVOS)
        .execute(pool)
        .await
        .map_err(|error| format!("No se pudo refrescar push pendiente: {error}"))?;

        if actualizadas.rows_affected() > 0 {
            return Ok(());
        }

        sqlx::query(
            "INSERT INTO bdp_push_pendientes \
             (id, user_id, dominio, entidad_id, operacion, payload_json) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(dominio)
        .bind(entidad_id)
        .bind(operacion)
        .bind(payload)
        .execute(pool)
        .await
        .map_err(|error| format!("No se pudo encolar push BDP: {error}"))?;
        Ok(())
    }

    /// Transición de estado tras procesar la fila. `incrementar_reintento`
    /// aplica solo a errores transitorios (no a `pendiente_suscripcion`, D2).
    pub async fn marcar_resultado(
        pool: &PgPool,
        user_id: Uuid,
        datos: MarcarResultado<'_>,
    ) -> Result<(), String> {
        let MarcarResultado {
            dominio,
            entidad_id,
            operacion,
            estado,
            error,
            incrementar_reintento,
        } = datos;
        sqlx::query(
            "UPDATE bdp_push_pendientes \
             SET estado = $5, ultimo_error = $6, updated_at = NOW(), \
                 reintentos = reintentos + CASE WHEN $7 THEN 1 ELSE 0 END \
             WHERE user_id = $1 AND dominio = $2 AND entidad_id = $3 AND operacion = $4",
        )
        .bind(user_id)
        .bind(dominio)
        .bind(entidad_id)
        .bind(operacion)
        .bind(estado)
        .bind(error)
        .bind(incrementar_reintento)
        .execute(pool)
        .await
        .map_err(|error| format!("No se pudo actualizar estado de push: {error}"))?;
        Ok(())
    }

    /// Lista las filas activas pendientes. Orden por dependencia de dominio
    /// (M12): departamento y familia antes que artículo, para que el push de un
    /// artículo no falle porque su departamento aún no existe en BDP.
    pub async fn listar_pendientes(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<BdpPushPendiente>, String> {
        let rows: Vec<BdpPushPendiente> = sqlx::query_as::<_, BdpPushPendiente>(
            "SELECT id, dominio, entidad_id, operacion, payload_json, estado, reintentos \
             FROM bdp_push_pendientes \
             WHERE user_id = $1 AND estado = ANY($2) \
             ORDER BY CASE dominio \
                 WHEN 'departamento' THEN 0 \
                 WHEN 'familia' THEN 1 \
                 WHEN 'articulo' THEN 2 \
                 ELSE 3 END, created_at ASC",
        )
        .bind(user_id)
        .bind(ESTADOS_ACTIVOS)
        .fetch_all(pool)
        .await
        .map_err(|error| format!("No se pudo listar push pendientes: {error}"))?;
        Ok(rows)
    }

    /* [208A-2/C4] Visibilidad de la cola (decisión D5): proyección para la UI
     * con estado, reintentos, último error y fechas, sin el payload completo. */
    pub async fn listar_filas(
        pool: &PgPool,
        user_id: Uuid,
        limite: i64,
    ) -> Result<Vec<BdpPushFila>, String> {
        let rows: Vec<BdpPushFila> = sqlx::query_as::<_, BdpPushFila>(
            "SELECT id, dominio, entidad_id, operacion, estado, reintentos, \
                    ultimo_error, updated_at \
             FROM bdp_push_pendientes \
             WHERE user_id = $1 \
             ORDER BY updated_at DESC \
             LIMIT $2",
        )
        .bind(user_id)
        .bind(limite)
        .fetch_all(pool)
        .await
        .map_err(|error| format!("No se pudo listar la cola de sincronización: {error}"))?;
        Ok(rows)
    }

    /// Fila de la cola por id (para reintento individual); `None` si no existe
    /// o no pertenece al usuario.
    pub async fn obtener_fila(
        pool: &PgPool,
        user_id: Uuid,
        fila_id: Uuid,
    ) -> Result<Option<BdpPushFila>, String> {
        sqlx::query_as::<_, BdpPushFila>(
            "SELECT id, dominio, entidad_id, operacion, estado, reintentos, \
                    ultimo_error, updated_at \
             FROM bdp_push_pendientes \
             WHERE id = $1 AND user_id = $2",
        )
        .bind(fila_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| format!("No se pudo leer la fila de sincronización: {error}"))
    }

    /// Fila completa (con payload) por id, para reintentar individualmente.
    pub async fn obtener_pendiente(
        pool: &PgPool,
        user_id: Uuid,
        fila_id: Uuid,
    ) -> Result<Option<BdpPushPendiente>, String> {
        sqlx::query_as::<_, BdpPushPendiente>(
            "SELECT id, dominio, entidad_id, operacion, payload_json, estado, reintentos \
             FROM bdp_push_pendientes \
             WHERE id = $1 AND user_id = $2",
        )
        .bind(fila_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| format!("No se pudo leer la fila de sincronización: {error}"))
    }
}

/// [208A-2/C4] Proyección de una fila de la cola para la UI de Sincronización.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct BdpPushFila {
    pub id: Uuid,
    pub dominio: String,
    pub entidad_id: String,
    pub operacion: String,
    pub estado: String,
    pub reintentos: i32,
    pub ultimo_error: Option<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
