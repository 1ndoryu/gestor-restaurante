// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use rust_decimal::Decimal;

use super::constantes::{
    DOMINIO_CLIENTE_PUNTOS, DOMINIO_STOCK, ESTADOS_ACTIVOS, ESTADO_DESCARTADO, ESTADO_ERROR,
    ESTADO_PENDIENTE, OPERACION_INVENTARIO, OPERACION_PUNTOS, OPERACION_REGULARIZAR,
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
    /// Marca de la fila al leerla: `marcar_resultado` solo la cierra si sigue
    /// intacta, para no pisar una fusión que haya entrado durante el envío.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Datos de la fila a actualizar tras procesarla. Agrupados para que
/// `marcar_resultado` no tenga más parámetros de los permitidos.
/// La fila se identifica por `id` y por la marca `updated_at` con la que el
/// worker la leyó: claves como `conteo:1` tienen varias filas (historial) y
/// actualizar por clave pisaba las antiguas y las fusiones recién llegadas.
pub struct MarcarResultado<'a> {
    pub fila_id: Uuid,
    pub marca: chrono::DateTime<chrono::Utc>,
    pub estado: &'a str,
    pub error: Option<&'a str>,
    pub incrementar_reintento: bool,
}

/// Identificación de la fila activa de un dominio/entidad/operación.
/// Un dominio puede tener histórico completo, así que el cierre sin id
/// solo puede tocar las filas que siguen activas.
pub struct ClavePendiente<'a> {
    pub user_id: Uuid,
    pub dominio: &'a str,
    pub entidad_id: &'a str,
    pub operacion: &'a str,
}

