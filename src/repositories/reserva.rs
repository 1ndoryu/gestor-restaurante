// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* 253A-5: Repositorio de reservas
263A-6: Filtros turno/estado, num_mesa, apellidos_cliente, resumen mensual
263A-8: Estadísticas de no-shows por canal */

use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{NoShowPorCanal, Reserva, ResumenDiario};

/// Datos para crear una reserva
pub struct NuevaReserva<'a> {
    pub user_id: Uuid,
    pub fecha: chrono::NaiveDate,
    pub hora: chrono::NaiveTime,
    pub nombre_cliente: &'a str,
    pub num_personas: i32,
    pub estado: &'a str,
    pub notas: &'a str,
    pub telefono: &'a str,
    pub num_mesa: Option<i32>,
    pub apellidos_cliente: &'a str,
    pub canal_id: Option<Uuid>,
    pub mesa_id: Option<Uuid>,
}

/// Datos para actualizar parcialmente una reserva
pub struct ActualizarReservaData<'a> {
    pub id: Uuid,
    pub user_id: Uuid,
    pub fecha: Option<chrono::NaiveDate>,
    pub hora: Option<chrono::NaiveTime>,
    pub nombre_cliente: Option<&'a str>,
    pub num_personas: Option<i32>,
    pub estado: Option<&'a str>,
    pub notas: Option<&'a str>,
    pub telefono: Option<&'a str>,
    pub num_mesa: Option<i32>,
    pub apellidos_cliente: Option<&'a str>,
    pub canal_id: Option<Uuid>,
    pub mesa_id: Option<Uuid>,
}

/// Filtros para listar reservas (263A-6)
pub struct FiltrosReserva {
    pub user_id: Uuid,
    pub page: i64,
    pub per_page: i64,
    pub fecha: Option<chrono::NaiveDate>,
    /// [303A-15] Inicio del rango de fechas (inclusive)
    pub fecha_desde: Option<chrono::NaiveDate>,
    /// [303A-15] Fin del rango de fechas (inclusive)
    pub fecha_hasta: Option<chrono::NaiveDate>,
    pub estado: Option<String>,
    pub hora_desde: Option<chrono::NaiveTime>,
    pub hora_hasta: Option<chrono::NaiveTime>,
    pub busqueda: Option<String>,
    /// [044A-8] Columna para ORDER BY (whitelist en `list()`)
    pub sort_by: Option<String>,
    /// [044A-8] Dirección: "asc" o "desc"
    pub sort_order: Option<String>,
}

pub struct ReservaRepository;

