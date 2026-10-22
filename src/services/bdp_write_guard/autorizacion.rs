// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::BdpWriteGuard;
use crate::models::ConfiguracionRestaurante;
use crate::services::BdpBackupService;

/// Datos de una escritura BDP agrupados, para no repetir una docena de
/// argumentos en cada paso de la transacción.
#[derive(Clone, Copy)]
struct Escritura<'a> {
    user_id: Uuid,
    scope: &'a str,
    target_entity_type: &'a str,
    target_entity_id: Uuid,
    entity_json_field: &'a str,
    datos_enviados: &'a serde_json::Value,
    idempotency_key: Option<&'a str>,
    base: &'a str,
}

/// Datos de una escritura a autorizar. Agrupados para que `authorize` no
/// tenga más parámetros de los permitidos.
pub struct EscrituraAutorizar<'a> {
    pub scope: &'a str,
    pub target_entity_type: &'a str,
    pub target_entity_id: Uuid,
    pub entity_json_field: &'a str,
    pub datos_enviados: &'a serde_json::Value,
    pub snapshot_pre_id: Option<Uuid>,
    pub idempotency_key: Option<&'a str>,
}

impl BdpWriteGuard {
    /// Convierte un armado temporal en una única intención auditable. La misma
    /// transacción consume el cupo, registra la intención y devuelve el modo a
    /// solo lectura ANTES de cualquier HTTP de escritura.
    /// Autoriza una escritura BDP consumiendo un armado existente.
    /// Si `idempotency_key` se proporciona, se guarda en el registro de auditoría
    /// para permitir deduplicación posterior (C1 auto-arming).
    pub async fn authorize(
        pool: &PgPool,
        user_id: Uuid,
        config: &ConfiguracionRestaurante,
        datos: EscrituraAutorizar<'_>,
    ) -> Result<Uuid, String> {
        let EscrituraAutorizar {
            scope,
            target_entity_type,
            target_entity_id,
            entity_json_field,
            datos_enviados,
            snapshot_pre_id,
            idempotency_key,
        } = datos;
        let base = BdpBackupService::canonical_target(config)?;
        let fingerprint = BdpBackupService::connection_fingerprint(config)?;
        let escritura = Escritura {
            user_id,
            scope,
            target_entity_type,
            target_entity_id,
            entity_json_field,
            datos_enviados,
            idempotency_key,
            base: &base,
        };

        /* [267A-7] Modo automático: autorización permanente sin armado por
         * operación. Misma auditoría e idempotencia que el modo manual, pero
         * no consume armado ni devuelve el modo a `read_only`.
         * [D-auto-push] Sí descarta el armado de esta entidad: `armar_push`
         * deja uno en cada envío automático y en modo `automatic` nadie lo
         * consume, así que a los dos envíos de la misma entidad el
         * `auto_arm_inner` se negaba con «Ya existe un armado BDP activo» y
         * el barrido periódico se quedaba parado el resto del TTL. */
        if config.bdp_sync_mode == "automatic" {
            return Self::authorize_automatic(pool, config, escritura, snapshot_pre_id).await;
        }

        let mut tx = pool
            .begin()
            .await
            .map_err(|error| format!("No se pudo iniciar autorización BDP: {error}"))?;

        /* Esta transacción es deliberadamente lineal: lock, bloqueo por
         * ambigüedad, consumo, auditoría y kill switch deben ser indivisibles. */
        bloquear_escritura(&mut tx, escritura).await?;
        comprobar_sin_pendiente(&mut tx, escritura).await?;
        let (arming_snapshot_id, razon) =
            consumir_armado(&mut tx, escritura, config, &fingerprint).await?;
        let audit_snapshot_id = snapshot_pre_id.or(arming_snapshot_id);
        let audit_id = registrar_intencion(&mut tx, escritura, audit_snapshot_id, &razon).await?;
        cerrar_armado(&mut tx, user_id).await?;

        tx.commit()
            .await
            .map_err(|error| format!("No se pudo confirmar autorización BDP: {error}"))?;
        Ok(audit_id)
    }

    /// Autoriza una escritura en modo `automatic`: verifica que la configuración
    /// siga apuntando al mismo destino con las mismas credenciales y registra
    /// la intención auditable. No consume armado (no hay) y no altera el modo.
    async fn authorize_automatic(
        pool: &PgPool,
        config: &ConfiguracionRestaurante,
        escritura: Escritura<'_>,
        snapshot_pre_id: Option<Uuid>,
    ) -> Result<Uuid, String> {
        let mut tx = pool
            .begin()
            .await
            .map_err(|error| format!("No se pudo iniciar autorización BDP: {error}"))?;

        bloquear_escritura(&mut tx, escritura).await?;
        comprobar_sin_pendiente(&mut tx, escritura).await?;
        comprobar_config_automatica(&mut tx, escritura, config).await?;
        let audit_id =
            registrar_intencion(&mut tx, escritura, snapshot_pre_id, "modo_automatico").await?;
        /* [D-auto-push] El armado ya cumplió su función (destino, alcance y
         * snapshot vigentes): se borra sin tocar `bdp_sync_mode`, que en este
         * modo es permanente. Mismo advisory lock de la escritura: ningún otro
         * envío de esta entidad puede estar armando a la vez. */
        descartar_armado(&mut tx, escritura).await?;

        tx.commit()
            .await
            .map_err(|error| format!("No se pudo confirmar autorización BDP: {error}"))?;
        Ok(audit_id)
    }
}

