// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [263A-14] Repositorio del plano de sala: zonas, mesas, combinaciones.
 * Queries para CRUD, batch update posiciones, plano completo, export/import. */

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    ActualizarMesaRequest, CombinacionMesas, CrearMesaRequest, Mesa, ParedSala, ZonaSala,
};

/* [263A-16] Struct auxiliar para las reservas asociadas a mesas (query raw) */
#[derive(Debug, sqlx::FromRow)]
pub struct ReservaMesaRow {
    pub reserva_id: uuid::Uuid,
    pub mesa_id: uuid::Uuid,
    pub hora: chrono::NaiveTime,
    pub nombre_cliente: String,
    pub apellidos_cliente: String,
    pub num_personas: i32,
    pub estado: String,
    pub telefono: String,
}

/* [014A-9] Reservas sin mesa_id vinculado pero con num_mesa (fallback por número) */
#[derive(Debug, sqlx::FromRow)]
pub struct ReservaSinMesaRow {
    pub reserva_id: uuid::Uuid,
    pub num_mesa: i32,
    pub hora: chrono::NaiveTime,
    pub nombre_cliente: String,
    pub apellidos_cliente: String,
    pub num_personas: i32,
    pub estado: String,
    pub telefono: String,
}

pub struct PlanoSalaRepository;

impl PlanoSalaRepository {
    /* ========== Zonas ========== */

    pub async fn listar_zonas(pool: &PgPool, user_id: Uuid) -> Result<Vec<ZonaSala>, sqlx::Error> {
        sqlx::query_as!(
            ZonaSala,
            "SELECT * FROM zonas_sala WHERE user_id = $1 ORDER BY orden, nombre",
            user_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn crear_zona(
        pool: &PgPool,
        user_id: Uuid,
        nombre: &str,
        orden: i32,
        ancho: i32,
        alto: i32,
    ) -> Result<ZonaSala, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            ZonaSala,
            r#"INSERT INTO zonas_sala (id, user_id, nombre, orden, ancho, alto)
               VALUES ($1, $2, $3, $4, $5, $6) RETURNING *"#,
            id,
            user_id,
            nombre,
            orden,
            ancho,
            alto
        )
        .fetch_one(pool)
        .await
    }

    pub async fn actualizar_zona(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        nombre: Option<&str>,
        orden: Option<i32>,
        ancho: Option<i32>,
        alto: Option<i32>,
    ) -> Result<ZonaSala, sqlx::Error> {
        sqlx::query_as!(
            ZonaSala,
            r#"UPDATE zonas_sala SET
                nombre = COALESCE($3, nombre),
                orden = COALESCE($4, orden),
                ancho = COALESCE($5, ancho),
                alto = COALESCE($6, alto),
                updated_at = NOW()
               WHERE id = $1 AND user_id = $2 RETURNING *"#,
            id,
            user_id,
            nombre,
            orden,
            ancho,
            alto
        )
        .fetch_one(pool)
        .await
    }