impl ReservaRepository {
    pub async fn create(pool: &PgPool, data: &NuevaReserva<'_>) -> Result<Reserva, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            Reserva,
            "INSERT INTO reservas (id, user_id, fecha, hora, nombre_cliente, num_personas, \
             estado, notas, telefono, num_mesa, apellidos_cliente, canal_id, mesa_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) \
             RETURNING *",
            id,
            data.user_id,
            data.fecha,
            data.hora,
            data.nombre_cliente,
            data.num_personas,
            data.estado,
            data.notas,
            data.telefono,
            data.num_mesa,
            data.apellidos_cliente,
            data.canal_id,
            data.mesa_id
        )
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Reserva>, sqlx::Error> {
        sqlx::query_as!(
            Reserva,
            "SELECT * FROM reservas WHERE id = $1 AND user_id = $2",
            id,
            user_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn list(
        pool: &PgPool,
        filtros: &FiltrosReserva,
    ) -> Result<(Vec<Reserva>, i64), sqlx::Error> {
        let offset = (filtros.page - 1) * filtros.per_page;

        /* [283A-28] Búsqueda ILIKE por nombre_cliente/apellidos_cliente */
        let patron = filtros
            .busqueda
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|b| format!("%{b}%"));

        /* [044A-8] ORDER BY dinámico — whitelist para prevenir SQL injection */
        let order_col = match filtros.sort_by.as_deref() {
            Some("hora") => "hora",
            Some("nombre_cliente") => "nombre_cliente",
            Some("num_personas") => "num_personas",
            Some("estado") => "estado",
            Some("num_mesa") => "num_mesa",
            _ => "fecha",
        };
        let order_dir = if filtros.sort_order.as_deref() == Some("asc") {
            "ASC"
        } else {
            "DESC"
        };
        /* Si se ordena por fecha, añadir hora como criterio secundario */
        let secondary = if order_col == "fecha" {
            ", hora ASC"
        } else {
            ""
        };

        /* [303A-4] Cuando no hay filtro explícito de estado, excluir canceladas.
         * [303A-15] Soporte rango de fechas.
         * [044A-8] Convertido a query_as runtime para ORDER BY dinámico.
         * [064A-3] estado soporta multi-valor separado por coma (string_to_array). */
        let query_str = format!(
            "SELECT * FROM reservas WHERE user_id = $1 \
             AND ($4::DATE IS NULL OR fecha = $4) \
             AND ($9::DATE IS NULL OR fecha >= $9) \
             AND ($10::DATE IS NULL OR fecha <= $10) \
             AND (($5::TEXT IS NULL AND estado != 'cancelada') OR estado = ANY(string_to_array($5, ','))) \
             AND ($6::TIME IS NULL OR hora >= $6) \
             AND ($7::TIME IS NULL OR hora < $7) \
             AND ($8::TEXT IS NULL \
                  OR nombre_cliente ILIKE $8 \
                  OR apellidos_cliente ILIKE $8 \
                  OR telefono ILIKE $8) \
             ORDER BY {order_col} {order_dir}{secondary} LIMIT $2 OFFSET $3"
        );

        let items = sqlx::query_as::<_, Reserva>(&query_str)
            .bind(filtros.user_id)
            .bind(filtros.per_page)
            .bind(offset)
            .bind(filtros.fecha)
            .bind(filtros.estado.as_deref())
            .bind(filtros.hora_desde)
            .bind(filtros.hora_hasta)
            .bind(patron.as_deref())
            .bind(filtros.fecha_desde)
            .bind(filtros.fecha_hasta)
            .fetch_all(pool)
            .await?;

        /* [064A-3] Convertido a query_scalar runtime para soportar string_to_array en estado */
        let rec = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT COUNT(*) FROM reservas WHERE user_id = $1 \
             AND ($2::DATE IS NULL OR fecha = $2) \
             AND ($7::DATE IS NULL OR fecha >= $7) \
             AND ($8::DATE IS NULL OR fecha <= $8) \
             AND (($3::TEXT IS NULL AND estado != 'cancelada') OR estado = ANY(string_to_array($3, ','))) \
             AND ($4::TIME IS NULL OR hora >= $4) \
             AND ($5::TIME IS NULL OR hora < $5) \
             AND ($6::TEXT IS NULL \
                  OR nombre_cliente ILIKE $6 \
                  OR apellidos_cliente ILIKE $6 \
                  OR telefono ILIKE $6)",
        )
        .bind(filtros.user_id)
        .bind(filtros.fecha)
        .bind(filtros.estado.as_deref())
        .bind(filtros.hora_desde)
        .bind(filtros.hora_hasta)
        .bind(patron.as_deref())
        .bind(filtros.fecha_desde)
        .bind(filtros.fecha_hasta)
        .fetch_one(pool)
        .await?;

        Ok((items, rec.unwrap_or(0)))
    }