/// Serializa las escrituras de la misma entidad y alcance con un advisory lock
/// de transacción; se libera solo al cerrar la transacción.
async fn bloquear_escritura(
    tx: &mut Transaction<'_, Postgres>,
    e: Escritura<'_>,
) -> Result<(), String> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!(
            "bdp-write:{}:{}:{}:{}",
            e.user_id, e.target_entity_type, e.target_entity_id, e.scope
        ))
        .execute(&mut **tx)
        .await
        .map_err(|error| format!("No se pudo bloquear la intención BDP: {error}"))?;
    Ok(())
}

/// Bloquea la escritura si la misma entidad tiene una operación pendiente o
/// ambigua: hasta reconciliarla no se sabe si ya se aplicó en BDP.
async fn comprobar_sin_pendiente(
    tx: &mut Transaction<'_, Postgres>,
    e: Escritura<'_>,
) -> Result<(), String> {
    let unresolved: bool = sqlx::query_scalar(
        r"SELECT EXISTS (
                 SELECT 1 FROM bdp_audit_log
                 WHERE user_id = $1
                   AND resultado IN ('pendiente', 'ambiguo')
                   AND (
                     (target_entity_type = $2 AND target_entity_id = $3)
                     OR datos_enviados ->> $4 = $3::TEXT
                   )
               )",
    )
    .bind(e.user_id)
    .bind(e.target_entity_type)
    .bind(e.target_entity_id)
    .bind(e.entity_json_field)
    .fetch_one(&mut **tx)
    .await
    .map_err(|error| format!("No se pudo verificar intención previa BDP: {error}"))?;
    if unresolved {
        return Err(format!(
            "Escritura BDP bloqueada: {}={} tiene una operación pendiente o ambigua",
            e.target_entity_type, e.target_entity_id
        ));
    }
    Ok(())
}

/// Comprueba en modo automático que la configuración sigue apuntando al mismo
/// destino y credenciales (segunda barrera: un PATCH de conexión ya fuerza
/// `read_only`).
async fn comprobar_config_automatica(
    tx: &mut Transaction<'_, Postgres>,
    e: Escritura<'_>,
    config: &ConfiguracionRestaurante,
) -> Result<(), String> {
    let config_ok: bool = sqlx::query_scalar(
        r"SELECT EXISTS (
                 SELECT 1 FROM configuracion_restaurante c
                 WHERE c.user_id = $1
                   AND TRIM(TRAILING '/' FROM TRIM(c.bdp_base_url)) = $2
                   AND c.bdp_login = $3
                   AND c.bdp_password = $4
                   AND c.bdp_integrator_code = $5
                   AND c.bdp_pos_id = $6
                   AND c.bdp_employee_id = $7
                   AND c.bdp_items_profile_id = $8
                   AND c.bdp_sync_mode = 'automatic'
               )",
    )
    .bind(e.user_id)
    .bind(e.base)
    .bind(&config.bdp_login)
    .bind(&config.bdp_password)
    .bind(&config.bdp_integrator_code)
    .bind(config.bdp_pos_id)
    .bind(config.bdp_employee_id)
    .bind(config.bdp_items_profile_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|error| format!("No se pudo verificar el modo automático BDP: {error}"))?;
    if !config_ok {
        return Err(
            "Escritura BDP bloqueada: el modo automático ya no está activo para este destino exacto"
                .into(),
        );
    }
    Ok(())
}

/// Consume una unidad del armado vigente para este alcance, entidad y destino
/// exacto. Devuelve el snapshot del armado y el motivo que se audita.
async fn consumir_armado(
    tx: &mut Transaction<'_, Postgres>,
    e: Escritura<'_>,
    config: &ConfiguracionRestaurante,
    fingerprint: &str,
) -> Result<(Option<Uuid>, String), String> {
    let consumed: Option<(Option<Uuid>, String)> = sqlx::query_as(
        r"UPDATE bdp_write_arming
               SET remaining_operations = remaining_operations - 1
               WHERE user_id = $1
                 AND base_url = $2
                 AND $3 = ANY(scopes)
                 AND target_entity_type = $4
                 AND target_entity_id = $5
                 AND connection_fingerprint = $6
                 AND snapshot_id IS NOT NULL
                 AND expires_at > NOW()
                 AND remaining_operations > 0
                 AND EXISTS (
                   SELECT 1 FROM configuracion_restaurante c
                   WHERE c.user_id = $1
                     AND TRIM(TRAILING '/' FROM TRIM(c.bdp_base_url)) = $2
                     AND c.bdp_login = $7
                     AND c.bdp_password = $8
                     AND c.bdp_integrator_code = $9
                     AND c.bdp_pos_id = $10
                     AND c.bdp_employee_id = $11
                     AND c.bdp_items_profile_id = $12
                     AND c.bdp_sync_mode = 'unidirectional'
                 )
               RETURNING snapshot_id, reason",
    )
    .bind(e.user_id)
    .bind(e.base)
    .bind(e.scope)
    .bind(e.target_entity_type)
    .bind(e.target_entity_id)
    .bind(fingerprint)
    .bind(&config.bdp_login)
    .bind(&config.bdp_password)
    .bind(&config.bdp_integrator_code)
    .bind(config.bdp_pos_id)
    .bind(config.bdp_employee_id)
    .bind(config.bdp_items_profile_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| format!("No se pudo verificar el armado BDP: {error}"))?;

    consumed.ok_or_else(|| {
        format!(
            "Escritura BDP bloqueada: no existe armado vigente para alcance {}, objetivo {}={}, destino exacto y cupo disponible",
            e.scope, e.target_entity_type, e.target_entity_id
        )
    })
}

