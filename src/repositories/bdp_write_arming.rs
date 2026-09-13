// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [267A-7] Repositorio del armado de escritura BDP (`bdp_write_arming`).
 * Centraliza snapshot vigente + armar + desarmar, que antes vivían con SQL
 * inline en `handlers::configuracion`. El handler conserva junta la puerta de
 * armado ([187A-1]): validación de destino/alcance/objetivo/huella y la
 * invalidación de caché de modo; aquí solo hay persistencia. */

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

/// Parámetros del armado de escritura (un único alcance, una operación).
/// `scopes` se pasa por valor porque sqlx codifica `Vec<String>` como `TEXT[]`.
pub struct NuevoArmado {
    pub user_id: Uuid,
    pub base_url: String,
    pub scopes: Vec<String>,
    pub target_entity_type: String,
    pub target_entity_id: Uuid,
    pub reason: String,
    pub duracion_minutos: i32,
    pub max_operaciones: i32,
    pub snapshot_id: Uuid,
    pub connection_fingerprint: String,
}

pub struct BdpWriteArmingRepository;

impl BdpWriteArmingRepository {
    /// Snapshot completo vigente de esta conexión BDP exacta (24h, sin lecturas fallidas).
    pub async fn buscar_snapshot_vigente(
        pool: &PgPool,
        user_id: Uuid,
        target_base_url: &str,
        fingerprint: &str,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar(
            r"SELECT id
                FROM bdp_snapshots
                WHERE user_id = $1
                  AND tipo = 'completo'
                  AND direccion = 'bdp'
                  AND target_base_url = $2
                  AND connection_fingerprint = $3
                  AND (expires_at IS NULL OR expires_at > NOW())
                  AND created_at >= NOW() - INTERVAL '24 hours'
                  AND datos->'articulos' IS NOT NULL AND datos->'articulos' <> 'null'::jsonb
                  AND datos->'clientes' IS NOT NULL AND datos->'clientes' <> 'null'::jsonb
                  AND datos->'departamentos' IS NOT NULL AND datos->'departamentos' <> 'null'::jsonb
                  AND datos->'salones' IS NOT NULL AND datos->'salones' <> 'null'::jsonb
                  AND datos->'empleados' IS NOT NULL AND datos->'empleados' <> 'null'::jsonb
                ORDER BY created_at DESC
                LIMIT 1",
        )
        .bind(user_id)
        .bind(target_base_url)
        .bind(fingerprint)
        .fetch_optional(pool)
        .await
    }

    /// Crea o reemplaza el armado del usuario (un armado vigente por usuario).
    /// `params` se consume porque `scopes` se enlaza por valor (`TEXT[]`).
    pub async fn armar(pool: &PgPool, params: NuevoArmado) -> Result<(), sqlx::Error> {
        sqlx::query(
            r"INSERT INTO bdp_write_arming
               (user_id, base_url, scopes, target_entity_type, target_entity_id,
                reason, expires_at, remaining_operations, snapshot_id, connection_fingerprint)
               VALUES ($1, $2, $3, $4, $5, $6, NOW() + ($7 * INTERVAL '1 minute'), $8, $9, $10)
               ON CONFLICT (user_id) DO UPDATE SET
                 base_url = EXCLUDED.base_url,
                 scopes = EXCLUDED.scopes,
                 target_entity_type = EXCLUDED.target_entity_type,
                 target_entity_id = EXCLUDED.target_entity_id,
                 reason = EXCLUDED.reason,
                 expires_at = EXCLUDED.expires_at,
                 remaining_operations = EXCLUDED.remaining_operations,
                 snapshot_id = EXCLUDED.snapshot_id,
                 connection_fingerprint = EXCLUDED.connection_fingerprint,
                 created_at = NOW()",
        )
        .bind(params.user_id)
        .bind(params.base_url)
        .bind(params.scopes)
        .bind(params.target_entity_type)
        .bind(params.target_entity_id)
        .bind(params.reason)
        .bind(params.duracion_minutos)
        .bind(params.max_operaciones)
        .bind(params.snapshot_id)
        .bind(params.connection_fingerprint)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Elimina el armado del usuario (vuelta a `read_only`).
    pub async fn desarmar(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM bdp_write_arming WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Variante transaccional del desarmado (cambio de configuración con
    /// invalidación de armado en la misma unidad de trabajo).
    pub async fn desarmar_tx(
        tx: &mut Transaction<'_, Postgres>,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM bdp_write_arming WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }
}
