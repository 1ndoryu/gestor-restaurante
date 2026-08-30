// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::ConfiguracionRestaurante;
use crate::services::{BdpBackupService, BdpWeblinkClient, ModoEfectivo, ServicioModoOperacion};

pub struct BdpWriteGuard;

/* [C1-2] Scopes de escritura BDP válidos. */
const VALID_BDP_WRITE_SCOPES: &[&str] = &[
    "create_order",
    "add_payment",
    "invoice",
    "create_customer",
    /* [198A-1/F1] Escrituras Glory -> BDP nuevas (catálogo, stock, deptos, comandas, plano, fidelización). */
    "create_article",
    "modify_article",
    "modify_prices",
    "create_department",
    "create_family",
    "regularize_stock",
    "transfer_stock",
    "inventory",
    "cancel_order",
    "add_tip",
    "add_points",
];

impl BdpWriteGuard {
    /// Un registro pendiente o ambiguo puede representar una operación remota
    /// aplicada sin confirmación local. Se bloquea cualquier nueva escritura
    /// sobre la misma entidad hasta reconciliación manual.
    /// Verifica si ya existe un registro de auditoría con la misma clave de
    /// `idempotency_key` para este usuario. Devuelve el `audit_id` y el resultado si
    /// se encuentra, o `None` si no existe.
    pub async fn check_idempotency(
        pool: &PgPool,
        user_id: Uuid,
        idempotency_key: &str,
    ) -> Result<Option<(Uuid, String)>, String> {
        let row: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT id, resultado FROM bdp_audit_log WHERE user_id = $1 AND idempotency_key = $2",
        )
        .bind(user_id)
        .bind(idempotency_key)
        .fetch_optional(pool)
        .await
        .map_err(|error| format!("No se pudo verificar idempotencia BDP: {error}"))?;
        Ok(row)
    }

    /// Crea un armado temporal para una operación de escritura BDP sin pasar
    /// por el flujo manual de configuración. Requiere que el feature flag
    /// `ff_bdp_auto_arm` esté activo y que `confirmation_text` coincida con el
    /// destino BDP canónico.
    ///
    /// [C1-2] Auto-arming: se usa desde el handler cuando el usuario confirma
    /// explícitamente una operación puntual (p. ej. "Enviar a BDP"). El
    /// pre-check de armado existente y el INSERT se ejecutan dentro de una
    /// transacción protegida por advisory lock para evitar condiciones de
    /// carrera con armado manual concurrente.
    #[allow(clippy::too_many_arguments)]
    pub async fn try_auto_arm(
        pool: &PgPool,
        user_id: Uuid,
        config: &ConfiguracionRestaurante,
        scope: &str,
        target_entity_type: &str,
        target_entity_id: Uuid,
        confirmation_text: &str,
    ) -> Result<(), String> {
        if !config.ff_bdp_auto_arm {
            return Err("Auto-arming BDP no está habilitado para este restaurante".into());
        }
        if ServicioModoOperacion::modo_efectivo_desde_config(config) != ModoEfectivo::Bdp {
            return Err("La integración BDP no está activa".into());
        }
        if !config.bdp_auto_backup_before_write {
            return Err("Escritura BDP bloqueada: auto-backup pre-write desactivado".into());
        }
        if !VALID_BDP_WRITE_SCOPES.contains(&scope) {
            return Err(format!("Scope de escritura BDP no válido: {scope}"));
        }

        let target = BdpBackupService::canonical_target(config)?;
        if confirmation_text.trim().trim_end_matches('/') != target {
            return Err("La confirmación no coincide con el destino BDP configurado".into());
        }

        Self::auto_arm_inner(
            pool,
            user_id,
            config,
            scope,
            target_entity_type,
            target_entity_id,
        )
        .await
    }

    /// [198A-1/F1] Auto-arming para el worker de push. Autorizado por
    /// `push_modalidad == "automatico"` (D1) o por una acción manual explícita
    /// (`forzar_manual`, botón "Sincronizar a BDP" — D1/D2) en lugar del flag
    /// interactivo `ff_bdp_auto_arm`; conserva el resto del fail-closed (sync
    /// activo, backup pre-write, scope válido, destino allowlist, snapshot
    /// vigente). El reintento manual tras bloqueo por suscripción (D2) usa
    /// `forzar_manual=true`.
    #[allow(clippy::too_many_arguments)]
    pub async fn armar_push(
        pool: &PgPool,
        user_id: Uuid,
        config: &ConfiguracionRestaurante,
        scope: &str,
        target_entity_type: &str,
        target_entity_id: Uuid,
        forzar_manual: bool,
    ) -> Result<(), String> {
        if !forzar_manual && config.push_modalidad != "automatico" {
            return Err("Push automático deshabilitado (push_modalidad no es 'automatico')".into());
        }
        if ServicioModoOperacion::modo_efectivo_desde_config(config) != ModoEfectivo::Bdp {
            return Err("La integración BDP no está activa".into());
        }
        if !config.bdp_auto_backup_before_write {
            return Err("Escritura BDP bloqueada: auto-backup pre-write desactivado".into());
        }
        if !VALID_BDP_WRITE_SCOPES.contains(&scope) {
            return Err(format!("Scope de escritura BDP no válido: {scope}"));
        }
        Self::auto_arm_inner(
            pool,
            user_id,
            config,
            scope,
            target_entity_type,
            target_entity_id,
        )
        .await
    }

    #[allow(clippy::too_many_lines)]
    async fn auto_arm_inner(
        pool: &PgPool,
        user_id: Uuid,
        config: &ConfiguracionRestaurante,
        scope: &str,
        target_entity_type: &str,
        target_entity_id: Uuid,
    ) -> Result<(), String> {
        let target = BdpBackupService::canonical_target(config)?;

        BdpWeblinkClient::new(config)
            .ensure_write_target_allowed()
            .map_err(|error| error.to_string())?;

        let fingerprint = BdpBackupService::connection_fingerprint(config)?;

        let mut tx = pool
            .begin()
            .await
            .map_err(|error| format!("No se pudo iniciar auto-arming BDP: {error}"))?;

        /* [C1-4] Serializar cualquier operación de armado para este usuario
         * dentro de la transacción para evitar que dos auto-arms concurrentes
         * (o uno concurrente con un armado manual) pisen el armado activo. */
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!("bdp-arming:{user_id}"))
            .execute(&mut *tx)
            .await
            .map_err(|error| format!("Error adquiriendo lock de armado BDP: {error}"))?;

        /* No pisar un armado manual/admin existente. */
        let arming_existente: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM bdp_write_arming WHERE user_id = $1 AND expires_at > NOW() AND remaining_operations > 0)",
        )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| format!("Error verificando armado BDP previo: {error}"))?;
        if arming_existente {
            return Err("Ya existe un armado BDP activo; consume el armado actual o espera a que venza antes de auto-armar".into());
        }

        let snapshot_id: Option<Uuid> = sqlx::query_scalar(
            r"SELECT id
                FROM bdp_snapshots
                WHERE user_id = $1
                  AND tipo = 'completo'
                  AND direccion = 'bdp'
                  AND target_base_url = $2
                  AND connection_fingerprint = $3
                  AND (expires_at IS NULL OR expires_at > NOW())
                  AND created_at >= NOW() - INTERVAL '24 hours'
                ORDER BY created_at DESC
                LIMIT 1",
        )
        .bind(user_id)
        .bind(&target)
        .bind(&fingerprint)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| format!("Error verificando snapshot BDP: {error}"))?;

        let snapshot_id = snapshot_id.ok_or_else(|| {
            "No se puede auto-armar BDP: falta un snapshot completo de esta conexión vigente."
                .to_string()
        })?;

        sqlx::query(
            r"INSERT INTO bdp_write_arming
               (user_id, base_url, scopes, target_entity_type, target_entity_id,
                reason, expires_at, remaining_operations, snapshot_id, connection_fingerprint)
               VALUES ($1, $2, $3, $4, $5, $6, NOW() + INTERVAL '5 minutes', 1, $7, $8)
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
        .bind(user_id)
        .bind(&target)
        .bind(vec![scope])
        .bind(target_entity_type)
        .bind(target_entity_id)
        .bind(format!(
            "auto_arm:{scope}:{target_entity_type}:{target_entity_id}"
        ))
        .bind(snapshot_id)
        .bind(&fingerprint)
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("No se pudo crear armado BDP automático: {error}"))?;

        sqlx::query(
            "UPDATE configuracion_restaurante SET bdp_sync_mode = 'unidirectional', updated_at = NOW() WHERE user_id = $1",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("No se pudo activar modo escritura BDP: {error}"))?;

        tx.commit()
            .await
            .map_err(|error| format!("No se pudo confirmar auto-arming BDP: {error}"))?;

        Ok(())
    }

    pub async fn ensure_no_unresolved(
        pool: &PgPool,
        user_id: Uuid,
        entity_field: &str,
        entity_id: Uuid,
        operations: &[&str],
    ) -> Result<(), String> {
        let exists: bool = sqlx::query_scalar(
            r"SELECT EXISTS (
                 SELECT 1 FROM bdp_audit_log
                 WHERE user_id = $1
                   AND datos_enviados ->> $2 = $3
                   AND operacion = ANY($4)
                   AND resultado IN ('pendiente', 'ambiguo')
               )",
        )
        .bind(user_id)
        .bind(entity_field)
        .bind(entity_id.to_string())
        .bind(operations)
        .fetch_one(pool)
        .await
        .map_err(|error| format!("No se pudo verificar estado ambiguo BDP: {error}"))?;
        if exists {
            return Err(format!(
                "Escritura BDP bloqueada: {entity_field}={entity_id} tiene una operación pendiente o ambigua; debe reconciliarse antes"
            ));
        }
        Ok(())
    }

    /// Convierte un armado temporal en una única intención auditable. La misma
    /// transacción consume el cupo, registra la intención y devuelve el modo a
    /// solo lectura ANTES de cualquier HTTP de escritura.
    #[allow(clippy::too_many_arguments)]
    /* [187A-1] Esta transacción es deliberadamente lineal: lock, bloqueo por
     * ambigüedad, consumo, auditoría y kill switch deben ser indivisibles. */
    #[allow(clippy::too_many_lines)]
    /// Autoriza una escritura BDP consumiendo un armado existente.
    /// Si `idempotency_key` se proporciona, se guarda en el registro de auditoría
    /// para permitir deduplicación posterior (C1 auto-arming).
    #[allow(clippy::too_many_arguments)]
    pub async fn authorize(
        pool: &PgPool,
        user_id: Uuid,
        config: &ConfiguracionRestaurante,
        scope: &str,
        target_entity_type: &str,
        target_entity_id: Uuid,
        entity_json_field: &str,
        datos_enviados: &serde_json::Value,
        snapshot_pre_id: Option<Uuid>,
        idempotency_key: Option<&str>,
    ) -> Result<Uuid, String> {
        /* La allowlist se valida antes de tocar auditoría o autorización. El
         * cliente HTTP repite esta comprobación justo antes del envío. */
        BdpWeblinkClient::new(config)
            .ensure_write_target_allowed()
            .map_err(|error| error.to_string())?;
        let base = BdpBackupService::canonical_target(config)?;
        let fingerprint = BdpBackupService::connection_fingerprint(config)?;

        let mut tx = pool
            .begin()
            .await
            .map_err(|error| format!("No se pudo iniciar autorización BDP: {error}"))?;

        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!(
                "bdp-write:{user_id}:{target_entity_type}:{target_entity_id}:{scope}"
            ))
            .execute(&mut *tx)
            .await
            .map_err(|error| format!("No se pudo bloquear la intención BDP: {error}"))?;

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
        .bind(user_id)
        .bind(target_entity_type)
        .bind(target_entity_id)
        .bind(entity_json_field)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| format!("No se pudo verificar intención previa BDP: {error}"))?;
        if unresolved {
            return Err(format!(
                "Escritura BDP bloqueada: {target_entity_type}={target_entity_id} tiene una operación pendiente o ambigua"
            ));
        }

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
        .bind(user_id)
        .bind(&base)
        .bind(scope)
        .bind(target_entity_type)
        .bind(target_entity_id)
        .bind(&fingerprint)
        .bind(&config.bdp_login)
        .bind(&config.bdp_password)
        .bind(&config.bdp_integrator_code)
        .bind(config.bdp_pos_id)
        .bind(config.bdp_employee_id)
        .bind(config.bdp_items_profile_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| format!("No se pudo verificar el armado BDP: {error}"))?;

        let (arming_snapshot_id, authorization_reason) = consumed.ok_or_else(|| {
            format!(
                "Escritura BDP bloqueada: no existe armado vigente para alcance {scope}, objetivo {target_entity_type}={target_entity_id}, destino exacto y cupo disponible"
            )
        })?;

        let audit_snapshot_id = snapshot_pre_id.or(arming_snapshot_id);

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
        .bind(user_id)
        .bind(scope)
        .bind(audit_snapshot_id)
        .bind(datos_enviados)
        .bind(&base)
        .bind(target_entity_type)
        .bind(target_entity_id)
        .bind(authorization_reason)
        .bind(idempotency_key)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| format!("No se pudo registrar intención BDP: {error}"))?;

        let Some(audit_id) = maybe_id else {
            let key = idempotency_key.unwrap_or("");
            let (existing_id, resultado): (Uuid, String) = sqlx::query_as(
                "SELECT id, resultado FROM bdp_audit_log WHERE user_id = $1 AND idempotency_key = $2",
            )
            .bind(user_id)
            .bind(key)
            .fetch_one(&mut *tx)
            .await
            .map_err(|error| format!("No se pudo leer auditoría BDP existente: {error}"))?;
            return Err(format!("idempotencia_duplicada:{existing_id}:{resultado}"));
        };

        let mode_updated = sqlx::query(
            "UPDATE configuracion_restaurante SET bdp_sync_mode = 'read_only', updated_at = NOW() WHERE user_id = $1",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("No se pudo cerrar modo escritura BDP: {error}"))?;
        if mode_updated.rows_affected() != 1 {
            return Err(
                "No se pudo cerrar modo escritura BDP: configuración inexistente".to_string(),
            );
        }

        sqlx::query("DELETE FROM bdp_write_arming WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|error| format!("No se pudo eliminar armado BDP consumido: {error}"))?;

        tx.commit()
            .await
            .map_err(|error| format!("No se pudo confirmar autorización BDP: {error}"))?;
        Ok(audit_id)
    }
}
