// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
use sqlx::PgPool;
use uuid::Uuid;

use super::BdpWriteGuard;
use crate::models::ConfiguracionRestaurante;
use crate::services::{BdpBackupService, BdpWeblinkClient, ModoEfectivo, ServicioModoOperacion};

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
        /* [267A-7] En modo automático no hay nada que armar: la autorización
         * permanente ya cubre la operación. No-op para no degradar el modo. */
        if config.bdp_sync_mode == "automatic" {
            return Ok(());
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
        /* [10AA-3-A] Si ya hay un armado vigente que cubre esta operación
         * exacta, se reutiliza. `auto_arm_inner` se niega a pisar un armado
         * activo y devolvía error, así que el armado manual (el que la usuaria
         * concede por lote con `PUT /configuracion/bdp/sync-mode`, nombrando
         * destino y entidad) bloqueaba justo la operación que autorizaba: la
         * fila quedaba en la cola y el push nunca salía. Reutilizar no rebaja
         * el fail-closed —el armado previo ya nombra este destino y esta
         * entidad— y `authorize` sigue siendo la puerta final. */
        if Self::armado_vigente_para(
            pool,
            user_id,
            config,
            scope,
            target_entity_type,
            target_entity_id,
        )
        .await?
        {
            return Ok(());
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

    /// [10AA-3-A] ¿Existe un armado vigente que cubre esta operación exacta?
    /// Las condiciones son las mismas que aplica el `UPDATE` de `authorize`
    /// para consumirlo (alcance, entidad, huella de conexión, snapshot y
    /// `bdp_sync_mode`): si devuelve `true`, `authorize` podrá consumirlo.
    async fn armado_vigente_para(
        pool: &PgPool,
        user_id: Uuid,
        config: &ConfiguracionRestaurante,
        scope: &str,
        target_entity_type: &str,
        target_entity_id: Uuid,
    ) -> Result<bool, String> {
        let base = BdpBackupService::canonical_target(config)?;
        let fingerprint = BdpBackupService::connection_fingerprint(config)?;
        sqlx::query_scalar(
            r"SELECT EXISTS (
                 SELECT 1 FROM bdp_write_arming
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
                       AND c.bdp_sync_mode = 'unidirectional'
                   )
               )",
        )
        .bind(user_id)
        .bind(&base)
        .bind(scope)
        .bind(target_entity_type)
        .bind(target_entity_id)
        .bind(&fingerprint)
        .fetch_one(pool)
        .await
        .map_err(|error| format!("No se pudo verificar el armado BDP vigente: {error}"))
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

    /// Cancela el armado creado por el worker cuando una lectura previa a la
    /// autorización falla. Sin esta compensación, el armado de un artículo
    /// quedaría activo y bloquearía indefinidamente la siguiente oportunidad
    /// de procesar la misma fila de cola.
    pub async fn cancelar_armado_push(
        pool: &PgPool,
        user_id: Uuid,
        config: &ConfiguracionRestaurante,
        scope: &str,
        target_entity_type: &str,
        target_entity_id: Uuid,
    ) -> Result<(), String> {
        let base = BdpBackupService::canonical_target(config)?;
        let fingerprint = BdpBackupService::connection_fingerprint(config)?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|error| format!("No se pudo iniciar cancelación de armado BDP: {error}"))?;

        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!("bdp-arming:{user_id}"))
            .execute(&mut *tx)
            .await
            .map_err(|error| format!("No se pudo bloquear cancelación de armado BDP: {error}"))?;

        let deleted = sqlx::query(
            r"DELETE FROM bdp_write_arming
               WHERE user_id = $1
                 AND base_url = $2
                 AND $3 = ANY(scopes)
                 AND target_entity_type = $4
                 AND target_entity_id = $5
                 AND connection_fingerprint = $6",
        )
        .bind(user_id)
        .bind(&base)
        .bind(scope)
        .bind(target_entity_type)
        .bind(target_entity_id)
        .bind(&fingerprint)
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("No se pudo cancelar armado BDP: {error}"))?;

        if deleted.rows_affected() == 1 {
            sqlx::query(
                "UPDATE configuracion_restaurante SET bdp_sync_mode = 'read_only', updated_at = NOW() WHERE user_id = $1",
            )
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|error| format!("No se pudo cerrar modo escritura BDP tras cancelar armado: {error}"))?;
        }

        tx.commit()
            .await
            .map_err(|error| format!("No se pudo confirmar cancelación de armado BDP: {error}"))?;
        Ok(())
    }
}
