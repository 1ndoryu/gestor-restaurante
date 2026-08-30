// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [197A-3] Aprovisiona BDP desde secretos del servidor para una cuenta
 * explícita. Es idempotente, no sobrescribe valores confirmados y deja toda
 * escritura en solo lectura. */

use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct BdpBootstrapSettings {
    pub user_email: String,
    pub base_url: String,
    pub login: String,
    pub password: String,
    pub integrator_code: String,
    pub pos_id: i32,
    pub employee_id: i32,
    pub items_profile_id: i32,
    pub default_article_code: String,
    pub default_article_name: String,
    pub tender_map: Value,
    pub order_type_map: Value,
    pub default_customer_code: String,
    pub poll_interval_secs: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BdpBootstrapOutcome {
    Disabled,
    AlreadyApplied { user_id: Uuid },
    Applied { user_id: Uuid },
}

pub struct BdpConfigBootstrapService;

impl BdpConfigBootstrapService {
    pub async fn apply_from_env(pool: &PgPool) -> Result<BdpBootstrapOutcome, String> {
        let Some(settings) = Self::settings_from_env()? else {
            return Ok(BdpBootstrapOutcome::Disabled);
        };
        Self::apply(pool, &settings).await
    }

    pub async fn apply(
        pool: &PgPool,
        settings: &BdpBootstrapSettings,
    ) -> Result<BdpBootstrapOutcome, String> {
        Self::validate(settings)?;

        let mut tx = pool
            .begin()
            .await
            .map_err(|error| format!("No se pudo iniciar bootstrap BDP: {error}"))?;

        let (user_id, already_applied) = lock_target_configuration(&mut tx, settings).await?;
        if already_applied {
            tx.commit()
                .await
                .map_err(|error| format!("No se pudo cerrar bootstrap BDP: {error}"))?;
            return Ok(BdpBootstrapOutcome::AlreadyApplied { user_id });
        }
        let (target_base_url, preserved_existing) =
            apply_safe_configuration(&mut tx, user_id, settings).await?;
        close_write_permissions(&mut tx, user_id).await?;
        audit_bootstrap(&mut tx, user_id, &target_base_url, preserved_existing).await?;

        tx.commit()
            .await
            .map_err(|error| format!("No se pudo confirmar bootstrap BDP: {error}"))?;
        Ok(BdpBootstrapOutcome::Applied { user_id })
    }

    fn settings_from_env() -> Result<Option<BdpBootstrapSettings>, String> {
        let Some(user_email) = env_optional("BDP_BOOTSTRAP_USER_EMAIL") else {
            return Ok(None);
        };
        Ok(Some(BdpBootstrapSettings {
            user_email,
            base_url: env_required("BDP_BASE_URL")?,
            login: env_required("BDP_LOGIN")?,
            password: env_required("BDP_PASSWORD")?,
            integrator_code: env_required("BDP_INTEGRATOR_CODE")?,
            pos_id: env_positive_i32("BDP_POS_ID")?,
            employee_id: env_positive_i32("BDP_EMPLOYEE_ID")?,
            items_profile_id: env_positive_i32("BDP_ITEMS_PROFILE_ID")?,
            default_article_code: env_required("BDP_DEFAULT_ARTICLE_CODE")?,
            default_article_name: env_required("BDP_DEFAULT_ARTICLE_NAME")?,
            tender_map: env_json_map("BDP_TENDER_MAP_JSON")?,
            order_type_map: env_json_map("BDP_ORDER_TYPE_MAP_JSON")?,
            default_customer_code: env_optional("BDP_DEFAULT_CUSTOMER_CODE").unwrap_or_default(),
            poll_interval_secs: env_optional("BDP_POLL_INTERVAL_SECS").map_or(Ok(60), |value| {
                value.parse::<i32>().map_err(|_| {
                    "BDP_POLL_INTERVAL_SECS debe ser un entero entre 10 y 600".to_string()
                })
            })?,
        }))
    }

    fn validate(settings: &BdpBootstrapSettings) -> Result<(), String> {
        let parsed = reqwest::Url::parse(settings.base_url.trim())
            .map_err(|_| "BDP_BASE_URL no es una URL válida".to_string())?;
        if !matches!(parsed.scheme(), "http" | "https")
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || parsed.path() != "/"
        {
            return Err("BDP_BASE_URL debe ser un origen HTTP(S) sin ruta ni credenciales".into());
        }
        if settings
            .default_article_code
            .trim()
            .parse::<i64>()
            .ok()
            .is_none_or(|value| value <= 0)
        {
            return Err("BDP_DEFAULT_ARTICLE_CODE debe ser numérico y positivo".into());
        }
        if !settings.default_customer_code.trim().is_empty()
            && settings
                .default_customer_code
                .trim()
                .parse::<i64>()
                .ok()
                .is_none_or(|value| value <= 0)
        {
            return Err("BDP_DEFAULT_CUSTOMER_CODE debe ser numérico y positivo".into());
        }
        if !(10..=600).contains(&settings.poll_interval_secs) {
            return Err("BDP_POLL_INTERVAL_SECS debe estar entre 10 y 600".into());
        }
        validate_map(&settings.tender_map, 1, "BDP_TENDER_MAP_JSON")?;
        validate_map(&settings.order_type_map, 0, "BDP_ORDER_TYPE_MAP_JSON")
    }
}

async fn lock_target_configuration(
    tx: &mut Transaction<'_, Postgres>,
    settings: &BdpBootstrapSettings,
) -> Result<(Uuid, bool), String> {
    let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE LOWER(email) = LOWER($1)")
        .bind(settings.user_email.trim())
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| format!("No se pudo buscar la cuenta BDP objetivo: {error}"))?
        .ok_or_else(|| {
            format!(
                "BDP_BOOTSTRAP_USER_EMAIL no corresponde a una cuenta existente: {}",
                settings.user_email
            )
        })?;
    sqlx::query(
        "INSERT INTO configuracion_restaurante (id, user_id)
         VALUES ($1, $2) ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(|error| format!("No se pudo crear configuración para bootstrap BDP: {error}"))?;
    let applied_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT bdp_env_bootstrap_applied_at
         FROM configuracion_restaurante WHERE user_id = $1 FOR UPDATE",
    )
    .bind(user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|error| format!("No se pudo bloquear configuración BDP: {error}"))?;
    Ok((user_id, applied_at.is_some()))
}

