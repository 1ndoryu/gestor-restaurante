// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* Listado paginado de ventas con filtros y orden en lista blanca. */

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::VentaConCliente;

use super::tipos::VentaRepository;

/* Conteo con los mismos filtros del listado (sin orden ni paginación). */
const SQL_CONTEO_FILTRADO: &str = "SELECT COUNT(*) FROM ventas v \
                 LEFT JOIN clientes c ON c.id = v.cliente_id \
                 WHERE v.user_id = $1 \
                 AND ($2::DATE IS NULL OR v.fecha >= $2) \
                 AND ($3::DATE IS NULL OR v.fecha <= $3) \
                 AND ($4::TEXT IS NULL \
                      OR v.descripcion ILIKE $4 \
                      OR v.turno ILIKE $4 \
                      OR v.canal ILIKE $4 \
                      OR c.nombre ILIKE $4 \
                      OR c.apellidos ILIKE $4 \
                      OR CONCAT(c.nombre, ' ', c.apellidos) ILIKE $4) \
                 AND ($5::TEXT IS NULL OR v.turno = ANY(string_to_array($5, ','))) \
                 AND ($6::TEXT IS NULL OR v.canal = ANY(string_to_array($6, ','))) \
                 AND ($7::TEXT IS NULL OR v.metodo_pago = ANY(string_to_array($7, ','))) \
                 AND ($8::TEXT IS NULL OR \
                      (CASE \
                         WHEN v.haddock_synced = true THEN 'synced' \
                         WHEN v.haddock_sync_error IS NOT NULL THEN 'error' \
                         ELSE 'pending' \
                       END) = ANY(string_to_array($8, ','))) \
                 AND ($9::TEXT IS NULL OR \
                      (CASE \
                         WHEN v.bdp_order_status IS NOT NULL THEN v.bdp_order_status \
                         WHEN v.bdp_synced = true THEN 'synced' \
                         WHEN v.bdp_sync_error IS NOT NULL THEN 'error' \
                         ELSE 'pending' \
                       END) = ANY(string_to_array($9, ',')))";

/* Conteo sin filtros de texto ni columna: solo usuario y rango de fechas. */
const SQL_CONTEO_SIMPLE: &str = "SELECT COUNT(*) FROM ventas WHERE user_id = $1 \
                 AND ($2::DATE IS NULL OR fecha >= $2) \
                 AND ($3::DATE IS NULL OR fecha <= $3)";

/// SQL del listado. `order_col` y `order_dir` vienen de la lista blanca del caller.
fn sql_listado(order_col: &str, order_dir: &str) -> String {
    format!(
            "SELECT v.id, v.user_id, v.fecha, v.comensales, v.descripcion, \
                    v.iva_porcentaje, v.turno, v.canal, v.metodo_pago, \
                    v.importe_base, v.importe_iva, v.reserva_id, v.cliente_id, \
                    CASE WHEN c.id IS NOT NULL \
                         THEN CONCAT(c.nombre, CASE WHEN c.apellidos != '' THEN CONCAT(' ', c.apellidos) ELSE '' END) \
                         ELSE NULL \
                    END AS nombre_cliente, \
                    v.created_at, v.updated_at, \
                    v.haddock_synced, v.haddock_synced_at, v.haddock_sync_error, \
                    v.bdp_synced, v.bdp_synced_at, v.bdp_sync_error, v.bdp_order_id, \
                    v.bdp_order_status, v.bdp_invoiced, \
                    v.anulada, v.anulada_at, v.anulacion_motivo, v.anulacion_usuario, \
                    v.facturada_local, v.factura_numero, v.factura_fecha \
             FROM ventas v \
             LEFT JOIN clientes c ON c.id = v.cliente_id \
             WHERE v.user_id = $1 \
             AND ($4::DATE IS NULL OR v.fecha >= $4) \
             AND ($5::DATE IS NULL OR v.fecha <= $5) \
             AND ($6::TEXT IS NULL \
                  OR v.descripcion ILIKE $6 \
                  OR v.turno ILIKE $6 \
                  OR v.canal ILIKE $6 \
                  OR c.nombre ILIKE $6 \
                  OR c.apellidos ILIKE $6 \
                  OR CONCAT(c.nombre, ' ', c.apellidos) ILIKE $6) \
             AND ($7::TEXT IS NULL OR v.turno = ANY(string_to_array($7, ','))) \
             AND ($8::TEXT IS NULL OR v.canal = ANY(string_to_array($8, ','))) \
             AND ($9::TEXT IS NULL OR v.metodo_pago = ANY(string_to_array($9, ','))) \
             AND ($10::TEXT IS NULL OR \
                  (CASE \
                     WHEN v.haddock_synced = true THEN 'synced' \
                     WHEN v.haddock_sync_error IS NOT NULL THEN 'error' \
                     ELSE 'pending' \
                   END) = ANY(string_to_array($10, ','))) \
             AND ($11::TEXT IS NULL OR \
                  (CASE \
                     WHEN v.bdp_order_status IS NOT NULL THEN v.bdp_order_status \
                     WHEN v.bdp_synced = true THEN 'synced' \
                     WHEN v.bdp_sync_error IS NOT NULL THEN 'error' \
                     ELSE 'pending' \
                   END) = ANY(string_to_array($11, ','))) \
             ORDER BY {order_col} {order_dir}, v.created_at DESC \
             LIMIT $2 OFFSET $3"
    )
}

