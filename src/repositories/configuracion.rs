// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [263A-17] Repositorio de configuración del restaurante.
 * Upsert: si no existe, crea con defaults; si existe, actualiza parcialmente.
 * [094A-4] Convertido a queries dinámicas para evitar problemas con SQLX_OFFLINE
 * al agregar google_review_url.
 * [065A-2] Agrega credenciales y parametros operativos BDP/WebLink. */

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{ActualizarConfiguracionRequest, ConfiguracionRestaurante};

/* [287A-5] El contrato SQL vive fuera del método para que la operación de
 * persistencia sea legible sin ocultar ni silenciar el límite de Clippy. */
const UPDATE_CONFIG_SQL: &str = "UPDATE configuracion_restaurante SET \
    reserva_email_obligatorio = COALESCE($2, reserva_email_obligatorio), \
    reserva_telefono_obligatorio = COALESCE($3, reserva_telefono_obligatorio), \
    reserva_nombre_obligatorio = COALESCE($4, reserva_nombre_obligatorio), \
    reserva_apellidos_obligatorio = COALESCE($5, reserva_apellidos_obligatorio), \
    iva_por_defecto = COALESCE($6, iva_por_defecto), nombre_restaurante = COALESCE($7, nombre_restaurante), \
    groq_api_key = COALESCE($8, groq_api_key), auto_venta_reserva = COALESCE($9, auto_venta_reserva), \
    hora_desayuno_inicio = COALESCE($10, hora_desayuno_inicio), hora_desayuno_fin = COALESCE($11, hora_desayuno_fin), \
    hora_comida_inicio = COALESCE($12, hora_comida_inicio), hora_comida_fin = COALESCE($13, hora_comida_fin), \
    hora_cena_inicio = COALESCE($14, hora_cena_inicio), hora_cena_fin = COALESCE($15, hora_cena_fin), \
    url_haddock = COALESCE($16, url_haddock), haddock_api_token = COALESCE($17, haddock_api_token), \
    haddock_sync_enabled = COALESCE($18, haddock_sync_enabled), bdp_base_url = COALESCE($19, bdp_base_url), \
    bdp_login = COALESCE($20, bdp_login), bdp_password = COALESCE($21, bdp_password), \
    bdp_integrator_code = COALESCE($22, bdp_integrator_code), bdp_sync_enabled = COALESCE($23, bdp_sync_enabled), \
    bdp_pos_id = COALESCE($24, bdp_pos_id), bdp_employee_id = COALESCE($25, bdp_employee_id), \
    bdp_items_profile_id = COALESCE($26, bdp_items_profile_id), bdp_default_article_code = COALESCE($27, bdp_default_article_code), \
    bdp_default_article_name = COALESCE($28, bdp_default_article_name), google_review_url = COALESCE($29, google_review_url), \
    telefono_restaurante = COALESCE($30, telefono_restaurante), url_reservas = COALESCE($31, url_reservas), \
    bdp_tender_map = COALESCE($32, bdp_tender_map), bdp_order_type_map = COALESCE($33, bdp_order_type_map), \
    bdp_default_customer_code = COALESCE($34, bdp_default_customer_code), bdp_poll_interval_secs = COALESCE($35, bdp_poll_interval_secs), \
    bdp_poll_enabled = COALESCE($36, bdp_poll_enabled), bdp_auto_sync_customers = COALESCE($37, bdp_auto_sync_customers), \
    bdp_sync_mode = COALESCE($38, bdp_sync_mode), bdp_backup_retention_days = COALESCE($39, bdp_backup_retention_days), \
    bdp_auto_backup_before_write = COALESCE($40, bdp_auto_backup_before_write), ff_bdp_auto_arm = COALESCE($41, ff_bdp_auto_arm), \
    ff_bdp_partial_payments = COALESCE($42, ff_bdp_partial_payments), ff_bdp_cancel_order = COALESCE($43, ff_bdp_cancel_order), \
    ff_bdp_purchase_notes_read = COALESCE($44, ff_bdp_purchase_notes_read), ff_bdp_purchase_notes_draft = COALESCE($45, ff_bdp_purchase_notes_draft), \
    ff_bdp_purchase_notes_receive = COALESCE($46, ff_bdp_purchase_notes_receive), bdp_catalog_price_type = COALESCE($47, bdp_catalog_price_type), \
    bdp_purchase_notes_profile_id = COALESCE($48, bdp_purchase_notes_profile_id), \
    modo_operacion = COALESCE($49, modo_operacion), \
    anulacion_modalidad = COALESCE($50, anulacion_modalidad), \
    permisos_catalogo_edicion = COALESCE($51, permisos_catalogo_edicion), \
    permisos_stock_ajuste = COALESCE($52, permisos_stock_ajuste), \
    permisos_albaranes_gestion = COALESCE($53, permisos_albaranes_gestion), \
    permisos_anulacion_ventas = COALESCE($54, permisos_anulacion_ventas), \
    permisos_pagos_locales = COALESCE($55, permisos_pagos_locales), \
    permisos_facturacion_local = COALESCE($56, permisos_facturacion_local), \
    push_modalidad = COALESCE($57, push_modalidad), bdp_tav_map = COALESCE($58, bdp_tav_map), \
    bdp_almacen_default = COALESCE($59, bdp_almacen_default), bdp_codreg_default = COALESCE($60, bdp_codreg_default), \
    bdp_articulo_rango_inicial = COALESCE($61, bdp_articulo_rango_inicial), updated_at = NOW() \
    WHERE user_id = $1 RETURNING *";

