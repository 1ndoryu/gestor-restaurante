// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
/* [198A-1/D7] Repositorio de clasificaciones locales (departamento/familia).
 * El código BDP se asigna secuencialmente por (user_id, tipo); el UNIQUE sobre
 * (user_id, tipo, code) protege de colisiones en concurrencia (rare). */

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{BdpCatalogoClasificacion, CrearBdpClasificacionRequest};

pub struct BdpCatalogoClasificacionRepository;

impl BdpCatalogoClasificacionRepository {
    pub async fn listar(
        pool: &PgPool,
        user_id: Uuid,
        tipo: &str,
    ) -> Result<Vec<BdpCatalogoClasificacion>, sqlx::Error> {
        sqlx::query_as::<_, BdpCatalogoClasificacion>(
            "SELECT * FROM bdp_catalogo_clasificaciones \
             WHERE user_id = $1 AND tipo = $2 ORDER BY code ASC",
        )
        .bind(user_id)
        .bind(tipo)
        .fetch_all(pool)
        .await
    }

    pub async fn crear(
        pool: &PgPool,
        user_id: Uuid,
        req: &CrearBdpClasificacionRequest,
    ) -> Result<BdpCatalogoClasificacion, sqlx::Error> {
        let code = Self::siguiente_code(pool, user_id, &req.tipo).await?;
        sqlx::query_as::<_, BdpCatalogoClasificacion>(
            "INSERT INTO bdp_catalogo_clasificaciones (id, user_id, tipo, code, nombre) \
             VALUES ($1, $2, $3, $4, $5) RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(&req.tipo)
        .bind(code)
        .bind(&req.nombre)
        .fetch_one(pool)
        .await
    }

    /* [159A-2/F1] Lookup + inserción con código BDP explícito para el import
     * de departamentos (el alta local sigue usando `crear` secuencial). */
    pub async fn buscar_por_code(
        pool: &PgPool,
        user_id: Uuid,
        tipo: &str,
        code: i32,
    ) -> Result<Option<BdpCatalogoClasificacion>, sqlx::Error> {
        sqlx::query_as::<_, BdpCatalogoClasificacion>(
            "SELECT * FROM bdp_catalogo_clasificaciones \
              WHERE user_id = $1 AND tipo = $2 AND code = $3",
        )
        .bind(user_id)
        .bind(tipo)
        .bind(code)
        .fetch_optional(pool)
        .await
    }

    pub async fn buscar_por_nombre(
        pool: &PgPool,
        user_id: Uuid,
        tipo: &str,
        nombre: &str,
    ) -> Result<Option<BdpCatalogoClasificacion>, sqlx::Error> {
        sqlx::query_as::<_, BdpCatalogoClasificacion>(
            "SELECT * FROM bdp_catalogo_clasificaciones \
              WHERE user_id = $1 AND tipo = $2 AND nombre = $3",
        )
        .bind(user_id)
        .bind(tipo)
        .bind(nombre)
        .fetch_optional(pool)
        .await
    }

    pub async fn crear_con_code(
        pool: &PgPool,
        user_id: Uuid,
        tipo: &str,
        code: i32,
        nombre: &str,
    ) -> Result<BdpCatalogoClasificacion, sqlx::Error> {
        sqlx::query_as::<_, BdpCatalogoClasificacion>(
            "INSERT INTO bdp_catalogo_clasificaciones (id, user_id, tipo, code, nombre) \
              VALUES ($1, $2, $3, $4, $5) RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(tipo)
        .bind(code)
        .bind(nombre)
        .fetch_one(pool)
        .await
    }

    pub async fn siguiente_code(
        pool: &PgPool,
        user_id: Uuid,
        tipo: &str,
    ) -> Result<i32, sqlx::Error> {
        let next: Option<i32> = sqlx::query_scalar(
            "SELECT COALESCE(MAX(code), 0) + 1 \
             FROM bdp_catalogo_clasificaciones WHERE user_id = $1 AND tipo = $2",
        )
        .bind(user_id)
        .bind(tipo)
        .fetch_one(pool)
        .await?;
        Ok(next.unwrap_or(1))
    }
}
