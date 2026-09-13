// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [267A-7] Repositorio de administración (modo demo).
 * Contiene el SQL del reset de datos; el handler solo valida DEMO_MODE y delega. */

use sqlx::PgPool;
use uuid::Uuid;

pub struct AdminRepository;

impl AdminRepository {
    /* [044A-3] Orden de eliminación respeta FK constraints.
     * Primero tablas-hoja (junction tables, dependientes), luego padres.
     * ON DELETE CASCADE resolvería algunos, pero eliminamos explícitamente
     * para no depender de que el cascade opere en el orden correcto. */
    pub async fn eliminar_datos_usuario(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
        let sentencias = [
            "DELETE FROM combinacion_mesa_items WHERE mesa_id IN (SELECT id FROM mesas WHERE zona_id IN (SELECT id FROM zonas_sala WHERE user_id = $1))",
            "DELETE FROM campana_destinatarios WHERE campana_id IN (SELECT id FROM campanas WHERE user_id = $1)",
            "DELETE FROM recordatorios_enviados WHERE regla_id IN (SELECT id FROM reglas_recordatorio WHERE user_id = $1)",
            "DELETE FROM reservas_etiquetas WHERE reserva_id IN (SELECT id FROM reservas WHERE user_id = $1)",
            "DELETE FROM clientes_etiquetas WHERE cliente_id IN (SELECT id FROM clientes WHERE user_id = $1)",
            "DELETE FROM notificaciones WHERE user_id = $1",
            "DELETE FROM reglas_recordatorio WHERE user_id = $1",
            "DELETE FROM campanas WHERE user_id = $1",
            "DELETE FROM plantillas_whatsapp WHERE user_id = $1",
            "DELETE FROM ventas WHERE user_id = $1",
            "DELETE FROM gastos WHERE user_id = $1",
            "DELETE FROM reservas WHERE user_id = $1",
            "DELETE FROM clientes WHERE user_id = $1",
            "DELETE FROM canales_reserva WHERE user_id = $1",
            "DELETE FROM combinaciones_mesas WHERE user_id = $1",
            "DELETE FROM mesas WHERE zona_id IN (SELECT id FROM zonas_sala WHERE user_id = $1)",
            "DELETE FROM zonas_sala WHERE user_id = $1",
            "DELETE FROM etiquetas WHERE user_id = $1",
            "DELETE FROM categorias_etiqueta WHERE user_id = $1",
            "DELETE FROM api_keys WHERE user_id = $1",
        ];
        for sql in &sentencias {
            sqlx::query(sql).bind(user_id).execute(pool).await?;
        }
        Ok(())
    }
}
