// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [D32] Repositorio de asignaciones (responsable y procedimiento) por operación no API. */

use std::collections::HashMap;

use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::services::bdp_operaciones_no_api::Asignacion;

pub struct BdpOperacionesNoApiRepository;

impl BdpOperacionesNoApiRepository {
    /// Asignaciones guardadas del usuario, indexadas por código de operación.
    pub async fn listar(pool: &PgPool, user_id: Uuid) -> Result<HashMap<String, Asignacion>, sqlx::Error> {
        let filas = sqlx::query(
            "SELECT codigo, responsable, procedimiento FROM bdp_operaciones_no_api WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        Ok(filas
            .iter()
            .map(|fila| {
                (
                    fila.get::<String, _>("codigo"),
                    Asignacion {
                        responsable: fila.get("responsable"),
                        procedimiento: fila.get("procedimiento"),
                    },
                )
            })
            .collect())
    }

    /// Crea o reemplaza la asignación de una operación.
    pub async fn guardar(
        pool: &PgPool,
        user_id: Uuid,
        codigo: &str,
        asignacion: &Asignacion,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO bdp_operaciones_no_api (user_id, codigo, responsable, procedimiento) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (user_id, codigo) DO UPDATE SET \
             responsable = EXCLUDED.responsable, procedimiento = EXCLUDED.procedimiento, updated_at = NOW()",
        )
        .bind(user_id)
        .bind(codigo)
        .bind(&asignacion.responsable)
        .bind(&asignacion.procedimiento)
        .execute(pool)
        .await?;
        Ok(())
    }
}