async fn apply_safe_configuration(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    settings: &BdpBootstrapSettings,
) -> Result<(String, bool), String> {
    let unconfigured: bool = sqlx::query_scalar(
        "SELECT BTRIM(bdp_base_url) = '' AND BTRIM(bdp_login) = ''
                AND BTRIM(bdp_integrator_code) = ''
         FROM configuracion_restaurante WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|error| format!("No se pudo evaluar configuración BDP previa: {error}"))?;
    let target_base_url: String = sqlx::query_scalar(
        "UPDATE configuracion_restaurante SET
            bdp_base_url = CASE WHEN BTRIM(bdp_base_url) = '' THEN $2 ELSE bdp_base_url END,
            bdp_login = CASE WHEN BTRIM(bdp_login) = '' THEN $3 ELSE bdp_login END,
            bdp_password = CASE WHEN BTRIM(bdp_password) = '' THEN $4 ELSE bdp_password END,
            bdp_integrator_code = CASE WHEN BTRIM(bdp_integrator_code) = '' THEN $5 ELSE bdp_integrator_code END,
            bdp_pos_id = CASE WHEN $14 THEN $6 ELSE bdp_pos_id END,
            bdp_employee_id = CASE WHEN $14 THEN $7 ELSE bdp_employee_id END,
            bdp_items_profile_id = CASE WHEN $14 THEN $8 ELSE bdp_items_profile_id END,
            bdp_default_article_code = CASE WHEN BTRIM(bdp_default_article_code) = ''
                OR UPPER(BTRIM(bdp_default_article_code)) = 'GLORY' THEN $9 ELSE bdp_default_article_code END,
            bdp_default_article_name = CASE WHEN BTRIM(bdp_default_article_name) = ''
                OR bdp_default_article_name = 'Servicio Glory' THEN $10 ELSE bdp_default_article_name END,
            bdp_tender_map = CASE WHEN bdp_tender_map = '{}'::jsonb THEN $11 ELSE bdp_tender_map END,
            bdp_order_type_map = CASE WHEN bdp_order_type_map = '{}'::jsonb THEN $12 ELSE bdp_order_type_map END,
            bdp_default_customer_code = CASE WHEN BTRIM(bdp_default_customer_code) = '' THEN $13 ELSE bdp_default_customer_code END,
            bdp_poll_interval_secs = CASE WHEN $14 THEN $15 ELSE bdp_poll_interval_secs END,
            bdp_sync_enabled = FALSE,
            bdp_poll_enabled = FALSE,
            bdp_auto_sync_customers = FALSE,
            bdp_sync_mode = 'read_only', bdp_env_bootstrap_applied_at = NOW(), updated_at = NOW()
         WHERE user_id = $1 RETURNING bdp_base_url",
    )
    .bind(user_id)
    .bind(settings.base_url.trim().trim_end_matches('/'))
    .bind(settings.login.trim())
    .bind(settings.password.as_str())
    .bind(settings.integrator_code.trim())
    .bind(settings.pos_id)
    .bind(settings.employee_id)
    .bind(settings.items_profile_id)
    .bind(settings.default_article_code.trim())
    .bind(settings.default_article_name.trim())
    .bind(&settings.tender_map)
    .bind(&settings.order_type_map)
    .bind(settings.default_customer_code.trim())
    .bind(unconfigured)
    .bind(settings.poll_interval_secs)
    .fetch_one(&mut **tx)
    .await
    .map_err(|error| format!("No se pudo aplicar bootstrap BDP: {error}"))?;
    Ok((target_base_url, !unconfigured))
}