    pub async fn update(
        pool: &PgPool,
        data: &ActualizarReservaData<'_>,
    ) -> Result<Option<Reserva>, sqlx::Error> {
        sqlx::query_as!(
            Reserva,
            "UPDATE reservas SET \
             fecha = COALESCE($3, fecha), \
             hora = COALESCE($4, hora), \
             nombre_cliente = COALESCE($5, nombre_cliente), \
             num_personas = COALESCE($6, num_personas), \
             estado = COALESCE($7, estado), \
             notas = COALESCE($8, notas), \
             telefono = COALESCE($9, telefono), \
             num_mesa = COALESCE($10, num_mesa), \
             apellidos_cliente = COALESCE($11, apellidos_cliente), \
             canal_id = COALESCE($12, canal_id), \
             mesa_id = COALESCE($13, mesa_id), \
             updated_at = NOW() \
             WHERE id = $1 AND user_id = $2 \
             RETURNING *",
            data.id,
            data.user_id,
            data.fecha,
            data.hora,
            data.nombre_cliente,
            data.num_personas,
            data.estado,
            data.notas,
            data.telefono,
            data.num_mesa,
            data.apellidos_cliente,
            data.canal_id,
            data.mesa_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM reservas WHERE id = $1 AND user_id = $2",
            id,
            user_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Conteo de reservas del mes actual y del día actual (Audio 7)
    pub async fn conteo_actual(pool: &PgPool, user_id: Uuid) -> Result<(i64, i64), sqlx::Error> {
        let rec_mes = sqlx::query!(
            "SELECT COUNT(*) as total FROM reservas WHERE user_id = $1 \
             AND fecha >= DATE_TRUNC('month', CURRENT_DATE) \
             AND fecha < DATE_TRUNC('month', CURRENT_DATE) + INTERVAL '1 month' \
             AND estado != 'cancelada'",
            user_id
        )
        .fetch_one(pool)
        .await?;

        let rec_hoy = sqlx::query!(
            "SELECT COUNT(*) as total FROM reservas WHERE user_id = $1 \
             AND fecha = CURRENT_DATE \
             AND estado != 'cancelada'",
            user_id
        )
        .fetch_one(pool)
        .await?;

        Ok((rec_mes.total.unwrap_or(0), rec_hoy.total.unwrap_or(0)))
    }

    /// Resumen diario para un mes completo — vista calendario (263A-7)
    pub async fn resumen_mensual(
        pool: &PgPool,
        user_id: Uuid,
        anio: i32,
        mes: i32,
    ) -> Result<Vec<ResumenDiario>, sqlx::Error> {
        sqlx::query_as!(
            ResumenDiario,
            "SELECT fecha, \
             COUNT(*)::BIGINT as \"total_reservas!\", \
             COALESCE(SUM(num_personas), 0)::BIGINT as \"total_personas!\" \
             FROM reservas \
             WHERE user_id = $1 \
             AND fecha >= make_date($2, $3, 1) \
             AND fecha < (make_date($2, $3, 1) + INTERVAL '1 month') \
             AND estado NOT IN ('cancelada') \
             GROUP BY fecha ORDER BY fecha",
            user_id,
            anio,
            mes
        )
        .fetch_all(pool)
        .await
    }

    /// Totales de no-show en un rango de fechas (263A-8)
    pub async fn no_show_totales(
        pool: &PgPool,
        user_id: Uuid,
        fecha_desde: Option<NaiveDate>,
        fecha_hasta: Option<NaiveDate>,
    ) -> Result<(i64, i64), sqlx::Error> {
        let rec = sqlx::query!(
            "SELECT \
             COUNT(*) FILTER (WHERE estado != 'cancelada')::BIGINT as \"total!\", \
             COUNT(*) FILTER (WHERE estado = 'no_show')::BIGINT as \"no_shows!\" \
             FROM reservas WHERE user_id = $1 \
             AND ($2::DATE IS NULL OR fecha >= $2) \
             AND ($3::DATE IS NULL OR fecha <= $3)",
            user_id,
            fecha_desde,
            fecha_hasta
        )
        .fetch_one(pool)
        .await?;
        Ok((rec.total, rec.no_shows))
    }

    /* [263A-15] Fix 500 en /api/reservas/no-shows:
     * 1. COALESCE(cr.nombre, 'Sin canal') — reservas sin canal_id producían NULL
     *    en el LEFT JOIN, causando "unexpected null; try decoding as an Option".
     * 2. GROUP BY cr.id, cr.nombre — previene mezclar canales con nombre duplicado. */
    /// No-shows desglosados por canal (263A-8)
    pub async fn no_show_por_canal(
        pool: &PgPool,
        user_id: Uuid,
        fecha_desde: Option<NaiveDate>,
        fecha_hasta: Option<NaiveDate>,
    ) -> Result<Vec<NoShowPorCanal>, sqlx::Error> {
        sqlx::query_as!(
            NoShowPorCanal,
            "SELECT \
             COALESCE(cr.nombre, 'Sin canal') as canal_nombre, \
             COUNT(*) FILTER (WHERE r.estado != 'cancelada')::BIGINT as \"total_reservas!\", \
             COUNT(*) FILTER (WHERE r.estado = 'no_show')::BIGINT as \"no_shows!\", \
             CASE WHEN COUNT(*) FILTER (WHERE r.estado != 'cancelada') > 0 \
               THEN ROUND(COUNT(*) FILTER (WHERE r.estado = 'no_show')::NUMERIC * 100.0 \
                    / COUNT(*) FILTER (WHERE r.estado != 'cancelada'), 1) \
               ELSE 0 END::FLOAT8 as \"ratio_porcentaje!\" \
             FROM reservas r \
             LEFT JOIN canales_reserva cr ON cr.id = r.canal_id \
             WHERE r.user_id = $1 \
             AND ($2::DATE IS NULL OR r.fecha >= $2) \
             AND ($3::DATE IS NULL OR r.fecha <= $3) \
             GROUP BY cr.id, cr.nombre \
             ORDER BY \"no_shows!\" DESC",
            user_id,
            fecha_desde,
            fecha_hasta
        )
        .fetch_all(pool)
        .await
    }

    /* [283A-2] Métodos para el chatbot.
     * [014A-11] Convertido a query_as! para verificación SQL en compilación. */

    /// [303A-4] Lista reservas que ocupan sitio para una fecha dada.
    /// Excluye canceladas, completadas y `no_show` — usada para disponibilidad
    /// y resumen del chatbot.
    pub async fn listar_por_fecha(
        pool: &PgPool,
        user_id: Uuid,
        fecha: NaiveDate,
    ) -> Result<Vec<Reserva>, sqlx::Error> {
        sqlx::query_as!(
            Reserva,
            "SELECT * FROM reservas \
             WHERE user_id = $1 AND fecha = $2 \
             AND estado NOT IN ('cancelada', 'completada', 'no_show') \
             ORDER BY hora ASC",
            user_id,
            fecha
        )
        .fetch_all(pool)
        .await
    }

    /// Cancela una reserva (cambia estado a 'cancelada')
    pub async fn cancelar(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            "UPDATE reservas SET estado = 'cancelada', updated_at = NOW() \
             WHERE id = $1 AND user_id = $2",
            id,
            user_id
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
