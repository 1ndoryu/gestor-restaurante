// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [094A-3] Repositorio de trabajadores: CRUD + permisos. */

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{PermisoSeccion, Trabajador};

pub struct TrabajadorRepository;

impl TrabajadorRepository {
    pub async fn create(
        pool: &PgPool,
        user_id: Uuid,
        nombre: &str,
        email: &str,
        password_hash: &str,
        cargo: &str,
    ) -> Result<Trabajador, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, Trabajador>(
            "INSERT INTO trabajadores (id, user_id, nombre, email, password_hash, cargo) \
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
        )
        .bind(id)
        .bind(user_id)
        .bind(nombre)
        .bind(email)
        .bind(password_hash)
        .bind(cargo)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Trabajador>, sqlx::Error> {
        sqlx::query_as::<_, Trabajador>("SELECT * FROM trabajadores WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .fetch_optional(pool)
            .await
    }

    /* Login: buscar por email dentro de los trabajadores del propietario */
    pub async fn find_by_email(
        pool: &PgPool,
        email: &str,
    ) -> Result<Option<Trabajador>, sqlx::Error> {
        sqlx::query_as::<_, Trabajador>(
            "SELECT * FROM trabajadores WHERE email = $1 AND activo = true",
        )
        .bind(email)
        .fetch_optional(pool)
        .await
    }

    pub async fn list(pool: &PgPool, user_id: Uuid) -> Result<Vec<Trabajador>, sqlx::Error> {
        sqlx::query_as::<_, Trabajador>(
            "SELECT * FROM trabajadores WHERE user_id = $1 ORDER BY nombre",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        nombre: Option<&str>,
        email: Option<&str>,
        password_hash: Option<&str>,
        cargo: Option<&str>,
        activo: Option<bool>,
    ) -> Result<Option<Trabajador>, sqlx::Error> {
        sqlx::query_as::<_, Trabajador>(
            "UPDATE trabajadores SET \
             nombre = COALESCE($3, nombre), \
             email = COALESCE($4, email), \
             password_hash = COALESCE($5, password_hash), \
             cargo = COALESCE($6, cargo), \
             activo = COALESCE($7, activo), \
             updated_at = NOW() \
             WHERE id = $1 AND user_id = $2 RETURNING *",
        )
        .bind(id)
        .bind(user_id)
        .bind(nombre)
        .bind(email)
        .bind(password_hash)
        .bind(cargo)
        .bind(activo)
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM trabajadores WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /* ========== Permisos ========== */

    pub async fn obtener_permisos(
        pool: &PgPool,
        trabajador_id: Uuid,
    ) -> Result<Vec<PermisoSeccion>, sqlx::Error> {
        sqlx::query_as::<_, PermisoSeccion>(
            "SELECT seccion, permitido FROM permisos_trabajador \
             WHERE trabajador_id = $1 ORDER BY seccion",
        )
        .bind(trabajador_id)
        .fetch_all(pool)
        .await
    }

    /* Obtener lista de secciones permitidas (solo nombres) */
    pub async fn secciones_permitidas(
        pool: &PgPool,
        trabajador_id: Uuid,
    ) -> Result<Vec<String>, sqlx::Error> {
        let rows = sqlx::query_scalar::<_, String>(
            "SELECT seccion FROM permisos_trabajador \
             WHERE trabajador_id = $1 AND permitido = true ORDER BY seccion",
        )
        .bind(trabajador_id)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /* Reemplazar todos los permisos de un trabajador.
     * Borra los existentes e inserta los nuevos. */
    pub async fn set_permisos(
        pool: &PgPool,
        trabajador_id: Uuid,
        secciones: &[String],
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM permisos_trabajador WHERE trabajador_id = $1")
            .bind(trabajador_id)
            .execute(pool)
            .await?;

        for seccion in secciones {
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO permisos_trabajador (id, trabajador_id, seccion, permitido) \
                 VALUES ($1, $2, $3, true)",
            )
            .bind(id)
            .bind(trabajador_id)
            .bind(seccion)
            .execute(pool)
            .await?;
        }

        Ok(())
    }
}
