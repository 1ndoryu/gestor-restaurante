// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [094A-5] Repositorio de reglas de inactividad + consulta de clientes inactivos. */

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::ReglaInactividad;

/* Datos mínimos para el envío automatizado */
#[derive(Debug, sqlx::FromRow)]
pub struct ClienteInactivo {
    pub cliente_id: Uuid,
    pub user_id: Uuid,
    pub nombre: String,
    pub email: String,
    pub telefono: String,
    pub regla_id: Uuid,
    pub canal: String,
    pub mensaje_plantilla: String,
}

pub struct InactividadRepository;

impl InactividadRepository {
    pub async fn create(
        pool: &PgPool,
        user_id: Uuid,
        nombre: &str,
        dias: i32,
        canal: &str,
        mensaje: &str,
    ) -> Result<ReglaInactividad, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, ReglaInactividad>(
            "INSERT INTO reglas_inactividad (id, user_id, nombre, dias_inactividad, canal, mensaje_plantilla) \
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
        )
        .bind(id)
        .bind(user_id)
        .bind(nombre)
        .bind(dias)
        .bind(canal)
        .bind(mensaje)
        .fetch_one(pool)
        .await
    }

    pub async fn list(pool: &PgPool, user_id: Uuid) -> Result<Vec<ReglaInactividad>, sqlx::Error> {
        sqlx::query_as::<_, ReglaInactividad>(
            "SELECT * FROM reglas_inactividad WHERE user_id = $1 ORDER BY dias_inactividad",
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
        dias: Option<i32>,
        canal: Option<&str>,
        mensaje: Option<&str>,
        activa: Option<bool>,
    ) -> Result<Option<ReglaInactividad>, sqlx::Error> {
        sqlx::query_as::<_, ReglaInactividad>(
            "UPDATE reglas_inactividad SET \
             nombre = COALESCE($3, nombre), \
             dias_inactividad = COALESCE($4, dias_inactividad), \
             canal = COALESCE($5, canal), \
             mensaje_plantilla = COALESCE($6, mensaje_plantilla), \
             activa = COALESCE($7, activa), \
             updated_at = NOW() \
             WHERE id = $1 AND user_id = $2 RETURNING *",
        )
        .bind(id)
        .bind(user_id)
        .bind(nombre)
        .bind(dias)
        .bind(canal)
        .bind(mensaje)
        .bind(activa)
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let r = sqlx::query("DELETE FROM reglas_inactividad WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(r.rows_affected() > 0)
    }

    /* Buscar clientes que cumplen alguna regla de inactividad activa
     * y que NO hayan recibido ya un envío para esa regla.
     * Recorre TODAS las reglas de TODOS los usuarios activos. */
    pub async fn clientes_inactivos_pendientes(
        pool: &PgPool,
    ) -> Result<Vec<ClienteInactivo>, sqlx::Error> {
        sqlx::query_as::<_, ClienteInactivo>(
            "SELECT c.id AS cliente_id, c.user_id, c.nombre, c.email, c.telefono, \
             r.id AS regla_id, r.canal, r.mensaje_plantilla \
             FROM reglas_inactividad r \
             JOIN clientes c ON c.user_id = r.user_id \
             WHERE r.activa = TRUE \
             AND c.ultima_visita IS NOT NULL \
             AND c.ultima_visita < NOW() - (r.dias_inactividad || ' days')::INTERVAL \
             AND NOT EXISTS ( \
                 SELECT 1 FROM envios_inactividad e \
                 WHERE e.regla_id = r.id AND e.cliente_id = c.id \
             ) \
             ORDER BY r.user_id, r.dias_inactividad",
        )
        .fetch_all(pool)
        .await
    }

    /* Registrar envío realizado */
    pub async fn registrar_envio(
        pool: &PgPool,
        regla_id: Uuid,
        cliente_id: Uuid,
        estado: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO envios_inactividad (regla_id, cliente_id, estado) \
             VALUES ($1, $2, $3) ON CONFLICT (regla_id, cliente_id) DO NOTHING",
        )
        .bind(regla_id)
        .bind(cliente_id)
        .bind(estado)
        .execute(pool)
        .await?;
        Ok(())
    }
}