/// Paginación, filtros y orden del listado de ventas, agrupados para que
/// `list` no tenga más parámetros de los permitidos.
pub struct FiltrosListadoVentas<'a> {
    pub page: i64,
    pub per_page: i64,
    pub desde: Option<chrono::NaiveDate>,
    pub hasta: Option<chrono::NaiveDate>,
    pub busqueda: Option<&'a str>,
    pub turno: Option<&'a str>,
    pub canal: Option<&'a str>,
    pub metodo_pago: Option<&'a str>,
    pub estado_haddock: Option<&'a str>,
    pub estado_bdp: Option<&'a str>,
    pub sort_by: Option<&'a str>,
    pub sort_order: Option<&'a str>,
}

impl VentaRepository {
    /* [044A-8+9] Whitelist de columnas — previene SQL injection.
     * [064A-3] Añadidos filtros por columna: turno, canal, metodo_pago (multi-valor separado por coma).
     * [064A-12] Filtro estado_haddock (synced/error/pending) con CASE en SQL. */
    #[allow(clippy::too_many_lines)]
    pub async fn list(
        pool: &PgPool,
        user_id: Uuid,
        filtros: FiltrosListadoVentas<'_>,
    ) -> Result<(Vec<VentaConCliente>, i64), sqlx::Error> {
        let FiltrosListadoVentas {
            page,
            per_page,
            desde,
            hasta,
            busqueda,
            turno,
            canal,
            metodo_pago,
            estado_haddock,
            estado_bdp,
            sort_by,
            sort_order,
        } = filtros;
        let offset = (page - 1) * per_page;

        /* Whitelist de columnas — previene SQL injection */
        let order_col = match sort_by {
            Some("importe_base") => "v.importe_base",
            Some("turno") => "v.turno",
            Some("canal") => "v.canal",
            Some("metodo_pago") => "v.metodo_pago",
            Some("nombre_cliente") => "nombre_cliente",
            _ => "v.fecha",
        };
        let order_dir = if matches!(sort_order, Some("asc")) {
            "ASC"
        } else {
            "DESC"
        };

        let busqueda_pattern = busqueda.filter(|b| !b.is_empty()).map(|b| format!("%{b}%"));

        /* Normalizar filtros vacíos a None */
        let turno_filter = turno.filter(|t| !t.is_empty());
        let canal_filter = canal.filter(|c| !c.is_empty());
        let metodo_filter = metodo_pago.filter(|m| !m.is_empty());
        let haddock_filter = estado_haddock.filter(|h| !h.is_empty());
        let bdp_filter = estado_bdp.filter(|b| !b.is_empty());

        let query_str = sql_listado(order_col, order_dir);
        let items = sqlx::query_as::<_, VentaConCliente>(&query_str)
            .bind(user_id)
            .bind(per_page)
            .bind(offset)
            .bind(desde)
            .bind(hasta)
            .bind(busqueda_pattern.as_deref())
            .bind(turno_filter)
            .bind(canal_filter)
            .bind(metodo_filter)
            .bind(haddock_filter)
            .bind(bdp_filter)
            .fetch_all(pool)
            .await?;

        /* COUNT con los mismos filtros */
        let has_text_filter = busqueda_pattern.is_some();
        let has_column_filters = turno_filter.is_some()
            || canal_filter.is_some()
            || metodo_filter.is_some()
            || haddock_filter.is_some()
            || bdp_filter.is_some();

        let count = if has_text_filter || has_column_filters {
            let rec = sqlx::query_scalar::<_, Option<i64>>(SQL_CONTEO_FILTRADO)
                .bind(user_id)
                .bind(desde)
                .bind(hasta)
                .bind(busqueda_pattern.as_deref())
                .bind(turno_filter)
                .bind(canal_filter)
                .bind(metodo_filter)
                .bind(haddock_filter)
                .bind(bdp_filter)
                .fetch_one(pool)
                .await?;
            rec.unwrap_or(0)
        } else {
            let rec = sqlx::query_scalar::<_, Option<i64>>(SQL_CONTEO_SIMPLE)
                .bind(user_id)
                .bind(desde)
                .bind(hasta)
                .fetch_one(pool)
                .await?;
            rec.unwrap_or(0)
        };

        Ok((items, count))
    }
}