/// Resultado a anotar en una fila de la cola.
pub struct ResultadoPendiente<'a> {
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

    /// Encola un conteo de inventario FUSIONANDO sus líneas con la fila activa del
    /// mismo almacén. Un conteo es estado absoluto de cada artículo, no del lote:
    /// con `encolar` (que reemplaza) el segundo conteo borraba del pendiente los
    /// artículos que solo venían en el primero y esas líneas nunca llegaban a BDP.
    /// Por artículo manda el conteo más reciente; las líneas del pendiente que no
    /// vengan en el nuevo se conservan porque siguen siendo su último conteo.
    pub async fn fusionar_inventario(
        pool: &PgPool,
        user_id: Uuid,
        entidad_id: &str,
        payload: &Value,
    ) -> Result<(), String> {
        /* Una sola sentencia: fusionar leyendo y escribiendo sin carreras. */
        let actualizadas = sqlx::query(
            "WITH previa AS ( \
                SELECT id, payload_json FROM bdp_push_pendientes \
                WHERE user_id = $1 AND dominio = 'stock' AND entidad_id = $2 \
                  AND operacion = $3 AND estado = ANY($6) \
                FOR UPDATE \
             ), lineas AS ( \
                SELECT elem, 0 AS nivel, ord FROM previa, \
                  jsonb_array_elements(COALESCE($4->'ArticlesList', '[]'::jsonb)) \
                  WITH ORDINALITY AS t(elem, ord) \
                UNION ALL \
                SELECT elem, 1 AS nivel, ord FROM previa, \
                  jsonb_array_elements(COALESCE(payload_json->'ArticlesList', '[]'::jsonb)) \
                  WITH ORDINALITY AS t(elem, ord) \
             ), fusionada AS ( \
                SELECT previa.id, jsonb_set($4, '{ArticlesList}', \
                  COALESCE((SELECT jsonb_agg(elem ORDER BY nivel, ord) FROM ( \
                     SELECT DISTINCT ON (elem->>'Article') elem, nivel, ord \
                     FROM lineas ORDER BY elem->>'Article', nivel, ord \
                  ) elegidas), '[]'::jsonb)) AS payload_json \
                FROM previa \
             ) \
             UPDATE bdp_push_pendientes p \
             SET payload_json = fusionada.payload_json, estado = $5, reintentos = 0, \
                 ultimo_error = NULL, updated_at = NOW() \
             FROM fusionada WHERE p.id = fusionada.id",
        )
        .bind(user_id)
        .bind(entidad_id)
        .bind(OPERACION_INVENTARIO)
        .bind(payload)
        .bind(ESTADO_PENDIENTE)
        .bind(ESTADOS_ACTIVOS)
        .execute(pool)
        .await
        .map_err(|error| format!("No se pudo fusionar el conteo de inventario: {error}"))?;

        /* Sin fila activa previa: primer conteo del almacén, se inserta normal. */
        if actualizadas.rows_affected() > 0 {
            return Ok(());
        }
        Self::encolar(
            pool,
            user_id,
            DOMINIO_STOCK,
            entidad_id,
            OPERACION_INVENTARIO,
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
    /// Cierra la fila que el worker envió. Si mientras se enviaba otra
    /// operación la fusionó (`updated_at` cambió), la fila sigue viva con lo
    /// fusionado y queda `error` con la nota de que el envío anterior se
    /// completó: lo nuevo no se pierde y el operador ve por qué sigue ahí.
    pub async fn marcar_resultado(pool: &PgPool, datos: MarcarResultado<'_>) -> Result<(), String> {
        let MarcarResultado {
            fila_id,
            marca,
            estado,
            error,
            incrementar_reintento,
        } = datos;
        let cerradas = sqlx::query_scalar::<_, Uuid>(
            "UPDATE bdp_push_pendientes \
             SET estado = $3, ultimo_error = $4, updated_at = NOW(), \
                 reintentos = reintentos + CASE WHEN $5 THEN 1 ELSE 0 END \
             WHERE id = $1 AND updated_at = $2 \
             RETURNING id",
        )
        .bind(fila_id)
        .bind(marca)
        .bind(estado)
        .bind(error)
        .bind(incrementar_reintento)
        .fetch_all(pool)
        .await
        .map_err(|error| format!("No se pudo actualizar estado de push: {error}"))?;
        if !cerradas.is_empty() {
            return Ok(());
        }
        /* La fila cambió mientras se enviaba: se deja viva con una nota para que
         * lo fusionado no se pierda y el operador sepa qué pasó. */
        let nota = format!(
            "El envío anterior terminó como {estado}, pero la fila cambió mientras \
             se enviaba; lo nuevo se enviará en el próximo flush"
        );
        sqlx::query(
            "UPDATE bdp_push_pendientes SET estado = $2, ultimo_error = $3, updated_at = NOW() \
             WHERE id = $1",
        )
        .bind(fila_id)
        .bind(ESTADO_ERROR)
        .bind(&nota)
        .execute(pool)
        .await
        .map_err(|error| format!("No se pudo anotar el envío concurrente: {error}"))?;
        Ok(())
    }

    /// Cierre por clave de una fila activa, para los llamadores que no tienen
    /// una fila concreta en la mano (puntos y tests). Solo toca filas activas,
    /// nunca el historial.
    pub async fn marcar_resultado_por_clave(
        pool: &PgPool,
        clave: ClavePendiente<'_>,
        datos: ResultadoPendiente<'_>,
    ) -> Result<(), String> {
        let ClavePendiente {
            user_id,
            dominio,
            entidad_id,
            operacion,
        } = clave;
        let ResultadoPendiente {
            estado,
            error,
            incrementar_reintento,
        } = datos;
        sqlx::query(
            "UPDATE bdp_push_pendientes              SET estado = $6, ultimo_error = $7, updated_at = NOW(),                  reintentos = reintentos + CASE WHEN $8 THEN 1 ELSE 0 END              WHERE user_id = $1 AND dominio = $2 AND entidad_id = $3 AND operacion = $4                AND estado = ANY($5)",
        )
        .bind(user_id)
        .bind(dominio)
        .bind(entidad_id)
        .bind(operacion)
        .bind(ESTADOS_ACTIVOS)
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
            "SELECT id, dominio, entidad_id, operacion, payload_json, estado, reintentos, \
                    updated_at \
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
            "SELECT id, dominio, entidad_id, operacion, payload_json, estado, reintentos, \
                    updated_at \
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

#[cfg(test)]
mod tests_puntos {
    use super::*;
    use serde_json::json;

    /* Fila activa de puntos del cliente: devuelve (PointsAdded, estado). */
    async fn fila_puntos(pool: &PgPool, user_id: Uuid) -> (String, String) {
        sqlx::query_as(
            "SELECT payload_json->>'PointsAdded', estado FROM bdp_push_pendientes \
             WHERE user_id = $1 AND dominio = $2 AND entidad_id = 'cli-1'",
        )
        .bind(user_id)
        .bind(DOMINIO_CLIENTE_PUNTOS)
        .fetch_one(pool)
        .await
        .expect("debe existir una fila de puntos")
    }

    /* [D12c] Dos movimientos seguidos suman en una sola fila; antes el segundo reemplazaba al primero. */
    #[sqlx::test(migrations = "./migrations")]
    async fn acumular_puntos_suma_deltas_sin_perder_el_primero(pool: PgPool) {
        let user_id = Uuid::new_v4();
        let payload = json!({ "PointsAdded": "1", "Reason": "venta 1" });
        BdpPushService::acumular_puntos(&pool, user_id, "cli-1", Decimal::ONE, &payload)
            .await
            .unwrap();
        BdpPushService::acumular_puntos(
            &pool,
            user_id,
            "cli-1",
            Decimal::from(2),
            &json!({ "PointsAdded": "2", "Reason": "venta 2" }),
        )
        .await
        .unwrap();

        let (puntos, estado) = fila_puntos(&pool, user_id).await;
        assert_eq!(Decimal::from_str_exact(&puntos).unwrap(), Decimal::from(3));
        assert_eq!(estado, ESTADO_PENDIENTE);
    }

    /* Neto 0 (+2 y −2) pasa a descartado: no se envía nada a BDP. */
    #[sqlx::test(migrations = "./migrations")]
    async fn acumular_puntos_neto_cero_descarta_la_fila(pool: PgPool) {
        let user_id = Uuid::new_v4();
        BdpPushService::acumular_puntos(
            &pool,
            user_id,
            "cli-1",
            Decimal::from(2),
            &json!({ "PointsAdded": "2" }),
        )
        .await
        .unwrap();
        BdpPushService::acumular_puntos(
            &pool,
            user_id,
            "cli-1",
            Decimal::from(-2),
            &json!({ "PointsAdded": "-2" }),
        )
        .await
        .unwrap();

        let (_, estado) = fila_puntos(&pool, user_id).await;
        assert_eq!(estado, ESTADO_DESCARTADO);
    }
}

#[cfg(test)]
mod tests_inventario {
    use super::*;
    use crate::services::bdp_push::constantes::ESTADO_SINCRONIZADO;
    use serde_json::json;

    /* Líneas del pendiente de inventario del almacén 1: (artículo, unidades). El
     * orden dentro del lote no le importa a BDP, así que se compara ordenado. */
    async fn lineas(pool: &PgPool, user_id: Uuid) -> Vec<(i64, String)> {
        sqlx::query_as(
            "SELECT (elem->>'Article')::bigint, elem->>'Units' FROM bdp_push_pendientes p, \
             jsonb_array_elements(p.payload_json->'ArticlesList') elem \
             WHERE p.user_id = $1 AND p.dominio = $2 AND p.entidad_id = 'conteo:1' ORDER BY 1",
        )
        .bind(user_id)
        .bind(DOMINIO_STOCK)
        .fetch_all(pool)
        .await
        .expect("debe existir el pendiente de inventario")
    }

    fn conteo(almacen: i32, articulos: &[(i64, &str)]) -> Value {
        json!({
            "CodReg": 1,
            "Store": almacen,
            "DateReg": "2026-10-21",
            "ArticlesList": articulos
                .iter()
                .map(|(codigo, unidades)| json!({"Article": codigo, "Units": unidades}))
                .collect::<Vec<_>>(),
        })
    }

    /* [D26h] Dos conteos del mismo almacén con artículos distintos: antes el
     * segundo reemplazaba el lote y el primero nunca llegaba a BDP. */
    #[sqlx::test(migrations = "./migrations")]
    async fn fusionar_inventario_conserva_los_articulos_del_lote_anterior(pool: PgPool) {
        let user_id = Uuid::new_v4();
        BdpPushService::fusionar_inventario(&pool, user_id, "conteo:1", &conteo(1, &[(1001, "2")]))
            .await
            .unwrap();
        BdpPushService::fusionar_inventario(&pool, user_id, "conteo:1", &conteo(1, &[(1002, "5")]))
            .await
            .unwrap();

        assert_eq!(
            lineas(&pool, user_id).await,
            vec![(1001, "2".to_string()), (1002, "5".to_string())]
        );
    }

    /* El mismo artículo contado dos veces: manda el conteo más reciente y no se
     * duplica la línea. */
    #[sqlx::test(migrations = "./migrations")]
    async fn fusionar_inventario_el_conteo_nuevo_gana_por_articulo(pool: PgPool) {
        let user_id = Uuid::new_v4();
        BdpPushService::fusionar_inventario(
            &pool,
            user_id,
            "conteo:1",
            &conteo(1, &[(1001, "2"), (1002, "5")]),
        )
        .await
        .unwrap();
        BdpPushService::fusionar_inventario(&pool, user_id, "conteo:1", &conteo(1, &[(1001, "7")]))
            .await
            .unwrap();

        assert_eq!(
            lineas(&pool, user_id).await,
            vec![(1001, "7".to_string()), (1002, "5".to_string())]
        );
    }

    /* [D26h] El cierre del worker no puede pisar una fusión que entró mientras
     * se enviaba: la fila sigue viva con lo nuevo, marcada para el próximo
     * flush. Antes `marcar_resultado` actualizaba por clave y marcaba
     * `sincronizado` el lote que aún no se había enviado. */
    #[sqlx::test(migrations = "./migrations")]
    async fn el_cierre_por_clave_no_pisa_una_fusion_ni_el_historial(pool: PgPool) {
        let user_id = Uuid::new_v4();
        BdpPushService::fusionar_inventario(&pool, user_id, "conteo:1", &conteo(1, &[(1001, "3")]))
            .await
            .unwrap();
        let enviada = BdpPushService::obtener_pendiente(
            &pool,
            user_id,
            sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM bdp_push_pendientes WHERE entidad_id = 'conteo:1'",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
        )
        .await
        .unwrap()
        .expect("fila enviada");

        /* Llega un conteo nuevo mientras el primero se está enviando. */
        BdpPushService::fusionar_inventario(&pool, user_id, "conteo:1", &conteo(1, &[(1002, "4")]))
            .await
            .unwrap();

        BdpPushService::marcar_resultado(
            &pool,
            MarcarResultado {
                fila_id: enviada.id,
                marca: enviada.updated_at,
                estado: ESTADO_SINCRONIZADO,
                error: None,
                incrementar_reintento: false,
            },
        )
        .await
        .unwrap();

        let (estado, error): (String, Option<String>) =
            sqlx::query_as("SELECT estado, ultimo_error FROM bdp_push_pendientes WHERE id = $1")
                .bind(enviada.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            estado, ESTADO_ERROR,
            "la fila sigue viva para el próximo flush"
        );
        assert!(
            error.unwrap_or_default().contains("cambió mientras"),
            "debe explicar por qué sigue en la cola"
        );
        assert_eq!(
            lineas(&pool, user_id).await,
            vec![(1001, "3".to_string()), (1002, "4".to_string())],
            "lo fusionado no se pierde"
        );
    }

    /* Almacenes distintos no se mezclan: cada uno conserva su lote. */
    #[sqlx::test(migrations = "./migrations")]
    async fn fusionar_inventario_no_mezcla_almacenes(pool: PgPool) {
        let user_id = Uuid::new_v4();
        BdpPushService::fusionar_inventario(&pool, user_id, "conteo:1", &conteo(1, &[(1001, "2")]))
            .await
            .unwrap();
        BdpPushService::fusionar_inventario(&pool, user_id, "conteo:2", &conteo(2, &[(1001, "9")]))
            .await
            .unwrap();

        assert_eq!(
            lineas(&pool, user_id).await,
            vec![(1001, "2".to_string())],
            "el almacén 1 no debe recibir líneas del conteo del almacén 2"
        );
    }
}
