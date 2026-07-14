/* [263A-17] Repositorio de configuración del restaurante.
 * Upsert: si no existe, crea con defaults; si existe, actualiza parcialmente.
 * [094A-4] Convertido a queries dinámicas para evitar problemas con SQLX_OFFLINE
 * al agregar google_review_url.
 * [065A-2] Agrega credenciales y parametros operativos BDP/WebLink. */

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{ActualizarConfiguracionRequest, ConfiguracionRestaurante};

pub struct ConfiguracionRepository;

impl ConfiguracionRepository {
    /// Obtiene la configuración del usuario. Si no existe, crea una con defaults.
    pub async fn obtener_o_crear(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<ConfiguracionRestaurante, sqlx::Error> {
        let existente = sqlx::query_as::<_, ConfiguracionRestaurante>(
            "SELECT * FROM configuracion_restaurante WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

        if let Some(config) = existente {
            return Ok(config);
        }

        /* Crear con defaults */
        let id = Uuid::new_v4();
        sqlx::query_as::<_, ConfiguracionRestaurante>(
            "INSERT INTO configuracion_restaurante (id, user_id) VALUES ($1, $2) RETURNING *",
        )
        .bind(id)
        .bind(user_id)
        .fetch_one(pool)
        .await
    }

    /// Actualiza parcialmente la configuración del usuario.
    pub async fn actualizar(
        pool: &PgPool,
        user_id: Uuid,
        req: &ActualizarConfiguracionRequest,
    ) -> Result<ConfiguracionRestaurante, sqlx::Error> {
        sqlx::query_as::<_, ConfiguracionRestaurante>(
            "UPDATE configuracion_restaurante SET \
                reserva_email_obligatorio = COALESCE($2, reserva_email_obligatorio), \
                reserva_telefono_obligatorio = COALESCE($3, reserva_telefono_obligatorio), \
                reserva_nombre_obligatorio = COALESCE($4, reserva_nombre_obligatorio), \
                reserva_apellidos_obligatorio = COALESCE($5, reserva_apellidos_obligatorio), \
                iva_por_defecto = COALESCE($6, iva_por_defecto), \
                nombre_restaurante = COALESCE($7, nombre_restaurante), \
                groq_api_key = COALESCE($8, groq_api_key), \
                auto_venta_reserva = COALESCE($9, auto_venta_reserva), \
                hora_desayuno_inicio = COALESCE($10, hora_desayuno_inicio), \
                hora_desayuno_fin = COALESCE($11, hora_desayuno_fin), \
                hora_comida_inicio = COALESCE($12, hora_comida_inicio), \
                hora_comida_fin = COALESCE($13, hora_comida_fin), \
                hora_cena_inicio = COALESCE($14, hora_cena_inicio), \
                hora_cena_fin = COALESCE($15, hora_cena_fin), \
                url_haddock = COALESCE($16, url_haddock), \
                haddock_api_token = COALESCE($17, haddock_api_token), \
                haddock_sync_enabled = COALESCE($18, haddock_sync_enabled), \
                     bdp_base_url = COALESCE($19, bdp_base_url), \
                     bdp_login = COALESCE($20, bdp_login), \
                     bdp_password = COALESCE($21, bdp_password), \
                     bdp_integrator_code = COALESCE($22, bdp_integrator_code), \
                     bdp_sync_enabled = COALESCE($23, bdp_sync_enabled), \
                     bdp_pos_id = COALESCE($24, bdp_pos_id), \
                     bdp_employee_id = COALESCE($25, bdp_employee_id), \
                     bdp_items_profile_id = COALESCE($26, bdp_items_profile_id), \
                     google_review_url = COALESCE($27, google_review_url), \
                     telefono_restaurante = COALESCE($28, telefono_restaurante), \
                     url_reservas = COALESCE($29, url_reservas), \
                     bdp_tender_map = COALESCE($30, bdp_tender_map), \
                     bdp_order_type_map = COALESCE($31, bdp_order_type_map), \
                     bdp_default_customer_code = COALESCE($32, bdp_default_customer_code), \
                     bdp_poll_interval_secs = COALESCE($33, bdp_poll_interval_secs), \
                updated_at = NOW() \
             WHERE user_id = $1 RETURNING *",
        )
        .bind(user_id)
        .bind(req.reserva_email_obligatorio)
        .bind(req.reserva_telefono_obligatorio)
        .bind(req.reserva_nombre_obligatorio)
        .bind(req.reserva_apellidos_obligatorio)
        .bind(req.iva_por_defecto)
        .bind(req.nombre_restaurante.as_deref())
        .bind(req.groq_api_key.as_deref())
        .bind(req.auto_venta_reserva)
        .bind(req.hora_desayuno_inicio)
        .bind(req.hora_desayuno_fin)
        .bind(req.hora_comida_inicio)
        .bind(req.hora_comida_fin)
        .bind(req.hora_cena_inicio)
        .bind(req.hora_cena_fin)
        .bind(req.url_haddock.as_deref())
        .bind(req.haddock_api_token.as_deref())
        .bind(req.haddock_sync_enabled)
        .bind(req.bdp_base_url.as_deref())
        .bind(req.bdp_login.as_deref())
        .bind(req.bdp_password.as_deref())
        .bind(req.bdp_integrator_code.as_deref())
        .bind(req.bdp_sync_enabled)
        .bind(req.bdp_pos_id)
        .bind(req.bdp_employee_id)
        .bind(req.bdp_items_profile_id)
        .bind(req.google_review_url.as_deref())
        .bind(req.telefono_restaurante.as_deref())
        .bind(req.url_reservas.as_deref())
        .bind(req.bdp_tender_map.as_ref())
        .bind(req.bdp_order_type_map.as_ref())
        .bind(req.bdp_default_customer_code.as_deref())
        .bind(req.bdp_poll_interval_secs)
        .fetch_one(pool)
        .await
    }
}