    pub async fn eliminar_zona(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM zonas_sala WHERE id = $1 AND user_id = $2",
            id,
            user_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /* ========== Mesas ========== */

    pub async fn listar_mesas_zona(pool: &PgPool, zona_id: Uuid) -> Result<Vec<Mesa>, sqlx::Error> {
        sqlx::query_as!(
            Mesa,
            "SELECT * FROM mesas WHERE zona_id = $1 ORDER BY numero",
            zona_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn crear_mesa(pool: &PgPool, req: &CrearMesaRequest) -> Result<Mesa, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            Mesa,
            r#"INSERT INTO mesas (id, zona_id, numero, pos_x, pos_y, ancho, alto, forma, min_personas, max_personas)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING *"#,
            id,
            req.zona_id,
            req.numero,
            req.pos_x.unwrap_or(0),
            req.pos_y.unwrap_or(0),
            req.ancho.unwrap_or(80),
            req.alto.unwrap_or(80),
            req.forma.as_deref().unwrap_or("cuadrada"),
            req.min_personas.unwrap_or(1),
            req.max_personas.unwrap_or(4)
        )
        .fetch_one(pool)
        .await
    }

    pub async fn actualizar_mesa(
        pool: &PgPool,
        id: Uuid,
        req: &ActualizarMesaRequest,
    ) -> Result<Mesa, sqlx::Error> {
        sqlx::query_as!(
            Mesa,
            r#"UPDATE mesas SET
                numero = COALESCE($2, numero),
                pos_x = COALESCE($3, pos_x),
                pos_y = COALESCE($4, pos_y),
                ancho = COALESCE($5, ancho),
                alto = COALESCE($6, alto),
                forma = COALESCE($7, forma),
                min_personas = COALESCE($8, min_personas),
                max_personas = COALESCE($9, max_personas),
                activa = COALESCE($10, activa),
                updated_at = NOW()
               WHERE id = $1 RETURNING *"#,
            id,
            req.numero,
            req.pos_x,
            req.pos_y,
            req.ancho,
            req.alto,
            req.forma.as_deref(),
            req.min_personas,
            req.max_personas,
            req.activa
        )
        .fetch_one(pool)
        .await
    }

    pub async fn eliminar_mesa(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM mesas WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /* Batch update de posiciones — drag-and-drop save */
    pub async fn actualizar_posiciones(
        pool: &PgPool,
        posiciones: &[(Uuid, i32, i32)],
    ) -> Result<u64, sqlx::Error> {
        let mut total = 0u64;
        for (id, x, y) in posiciones {
            let result = sqlx::query!(
                "UPDATE mesas SET pos_x = $2, pos_y = $3, updated_at = NOW() WHERE id = $1",
                id,
                x,
                y
            )
            .execute(pool)
            .await?;
            total += result.rows_affected();
        }
        Ok(total)
    }

    /* ========== Combinaciones ========== */

    pub async fn listar_combinaciones(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<CombinacionMesas>, sqlx::Error> {
        sqlx::query_as!(
            CombinacionMesas,
            "SELECT * FROM combinaciones_mesas WHERE user_id = $1 ORDER BY nombre",
            user_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn mesas_de_combinacion(
        pool: &PgPool,
        combinacion_id: Uuid,
    ) -> Result<Vec<Mesa>, sqlx::Error> {
        sqlx::query_as!(
            Mesa,
            r#"SELECT m.* FROM mesas m
               INNER JOIN combinacion_mesa_items ci ON ci.mesa_id = m.id
               WHERE ci.combinacion_id = $1
               ORDER BY m.numero"#,
            combinacion_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn crear_combinacion(
        pool: &PgPool,
        user_id: Uuid,
        nombre: &str,
        min_personas: i32,
        max_personas: i32,
        mesa_ids: &[Uuid],
    ) -> Result<CombinacionMesas, sqlx::Error> {
        let id = Uuid::new_v4();
        let combo = sqlx::query_as!(
            CombinacionMesas,
            r#"INSERT INTO combinaciones_mesas (id, user_id, nombre, min_personas, max_personas)
               VALUES ($1, $2, $3, $4, $5) RETURNING *"#,
            id,
            user_id,
            nombre,
            min_personas,
            max_personas
        )
        .fetch_one(pool)
        .await?;

        for mesa_id in mesa_ids {
            sqlx::query!(
                "INSERT INTO combinacion_mesa_items (combinacion_id, mesa_id) VALUES ($1, $2)",
                id,
                mesa_id
            )
            .execute(pool)
            .await?;
        }

        Ok(combo)
    }

    pub async fn eliminar_combinacion(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM combinaciones_mesas WHERE id = $1 AND user_id = $2",
            id,
            user_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /* ========== Plano completo ========== */

    pub async fn obtener_mesa(pool: &PgPool, id: Uuid) -> Result<Mesa, sqlx::Error> {
        sqlx::query_as!(Mesa, "SELECT * FROM mesas WHERE id = $1", id)
            .fetch_one(pool)
            .await
    }

    /* [198A-1/D10] Códigos (mesa, salón) para `CallWaiter` de BDP. Usa el número
     * de mesa y el `orden` de su zona como identificador de salón (el plano no
     * guarda el RoomCode BDP; se confirma en F0). Scoped por user_id. */
    pub async fn codigos_mesa_para_bdp(
        pool: &PgPool,
        mesa_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<(i32, i32)>, sqlx::Error> {
        sqlx::query_as::<_, (i32, i32)>(
            "SELECT m.numero, z.orden FROM mesas m \
             INNER JOIN zonas_sala z ON z.id = m.zona_id \
             WHERE m.id = $1 AND z.user_id = $2",
        )
        .bind(mesa_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
    }

    /* Obtiene el nombre de la zona de una mesa */
    pub async fn nombre_zona(pool: &PgPool, zona_id: Uuid) -> Result<String, sqlx::Error> {
        let row = sqlx::query_scalar!("SELECT nombre FROM zonas_sala WHERE id = $1", zona_id)
            .fetch_one(pool)
            .await?;
        Ok(row)
    }

    /* Busca mesa por zona_nombre + numero (para import) */
    pub async fn buscar_mesa_por_zona_numero(
        pool: &PgPool,
        user_id: Uuid,
        zona_nombre: &str,
        numero: i32,
    ) -> Result<Option<Mesa>, sqlx::Error> {
        sqlx::query_as!(
            Mesa,
            r#"SELECT m.* FROM mesas m
               INNER JOIN zonas_sala z ON z.id = m.zona_id
               WHERE z.user_id = $1 AND z.nombre = $2 AND m.numero = $3"#,
            user_id,
            zona_nombre,
            numero
        )
        .fetch_optional(pool)
        .await
    }

    /* Elimina todo el plano de un usuario (para import limpio) */
    pub async fn eliminar_plano_completo(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM combinaciones_mesas WHERE user_id = $1",
            user_id
        )
        .execute(pool)
        .await?;
        sqlx::query!("DELETE FROM zonas_sala WHERE user_id = $1", user_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /* ========== Ocupación (263A-16) ========== */

    /// Reservas del día asociadas a mesas (por `mesa_id`), opcionalmente filtradas por turno.
    pub async fn reservas_por_mesa(
        pool: &PgPool,
        user_id: Uuid,
        fecha: chrono::NaiveDate,
        hora_desde: Option<chrono::NaiveTime>,
        hora_hasta: Option<chrono::NaiveTime>,
    ) -> Result<Vec<ReservaMesaRow>, sqlx::Error> {
        sqlx::query_as!(
            ReservaMesaRow,
            r#"SELECT r.id as reserva_id, r.mesa_id as "mesa_id!", r.hora,
                      r.nombre_cliente, r.apellidos_cliente, r.num_personas,
                      r.estado, r.telefono
               FROM reservas r
               WHERE r.user_id = $1
                 AND r.fecha = $2
                 AND r.mesa_id IS NOT NULL
                 AND r.estado NOT IN ('cancelada')
                 AND ($3::TIME IS NULL OR r.hora >= $3)
                 AND ($4::TIME IS NULL OR r.hora < $4)
               ORDER BY r.hora ASC"#,
            user_id,
            fecha,
            hora_desde,
            hora_hasta
        )
        .fetch_all(pool)
        .await
    }

    /* [014A-9] Reservas sin mesa_id vinculado pero con num_mesa (fallback por número).
     * Permite que mesas nuevas muestren reservas creadas antes de que la mesa existiera. */
    pub async fn reservas_sin_mesa_id(
        pool: &PgPool,
        user_id: Uuid,
        fecha: chrono::NaiveDate,
        hora_desde: Option<chrono::NaiveTime>,
        hora_hasta: Option<chrono::NaiveTime>,
    ) -> Result<Vec<ReservaSinMesaRow>, sqlx::Error> {
        sqlx::query_as!(
            ReservaSinMesaRow,
            r#"SELECT r.id as reserva_id, r.num_mesa as "num_mesa!",
                      r.hora, r.nombre_cliente, r.apellidos_cliente,
                      r.num_personas, r.estado, r.telefono
               FROM reservas r
               WHERE r.user_id = $1
                 AND r.fecha = $2
                 AND r.mesa_id IS NULL
                 AND r.num_mesa IS NOT NULL
                 AND r.estado NOT IN ('cancelada')
                 AND ($3::TIME IS NULL OR r.hora >= $3)
                 AND ($4::TIME IS NULL OR r.hora < $4)
               ORDER BY r.hora ASC"#,
            user_id,
            fecha,
            hora_desde,
            hora_hasta
        )
        .fetch_all(pool)
        .await
    }

    /* ========== Paredes — 094A-7 ========== */

    pub async fn listar_paredes_zona(
        pool: &PgPool,
        zona_id: Uuid,
    ) -> Result<Vec<ParedSala>, sqlx::Error> {
        sqlx::query_as::<_, ParedSala>(
            "SELECT * FROM paredes_sala WHERE zona_id = $1 ORDER BY created_at",
        )
        .bind(zona_id)
        .fetch_all(pool)
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn crear_pared(
        pool: &PgPool,
        zona_id: Uuid,
        pos_x: i32,
        pos_y: i32,
        ancho: i32,
        alto: i32,
        rotacion: i32,
        color: &str,
    ) -> Result<ParedSala, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, ParedSala>(
            "INSERT INTO paredes_sala (id, zona_id, pos_x, pos_y, ancho, alto, rotacion, color) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *",
        )
        .bind(id)
        .bind(zona_id)
        .bind(pos_x)
        .bind(pos_y)
        .bind(ancho)
        .bind(alto)
        .bind(rotacion)
        .bind(color)
        .fetch_one(pool)
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn actualizar_pared(
        pool: &PgPool,
        id: Uuid,
        pos_x: Option<i32>,
        pos_y: Option<i32>,
        ancho: Option<i32>,
        alto: Option<i32>,
        rotacion: Option<i32>,
        color: Option<&str>,
    ) -> Result<ParedSala, sqlx::Error> {
        sqlx::query_as::<_, ParedSala>(
            "UPDATE paredes_sala SET \
             pos_x = COALESCE($2, pos_x), \
             pos_y = COALESCE($3, pos_y), \
             ancho = COALESCE($4, ancho), \
             alto = COALESCE($5, alto), \
             rotacion = COALESCE($6, rotacion), \
             color = COALESCE($7, color), \
             updated_at = NOW() \
             WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(pos_x)
        .bind(pos_y)
        .bind(ancho)
        .bind(alto)
        .bind(rotacion)
        .bind(color)
        .fetch_one(pool)
        .await
    }

    pub async fn eliminar_pared(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM paredes_sala WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn actualizar_posiciones_paredes(
        pool: &PgPool,
        posiciones: &[(Uuid, i32, i32)],
    ) -> Result<u64, sqlx::Error> {
        let mut total = 0u64;
        for (id, x, y) in posiciones {
            let result = sqlx::query(
                "UPDATE paredes_sala SET pos_x = $2, pos_y = $3, updated_at = NOW() WHERE id = $1",
            )
            .bind(id)
            .bind(x)
            .bind(y)
            .execute(pool)
            .await?;
            total += result.rows_affected();
        }
        Ok(total)
    }
}
