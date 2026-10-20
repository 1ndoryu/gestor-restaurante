// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [L10/D01] Registro formal del diagnóstico de salud en `bdp_sync_cursor` (dominio
 * `salud`): deja constancia de que salud, versión, login y acceso respondieron. */

use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

pub const DOMINIO_CURSOR_SALUD: &str = "salud";

pub struct BdpSyncCursorRepository;

impl BdpSyncCursorRepository {
    /// Upsert del último diagnóstico. Sin `error` marca éxito; con `error` guarda el motivo
    /// y conserva el `ultimo_exito` anterior.
    pub async fn registrar_salud(
        pool: &PgPool,
        user_id: Uuid,
        posicion: &Value,
        error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO bdp_sync_cursor \
             (id, user_id, dominio, perfil, posicion, ultimo_exito, ultimo_intento, ultimo_error) \
             VALUES ($1, $2, $3, '', $4, CASE WHEN $5::text IS NULL THEN NOW() ELSE NULL END, NOW(), $5) \
             ON CONFLICT (user_id, dominio, perfil) DO UPDATE SET \
             posicion = EXCLUDED.posicion, \
             ultimo_exito = CASE WHEN $5::text IS NULL THEN NOW() ELSE bdp_sync_cursor.ultimo_exito END, \
             ultimo_intento = NOW(), \
             ultimo_error = EXCLUDED.ultimo_error",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(DOMINIO_CURSOR_SALUD)
        .bind(posicion)
        .bind(error)
        .execute(pool)
        .await?;
        Ok(())
    }
}
