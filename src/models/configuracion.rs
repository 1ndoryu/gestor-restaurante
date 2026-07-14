/* [263A-17] Modelo de configuración del restaurante.
 * Campos obligatorios al reservar + IVA por defecto + nombre restaurante.
 * [014A-1] auto_venta_reserva: al completar reserva, crear venta automáticamente.
 * [014A-4] Turnos configurables: horas de desayuno, comida, cena.
 * [034A-3] url_haddock: enlace configurable a plataforma externa Haddock.
 * [065A-2] bdp_*: credenciales y parametros operativos de WebLink REST API. */

use chrono::{DateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Configuración almacenada del restaurante
#[derive(Debug, Clone, Default, FromRow, Serialize, ToSchema)]
#[allow(clippy::struct_excessive_bools)]
pub struct ConfiguracionRestaurante {
    pub id: Uuid,
    pub user_id: Uuid,
    pub reserva_email_obligatorio: bool,
    pub reserva_telefono_obligatorio: bool,
    pub reserva_nombre_obligatorio: bool,
    pub reserva_apellidos_obligatorio: bool,
    pub iva_por_defecto: rust_decimal::Decimal,
    pub nombre_restaurante: String,
    /* [283A-8] API key de Groq para digitalización de documentos. */
    #[serde(skip_serializing)]
    pub groq_api_key: Option<String>,
    /* [014A-1] Si true, al marcar reserva como "completada" se crea venta automáticamente */
    pub auto_venta_reserva: bool,
    /* [014A-4] Rangos horarios de turnos (configurables) */
    pub hora_desayuno_inicio: NaiveTime,
    pub hora_desayuno_fin: NaiveTime,
    pub hora_comida_inicio: NaiveTime,
    pub hora_comida_fin: NaiveTime,
    pub hora_cena_inicio: NaiveTime,
    pub hora_cena_fin: NaiveTime,
    /* [034A-3] URL de la plataforma Haddock para vista detallada */
    pub url_haddock: String,
    /* [064A-5] Token Base64 de autenticación para Haddock POS API (Basic Auth).
     * Se obtiene desde Haddock > Configuración > Integraciones > POS API. */
    #[serde(skip_serializing)]
    pub haddock_api_token: String,
    /* [064A-5] Toggle para activar/desactivar la sincronización de ventas con Haddock */
    pub haddock_sync_enabled: bool,
    /* [065A-2] Base publica del WebLink REST API instalado en el PC del restaurante. */
    pub bdp_base_url: String,
    #[serde(skip_serializing)]
    pub bdp_login: String,
    #[serde(skip_serializing)]
    pub bdp_password: String,
    #[serde(skip_serializing)]
    pub bdp_integrator_code: String,
    pub bdp_sync_enabled: bool,
    pub bdp_pos_id: i32,
    pub bdp_employee_id: i32,
    pub bdp_items_profile_id: i32,
    /* [065A-5] Artículo BDP genérico para el mapeo Glory → BDP.
     * Si el código es numérico, se usa directamente como ArtCode.
     * Si no, se busca el primer artículo del perfil. */
    pub bdp_default_article_code: String,
    pub bdp_default_article_name: String,
    /* [F1.2] Mapeos BDP configurables por el usuario.
     * bdp_tender_map: {"efectivo": "1", "tarjeta": "2"} — método_pago Glory → código tend BDP.
     * bdp_order_type_map: {"comedor": "0", "barra": "0"} — canal_venta Glory → código tipo pedido BDP.
     * bdp_default_customer_code: código cliente BDP para ventas sin cliente asociado. */
    pub bdp_tender_map: serde_json::Value,
    pub bdp_order_type_map: serde_json::Value,
    pub bdp_default_customer_code: String,
    /* [F4.5] Intervalo de polling para consultar estado de comandas BDP (segundos) */
    pub bdp_poll_interval_secs: i32,
    /* [094A-4] URL de Google Business para redirigir reseñas positivas */
    pub google_review_url: String,
    /* [094A-6] Datos para botones CTA en mensajes WhatsApp */
    pub telefono_restaurante: String,
    pub url_reservas: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request para actualizar la configuración
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ActualizarConfiguracionRequest {
    pub reserva_email_obligatorio: Option<bool>,
    pub reserva_telefono_obligatorio: Option<bool>,
    pub reserva_nombre_obligatorio: Option<bool>,
    pub reserva_apellidos_obligatorio: Option<bool>,
    #[validate(custom(function = "validar_iva"))]
    pub iva_por_defecto: Option<rust_decimal::Decimal>,
    #[validate(length(max = 255))]
    pub nombre_restaurante: Option<String>,
    /* [283A-8] API key de Groq */
    #[validate(length(max = 200))]
    pub groq_api_key: Option<String>,
    /* [014A-1] Toggle auto-venta al completar reserva */
    pub auto_venta_reserva: Option<bool>,
    /* [014A-4] Rangos horarios configurables */
    pub hora_desayuno_inicio: Option<NaiveTime>,
    pub hora_desayuno_fin: Option<NaiveTime>,
    pub hora_comida_inicio: Option<NaiveTime>,
    pub hora_comida_fin: Option<NaiveTime>,
    pub hora_cena_inicio: Option<NaiveTime>,
    pub hora_cena_fin: Option<NaiveTime>,
    /* [034A-3] URL de la plataforma Haddock (vacía = sin enlace) */
    #[validate(length(max = 500))]
    pub url_haddock: Option<String>,
    /* [064A-5] Token API de Haddock (Basic Auth, Base64) */
    #[validate(length(max = 500))]
    pub haddock_api_token: Option<String>,
    /* [064A-5] Activar sincronización de ventas con Haddock */
    pub haddock_sync_enabled: Option<bool>,
    /* [065A-2] Configuracion BDP WebLink REST API */
    #[validate(length(max = 500))]
    pub bdp_base_url: Option<String>,
    #[validate(length(max = 100))]
    pub bdp_login: Option<String>,
    #[validate(length(max = 200))]
    pub bdp_password: Option<String>,
    #[validate(length(max = 100))]
    pub bdp_integrator_code: Option<String>,
    pub bdp_sync_enabled: Option<bool>,
    #[validate(range(min = 1, max = 999_999))]
    pub bdp_pos_id: Option<i32>,
    #[validate(range(min = 1, max = 999_999))]
    pub bdp_employee_id: Option<i32>,
    #[validate(range(min = 1, max = 999_999))]
    pub bdp_items_profile_id: Option<i32>,
    /* [094A-4] URL de Google Business para reseñas positivas */
    #[validate(length(max = 500))]
    pub google_review_url: Option<String>,
    /* [094A-6] Teléfono del restaurante para botón "Llámanos" */
    #[validate(length(max = 30))]
    pub telefono_restaurante: Option<String>,
    /* [094A-6] URL de reservas para botón "Reserva ya" */
    #[validate(length(max = 500))]
    pub url_reservas: Option<String>,
    /* [F1.2] Mapeos BDP — PATCH parcial */
    pub bdp_tender_map: Option<serde_json::Value>,
    pub bdp_order_type_map: Option<serde_json::Value>,
    #[validate(length(max = 100))]
    pub bdp_default_customer_code: Option<String>,
    /* [F4.5] Intervalo de polling BDP */
    pub bdp_poll_interval_secs: Option<i32>,
}

fn validar_iva(valor: &rust_decimal::Decimal) -> Result<(), validator::ValidationError> {
    let cero = rust_decimal::Decimal::ZERO;
    let cien = rust_decimal::Decimal::ONE_HUNDRED;
    if *valor < cero || *valor > cien {
        let mut err = validator::ValidationError::new("rango_iva");
        err.message = Some("IVA debe estar entre 0 y 100%".into());
        return Err(err);
    }
    Ok(())
}
