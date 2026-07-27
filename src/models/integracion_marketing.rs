/* [283A-23] Modelo de integraciones de marketing.
 * Almacena credentials de terceros (SMTP, Twilio, Meta WhatsApp) por usuario.
 * Las credenciales no se serializan en respuestas GET por seguridad —
 * solo se devuelve si están configuradas o no. */

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
#[cfg(test)]
use std::sync::{LazyLock, Mutex};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

const META_ENV_WABA_ID: &str = "META_WABA_ID";
const META_ENV_BUSINESS_APP_ID: &str = "META_BUSINESS_APP_ID";
const META_ENV_ACCESS_TOKEN: &str = "META_ACCESS_TOKEN";
const META_ENV_PHONE_NUMBER_ID: &str = "META_PHONE_NUMBER_ID";

/// Integraciones de marketing almacenadas por usuario
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct IntegracionMarketing {
    pub id: Uuid,
    pub user_id: Uuid,

    /* SMTP — se ocultan en respuesta JSON */
    #[serde(skip_serializing)]
    pub smtp_host: Option<String>,
    #[serde(skip_serializing)]
    pub smtp_port: Option<i32>,
    #[serde(skip_serializing)]
    pub smtp_user: Option<String>,
    #[serde(skip_serializing)]
    pub smtp_password: Option<String>,
    #[serde(skip_serializing)]
    pub smtp_from_email: Option<String>,
    #[serde(skip_serializing)]
    pub smtp_from_name: Option<String>,

    /* Twilio — se ocultan en respuesta JSON */
    #[serde(skip_serializing)]
    pub twilio_account_sid: Option<String>,
    #[serde(skip_serializing)]
    pub twilio_auth_token: Option<String>,
    #[serde(skip_serializing)]
    pub twilio_from_number: Option<String>,

    /* Meta WhatsApp — se ocultan en respuesta JSON */
    #[serde(skip_serializing)]
    pub meta_waba_id: Option<String>,
    #[serde(skip_serializing)]
    pub meta_business_app_id: Option<String>,
    #[serde(skip_serializing)]
    pub meta_access_token: Option<String>,
    /* [303A-1] Phone Number ID: requerido para enviar mensajes via Cloud API.
     * POST /{Version}/{Phone-Number-ID}/messages */
    #[serde(skip_serializing)]
    pub meta_phone_number_id: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl IntegracionMarketing {
    #[must_use]
    pub fn with_env_fallback(mut self) -> Self {
        self.meta_waba_id = prefer_env(self.meta_waba_id, META_ENV_WABA_ID);
        self.meta_business_app_id = prefer_env(self.meta_business_app_id, META_ENV_BUSINESS_APP_ID);
        self.meta_access_token = prefer_env(self.meta_access_token, META_ENV_ACCESS_TOKEN);
        self.meta_phone_number_id = prefer_env(self.meta_phone_number_id, META_ENV_PHONE_NUMBER_ID);
        self
    }

    /// Indica si SMTP está configurado (`host` + `user` + `password` presentes)
    #[must_use]
    pub fn smtp_configurado(&self) -> bool {
        self.smtp_host.is_some() && self.smtp_user.is_some() && self.smtp_password.is_some()
    }

    /// Indica si Twilio está configurado
    #[must_use]
    pub fn twilio_configurado(&self) -> bool {
        self.twilio_account_sid.is_some()
            && self.twilio_auth_token.is_some()
            && self.twilio_from_number.is_some()
    }

    /// Indica si Meta `WhatsApp` está configurado para enviar mensajes.
    /// Requiere `waba_id` (para templates), `access_token` y `phone_number_id` (para mensajes).
    #[must_use]
    pub fn meta_configurado(&self) -> bool {
        self.meta_waba_id.is_some()
            && self.meta_access_token.is_some()
            && self.meta_phone_number_id.is_some()
    }

    /// Indica si Meta está configurado al menos para enviar templates a aprobación.
    /// Solo requiere `waba_id` + `access_token` (no necesita `phone_number_id`).
    #[must_use]
    pub fn meta_templates_configurado(&self) -> bool {
        self.meta_waba_id.is_some() && self.meta_access_token.is_some()
    }
}

/// Vista pública: muestra solo si cada integración está configurada, sin exponer credentials
#[derive(Debug, Serialize, ToSchema)]
pub struct IntegracionMarketingPublica {
    pub id: Uuid,
    pub smtp_configurado: bool,
    pub smtp_from_email: Option<String>,
    pub smtp_from_name: Option<String>,
    pub twilio_configurado: bool,
    pub twilio_from_number: Option<String>,
    pub meta_configurado: bool,
    pub meta_waba_id: Option<String>,
    /* [303A-1] Indica si tiene phone_number_id para enviar mensajes */
    pub meta_phone_number_id: Option<String>,
}

impl From<&IntegracionMarketing> for IntegracionMarketingPublica {
    fn from(i: &IntegracionMarketing) -> Self {
        Self {
            id: i.id,
            smtp_configurado: i.smtp_configurado(),
            smtp_from_email: i.smtp_from_email.clone(),
            smtp_from_name: i.smtp_from_name.clone(),
            twilio_configurado: i.twilio_configurado(),
            twilio_from_number: i.twilio_from_number.clone(),
            meta_configurado: i.meta_configurado(),
            meta_waba_id: i.meta_waba_id.clone(),
            meta_phone_number_id: i.meta_phone_number_id.clone(),
        }
    }
}

fn prefer_env(current: Option<String>, env_name: &str) -> Option<String> {
    match current.and_then(non_empty_string) {
        Some(value) => Some(value),
        None => std::env::var(env_name).ok().and_then(non_empty_string),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn non_empty_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Request para actualizar integraciones de marketing
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ActualizarIntegracionesRequest {
    #[validate(length(max = 255))]
    pub smtp_host: Option<String>,
    pub smtp_port: Option<i32>,
    #[validate(length(max = 255))]
    pub smtp_user: Option<String>,
    #[validate(length(max = 255))]
    pub smtp_password: Option<String>,
    #[validate(length(max = 255))]
    pub smtp_from_email: Option<String>,
    #[validate(length(max = 100))]
    pub smtp_from_name: Option<String>,

    #[validate(length(max = 100))]
    pub twilio_account_sid: Option<String>,
    #[validate(length(max = 100))]
    pub twilio_auth_token: Option<String>,
    #[validate(length(max = 20))]
    pub twilio_from_number: Option<String>,

    #[validate(length(max = 100))]
    pub meta_waba_id: Option<String>,
    #[validate(length(max = 100))]
    pub meta_business_app_id: Option<String>,
    pub meta_access_token: Option<String>,
    /* [303A-1] Phone Number ID de Meta — requerido para enviar mensajes */
    #[validate(length(max = 100))]
    pub meta_phone_number_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    fn integracion_vacia() -> IntegracionMarketing {
        IntegracionMarketing {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            smtp_host: None,
            smtp_port: None,
            smtp_user: None,
            smtp_password: None,
            smtp_from_email: None,
            smtp_from_name: None,
            twilio_account_sid: None,
            twilio_auth_token: None,
            twilio_from_number: None,
            meta_waba_id: None,
            meta_business_app_id: None,
            meta_access_token: None,
            meta_phone_number_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn snapshot_meta_env() -> [Option<String>; 4] {
        [
            std::env::var(META_ENV_WABA_ID).ok(),
            std::env::var(META_ENV_BUSINESS_APP_ID).ok(),
            std::env::var(META_ENV_ACCESS_TOKEN).ok(),
            std::env::var(META_ENV_PHONE_NUMBER_ID).ok(),
        ]
    }

    fn restore_meta_env(values: &[Option<String>; 4]) {
        for (name, value) in [
            (META_ENV_WABA_ID, values[0].clone()),
            (META_ENV_BUSINESS_APP_ID, values[1].clone()),
            (META_ENV_ACCESS_TOKEN, values[2].clone()),
            (META_ENV_PHONE_NUMBER_ID, values[3].clone()),
        ] {
            match value {
                Some(existing) => std::env::set_var(name, existing),
                None => std::env::remove_var(name),
            }
        }
    }

    #[test]
    fn env_fallback_completa_meta_si_bd_esta_vacia() {
        let _guard = ENV_LOCK.lock().unwrap();
        let previous = snapshot_meta_env();

        std::env::set_var(META_ENV_WABA_ID, "1330208045872639");
        std::env::set_var(META_ENV_BUSINESS_APP_ID, "2109664896264441");
        std::env::set_var(META_ENV_ACCESS_TOKEN, "token-meta");
        std::env::set_var(META_ENV_PHONE_NUMBER_ID, "1023208230887075");

        let effective = integracion_vacia().with_env_fallback();

        assert_eq!(effective.meta_waba_id.as_deref(), Some("1330208045872639"));
        assert_eq!(
            effective.meta_business_app_id.as_deref(),
            Some("2109664896264441")
        );
        assert_eq!(effective.meta_access_token.as_deref(), Some("token-meta"));
        assert_eq!(
            effective.meta_phone_number_id.as_deref(),
            Some("1023208230887075")
        );
        assert!(effective.meta_configurado());

        restore_meta_env(&previous);
    }

    #[test]
    fn env_fallback_no_pisa_valores_ya_guardados() {
        let _guard = ENV_LOCK.lock().unwrap();
        let previous = snapshot_meta_env();

        std::env::set_var(META_ENV_WABA_ID, "desde-env");

        let effective = IntegracionMarketing {
            meta_waba_id: Some("desde-bd".to_string()),
            ..integracion_vacia()
        }
        .with_env_fallback();

        assert_eq!(effective.meta_waba_id.as_deref(), Some("desde-bd"));

        restore_meta_env(&previous);
    }
}