/// Registra la intención en la auditoría. Si `idempotency_key` ya existe, no
/// inserta nada y devuelve el registro previo como error estructurado.
async fn registrar_intencion(
    tx: &mut Transaction<'_, Postgres>,
    e: Escritura<'_>,
    snapshot_id: Option<Uuid>,
    razon: &str,
) -> Result<Uuid, String> {
    /* [C1-5] Si se proporciona idempotency_key, usar ON CONFLICT para que
     * dos requests concurrentes con la misma clave no creen dos
     * intenciones. Si ya existe, devolvemos un error estructurado con el
     * resultado actual para que el handler decida. Las filas con
     * idempotency_key NULL nunca pueden conflictar gracias al índice
     * parcial, por lo que la misma consulta sirve para ambos casos. */
    let maybe_id: Option<Uuid> = sqlx::query_scalar(
        r"INSERT INTO bdp_audit_log
               (user_id, operacion, direccion, snapshot_pre_id, datos_enviados,
                resultado, target_base_url, target_entity_type, target_entity_id,
                authorization_reason, idempotency_key)
               VALUES ($1, $2, 'glory_to_bdp', $3, $4, 'pendiente', $5, $6, $7, $8, $9)
               ON CONFLICT (user_id, idempotency_key) WHERE idempotency_key IS NOT NULL DO NOTHING
               RETURNING id",
    )
    .bind(e.user_id)
    .bind(e.scope)
    .bind(snapshot_id)
    .bind(e.datos_enviados)
    .bind(e.base)
    .bind(e.target_entity_type)
    .bind(e.target_entity_id)
    .bind(razon)
    .bind(e.idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| format!("No se pudo registrar intención BDP: {error}"))?;

    let Some(audit_id) = maybe_id else {
        let key = e.idempotency_key.unwrap_or("");
        let (existing_id, resultado): (Uuid, String) = sqlx::query_as(
            "SELECT id, resultado FROM bdp_audit_log WHERE user_id = $1 AND idempotency_key = $2",
        )
        .bind(e.user_id)
        .bind(key)
        .fetch_one(&mut **tx)
        .await
        .map_err(|error| format!("No se pudo leer auditoría BDP existente: {error}"))?;
        return Err(format!("idempotencia_duplicada:{existing_id}:{resultado}"));
    };
    Ok(audit_id)
}

/// Devuelve el modo a solo lectura y borra el armado consumido. Si no hay
/// configuración que actualizar, aborta antes de borrar el armado.
async fn cerrar_armado(tx: &mut Transaction<'_, Postgres>, user_id: Uuid) -> Result<(), String> {
    let mode_updated = sqlx::query(
        "UPDATE configuracion_restaurante SET bdp_sync_mode = 'read_only', updated_at = NOW() WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(|error| format!("No se pudo cerrar modo escritura BDP: {error}"))?;
    if mode_updated.rows_affected() != 1 {
        return Err("No se pudo cerrar modo escritura BDP: configuración inexistente".to_string());
    }

    sqlx::query("DELETE FROM bdp_write_arming WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut **tx)
        .await
        .map_err(|error| format!("No se pudo eliminar armado BDP consumido: {error}"))?;
    Ok(())
}

/// [D-auto-push] Borra el armado de la entidad y alcance de esta escritura sin
/// tocar `bdp_sync_mode`. En modo `automatic` el armado no se consume: sin
/// esto, la primera fila de una entidad dejaba un armado vivo que el
/// `auto_arm_inner` del siguiente envío se negaba a pisar.
async fn descartar_armado(
    tx: &mut Transaction<'_, Postgres>,
    e: Escritura<'_>,
) -> Result<(), String> {
    sqlx::query(
        "DELETE FROM bdp_write_arming
          WHERE user_id = $1 AND target_entity_type = $2 AND target_entity_id = $3
            AND $4 = ANY(scopes)",
    )
    .bind(e.user_id)
    .bind(e.target_entity_type)
    .bind(e.target_entity_id)
    .bind(e.scope)
    .execute(&mut **tx)
    .await
    .map_err(|error| format!("No se pudo descartar el armado BDP: {error}"))?;
    Ok(())
}