pub struct ConfiguracionRepository;

impl ConfiguracionRepository {
    /// Obtiene la configuración del usuario. Si no existe, crea una con defaults.
    pub async fn obtener_o_crear(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<ConfiguracionRestaurante, sqlx::Error> {
        if let Some(config) = Self::obtener(pool, user_id).await? {
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

    /// Lectura pura de la configuración (sin efecto colateral de escritura).
    /// [128A-1/F8-4] `verificar_permiso` usa esta variante: un chequeo de
    /// permiso no debe crear filas de configuración (`obtener_o_crear` en cada
    /// request). Sin fila devuelve `None` (fail-closed a 'admin').
    pub async fn obtener(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Option<ConfiguracionRestaurante>, sqlx::Error> {
        sqlx::query_as::<_, ConfiguracionRestaurante>(
            "SELECT * FROM configuracion_restaurante WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
    }

    /// Actualiza parcialmente la configuración del usuario.
    pub async fn actualizar(
        pool: &PgPool,
        user_id: Uuid,
        req: &ActualizarConfiguracionRequest,
    ) -> Result<ConfiguracionRestaurante, sqlx::Error> {
        sqlx::query_as::<_, ConfiguracionRestaurante>(UPDATE_CONFIG_SQL)
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
            .bind(req.bdp_default_article_code.as_deref())
            .bind(req.bdp_default_article_name.as_deref())
            .bind(req.google_review_url.as_deref())
            .bind(req.telefono_restaurante.as_deref())
            .bind(req.url_reservas.as_deref())
            .bind(req.bdp_tender_map.as_ref())
            .bind(req.bdp_order_type_map.as_ref())
            .bind(req.bdp_default_customer_code.as_deref())
            .bind(req.bdp_poll_interval_secs)
            .bind(req.bdp_poll_enabled)
            .bind(req.bdp_auto_sync_customers)
            .bind(req.bdp_sync_mode.as_deref())
            .bind(req.bdp_backup_retention_days)
            .bind(req.bdp_auto_backup_before_write)
            .bind(req.ff_bdp_auto_arm)
            .bind(req.ff_bdp_partial_payments)
            .bind(req.ff_bdp_cancel_order)
            .bind(req.ff_bdp_purchase_notes_read)
            .bind(req.ff_bdp_purchase_notes_draft)
            .bind(req.ff_bdp_purchase_notes_receive)
            .bind(req.bdp_catalog_price_type)
            .bind(req.bdp_purchase_notes_profile_id)
            .bind(req.modo_operacion.as_deref())
            .bind(req.anulacion_modalidad.as_deref())
            .bind(req.permisos_catalogo_edicion.as_deref())
            .bind(req.permisos_stock_ajuste.as_deref())
            .bind(req.permisos_albaranes_gestion.as_deref())
            .bind(req.permisos_anulacion_ventas.as_deref())
            .bind(req.permisos_pagos_locales.as_deref())
            .bind(req.permisos_facturacion_local.as_deref())
            .bind(req.push_modalidad.as_deref())
            .bind(req.bdp_tav_map.as_ref())
            .bind(req.bdp_almacen_default)
            .bind(req.bdp_codreg_default)
            .bind(req.bdp_articulo_rango_inicial)
            .fetch_one(pool)
            .await
    }
}