async fn close_write_permissions(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<(), String> {
    sqlx::query("DELETE FROM bdp_write_arming WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut **tx)
        .await
        .map_err(|error| format!("No se pudo cerrar autorización BDP previa: {error}"))?;
    Ok(())
}

async fn audit_bootstrap(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    target_base_url: &str,
    preserved_existing: bool,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO bdp_audit_log
            (user_id, operacion, direccion, datos_enviados, resultado,
             target_base_url, authorization_reason)
         VALUES ($1, 'config_bootstrap', 'internal', $2, 'exito', $3, $4)",
    )
    .bind(user_id)
    .bind(serde_json::json!({
        "source": "server_environment",
        "preserved_existing_values": preserved_existing,
        "write_mode": "read_only"
    }))
    .bind(target_base_url)
    .bind("Aprovisionamiento automático dirigido desde secretos del servidor")
    .execute(&mut **tx)
    .await
    .map_err(|error| format!("No se pudo auditar bootstrap BDP: {error}"))?;
    Ok(())
}

fn env_optional(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn env_required(name: &str) -> Result<String, String> {
    env_optional(name).ok_or_else(|| format!("{name} es obligatorio para el bootstrap BDP"))
}

fn env_positive_i32(name: &str) -> Result<i32, String> {
    env_required(name)?
        .parse::<i32>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{name} debe ser un entero positivo"))
}

fn env_json_map(name: &str) -> Result<Value, String> {
    let raw = env_optional(name).unwrap_or_else(|| "{}".to_string());
    serde_json::from_str(&raw).map_err(|_| format!("{name} debe contener un objeto JSON válido"))
}

fn validate_map(value: &Value, minimum: i64, name: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{name} debe contener un objeto JSON"))?;
    for (key, value) in object {
        let parsed = value
            .as_i64()
            .or_else(|| value.as_str()?.trim().parse::<i64>().ok());
        if key.trim().is_empty() || parsed.is_none_or(|item| item < minimum) {
            return Err(format!("{name} contiene un código inválido en '{key}'"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{BdpBootstrapSettings, BdpConfigBootstrapService};

    fn valid_settings() -> BdpBootstrapSettings {
        BdpBootstrapSettings {
            user_email: "restaurante@example.com".into(),
            base_url: "http://127.0.0.1:8068".into(),
            login: "admin".into(),
            password: "secret".into(),
            integrator_code: "integrator".into(),
            pos_id: 31,
            employee_id: 1,
            items_profile_id: 1,
            default_article_code: "1001".into(),
            default_article_name: "Servicio".into(),
            tender_map: serde_json::json!({"efectivo": 1}),
            order_type_map: serde_json::json!({"comedor": 1}),
            default_customer_code: "1".into(),
            poll_interval_secs: 60,
        }
    }

    #[test]
    fn validacion_rechaza_placeholder_y_url_con_ruta() {
        let mut settings = valid_settings();
        settings.default_article_code = "GLORY".into();
        assert!(BdpConfigBootstrapService::validate(&settings).is_err());

        let mut settings = valid_settings();
        settings.base_url = "http://127.0.0.1:8068/api".into();
        assert!(BdpConfigBootstrapService::validate(&settings).is_err());
    }

    #[test]
    fn validacion_acepta_configuracion_dirigida_segura() {
        assert!(BdpConfigBootstrapService::validate(&valid_settings()).is_ok());
    }
}
