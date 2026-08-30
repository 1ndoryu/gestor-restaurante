// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [094A-4] Repositorio de reseñas — queries dinámicas para SQLX_OFFLINE. */

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{Resena, ResenaAdmin};

pub struct ResenaRepository;

impl ResenaRepository {
    /* Crear solicitud de reseña con token único */
    pub async fn create(
        pool: &PgPool,
        user_id: Uuid,
        reserva_id: Option<Uuid>,
        cliente_id: Option<Uuid>,
        token: &str,
    ) -> Result<Resena, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, Resena>(
            "INSERT INTO resenas (id, user_id, reserva_id, cliente_id, token) \
             VALUES ($1, $2, $3, $4, $5) RETURNING *",
        )
        .bind(id)
        .bind(user_id)
        .bind(reserva_id)
        .bind(cliente_id)
        .bind(token)
        .fetch_one(pool)
        .await
    }

    /* Buscar por token público */
    pub async fn find_by_token(pool: &PgPool, token: &str) -> Result<Option<Resena>, sqlx::Error> {
        sqlx::query_as::<_, Resena>("SELECT * FROM resenas WHERE token = $1")
            .bind(token)
            .fetch_optional(pool)
            .await
    }

    /* Registrar respuesta del cliente */
    pub async fn responder(
        pool: &PgPool,
        token: &str,
        puntuacion: i16,
        comentario: &str,
        redirigido_google: bool,
    ) -> Result<Resena, sqlx::Error> {
        sqlx::query_as::<_, Resena>(
            "UPDATE resenas SET puntuacion = $2, comentario = $3, \
             redirigido_google = $4, respondida_at = NOW() \
             WHERE token = $1 RETURNING *",
        )
        .bind(token)
        .bind(puntuacion)
        .bind(comentario)
        .bind(redirigido_google)
        .fetch_one(pool)
        .await
    }

    /* Listar reseñas del propietario con paginación y filtros */
    #[allow(clippy::too_many_arguments)]
    pub async fn list(
        pool: &PgPool,
        user_id: Uuid,
        page: i64,
        per_page: i64,
        min_puntuacion: Option<i16>,
        max_puntuacion: Option<i16>,
        solo_respondidas: bool,
    ) -> Result<(Vec<ResenaAdmin>, i64), sqlx::Error> {
        let offset = (page - 1) * per_page;

        let rows = sqlx::query_as::<_, ResenaAdmin>(
            "SELECT r.id, r.reserva_id, \
             CASE WHEN c.id IS NOT NULL THEN CONCAT(c.nombre, ' ', c.apellidos) ELSE NULL END AS cliente_nombre, \
             r.puntuacion, r.comentario, r.redirigido_google, r.created_at, r.respondida_at \
             FROM resenas r \
             LEFT JOIN clientes c ON c.id = r.cliente_id \
             WHERE r.user_id = $1 \
             AND ($4::SMALLINT IS NULL OR r.puntuacion >= $4) \
             AND ($5::SMALLINT IS NULL OR r.puntuacion <= $5) \
             AND ($6::BOOLEAN IS FALSE OR r.respondida_at IS NOT NULL) \
             ORDER BY r.created_at DESC \
             LIMIT $2 OFFSET $3",
        )
        .bind(user_id)
        .bind(per_page)
        .bind(offset)
        .bind(min_puntuacion)
        .bind(max_puntuacion)
        .bind(solo_respondidas)
        .fetch_all(pool)
        .await?;

        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::BIGINT FROM resenas r \
             WHERE r.user_id = $1 \
             AND ($2::SMALLINT IS NULL OR r.puntuacion >= $2) \
             AND ($3::SMALLINT IS NULL OR r.puntuacion <= $3) \
             AND ($4::BOOLEAN IS FALSE OR r.respondida_at IS NOT NULL)",
        )
        .bind(user_id)
        .bind(min_puntuacion)
        .bind(max_puntuacion)
        .bind(solo_respondidas)
        .fetch_one(pool)
        .await?;

        Ok((rows, total.0))
    }

    /* ¿Ya se solicitó reseña para esta reserva? */
    pub async fn existe_para_reserva(pool: &PgPool, reserva_id: Uuid) -> Result<bool, sqlx::Error> {
        let row: (bool,) =
            sqlx::query_as("SELECT EXISTS(SELECT 1 FROM resenas WHERE reserva_id = $1)")
                .bind(reserva_id)
                .fetch_one(pool)
                .await?;
        Ok(row.0)
    }
}
