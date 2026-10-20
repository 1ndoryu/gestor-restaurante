// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
/* [por que] sqlx sin feature "macros" ni base en tiempo de compilación: `query!`
 * rompería el build. El SQL se prueba contra la base real en `tests/`.
 *
 * [199A-1/F1] El buzón de avisos: lo que el TPV deja y la web mira. Parte de
 * `bdp_avisos`; ver la cabecera del módulo para el contrato. */

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use super::campos::AvisoPreparado;
/* --------------------------------------------------------------------------
 * Buzón
 * -------------------------------------------------------------------------- */

/// Una fila del buzón, tal como se expone en la API.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct FilaAviso {
    pub id: Uuid,
    pub accion: String,
    pub origen: Option<String>,
    pub clave_idempotencia: String,
    pub payload: Value,
    pub estado: String,
    pub intentos: i32,
    pub ultimo_error: Option<String>,
    pub recibido_en: DateTime<Utc>,
    pub procesado_en: Option<DateTime<Utc>>,
}

/// Qué ha pasado al meter el aviso en el buzón.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultadoInbox {
    Nuevo(Uuid),
    /// Ya estaba: mismo aviso reenviado. No se toca la fila.
    Duplicado,
    /// Misma clave y cuerpo distinto: no se pisa el original y se avisa.
    Conflicto,
}

pub struct BdpAvisosService;

impl BdpAvisosService {
    /// Cuenta a la que pertenecen los avisos. La configuración es **por usuario**
    /// (cada cuenta que entra en la web se lleva la suya), así que «la cuenta del
    /// restaurante» hay que deducirla; nunca suponerla.
    ///
    /// [D16] Dos pistas, en orden. La comanda del aviso: la venta que la tiene
    /// apuntada es, sin discusión, su dueña. Y si el aviso no trae comanda o su
    /// venta todavía no existe, la cuenta con la integración BDP en marcha, que
    /// es la que tiene un TPV emitiendo avisos. Cuando la pista no da exactamente
    /// una cuenta esto para en vez de repartir avisos al azar (fail-closed).
    pub async fn resolver_usuario(pool: &PgPool, comanda: Option<i64>) -> Result<Uuid, String> {
        if let Some(comanda) = comanda {
            let duenas: Vec<Uuid> = sqlx::query_scalar(
                "SELECT DISTINCT user_id FROM ventas WHERE bdp_order_id = $1 ORDER BY user_id",
            )
            .bind(comanda)
            .fetch_all(pool)
            .await
            .map_err(|error| {
                format!("No se pudo leer la venta de la comanda {comanda}: {error}")
            })?;
            /* Sin venta local (la comanda pudo crearse en otro TPV) o con más de
             * una dueña, la comanda no decide: se sigue por la configuración. */
            if let [unica] = duenas.as_slice() {
                return Ok(*unica);
            }
        }

        let en_bdp: Vec<Uuid> = sqlx::query_scalar(
            "SELECT user_id FROM configuracion_restaurante \
              WHERE bdp_sync_enabled \
                AND btrim(bdp_base_url) <> '' AND btrim(bdp_login) <> '' \
                AND btrim(bdp_password) <> '' AND btrim(bdp_integrator_code) <> '' \
              ORDER BY user_id",
        )
        .fetch_all(pool)
        .await
        .map_err(|error| format!("No se pudo leer la configuración del restaurante: {error}"))?;

        /* Sin integración configurada en ninguna cuenta se cae a la configuración
         * a secas: es la base recién sembrada, con una sola. */
        let cuentas: Vec<Uuid> = if en_bdp.is_empty() {
            sqlx::query_scalar("SELECT user_id FROM configuracion_restaurante ORDER BY user_id")
                .fetch_all(pool)
                .await
                .map_err(|error| {
                    format!("No se pudo leer la configuración del restaurante: {error}")
                })?
        } else {
            en_bdp
        };

        match cuentas.as_slice() {
            [unica] => Ok(*unica),
            [] => Err(
                "No hay ninguna configuración de restaurante: el receptor de avisos no sabe a qué cuenta apuntar."
                    .to_string(),
            ),
            _ => Err(
                "Hay más de una cuenta con la integración BDP en marcha: el receptor de avisos no puede decidir el destino."
                    .to_string(),
            ),
        }
    }

    /// Mete el aviso en el buzón. La clave única `(user_id, clave_idempotencia)`
    /// es la que impide que un reenvío mueva nada dos veces.
    pub async fn recibir(
        pool: &PgPool,
        user_id: Uuid,
        origen: Option<&str>,
        aviso: &AvisoPreparado,
    ) -> Result<ResultadoInbox, String> {
        let id = Uuid::new_v4();
        let insertado: Option<Uuid> = sqlx::query_scalar(
            r"INSERT INTO bdp_avisos_inbox
                 (id, user_id, accion, origen, clave_idempotencia, payload, payload_hash,
                  estado, ultimo_error)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
               ON CONFLICT (user_id, clave_idempotencia) DO NOTHING
               RETURNING id",
        )
        .bind(id)
        .bind(user_id)
        .bind(aviso.accion.slug())
        .bind(origen)
        .bind(&aviso.clave_idempotencia)
        .bind(&aviso.payload)
        .bind(&aviso.payload_hash)
        .bind(aviso.estado)
        .bind(aviso.nota.as_deref())
        .fetch_optional(pool)
        .await
        .map_err(|error| format!("No se pudo guardar el aviso en el buzón: {error}"))?;

        if let Some(id) = insertado {
            return Ok(ResultadoInbox::Nuevo(id));
        }

        /* Ya existía: se compara la huella para distinguir el reenvío legítimo
         * (mismo cuerpo) del reenvío con otro contenido, que sí es un problema. */
        let huella: Option<String> = sqlx::query_scalar(
            "SELECT payload_hash FROM bdp_avisos_inbox \
             WHERE user_id = $1 AND clave_idempotencia = $2",
        )
        .bind(user_id)
        .bind(&aviso.clave_idempotencia)
        .fetch_optional(pool)
        .await
        .map_err(|error| format!("No se pudo comprobar el aviso duplicado: {error}"))?;

        match huella.as_deref() {
            Some(huella) if huella == aviso.payload_hash => Ok(ResultadoInbox::Duplicado),
            Some(_) => Ok(ResultadoInbox::Conflicto),
            /* Carrera con un borrado: se trata como duplicado y se deja rastro. */
            None => Ok(ResultadoInbox::Duplicado),
        }
    }

    /// Avisos de la cuenta, del más reciente al más antiguo.
    pub async fn listar(
        pool: &PgPool,
        user_id: Uuid,
        estado: Option<&str>,
        limite: i64,
    ) -> Result<Vec<FilaAviso>, String> {
        sqlx::query_as::<_, FilaAviso>(
            r"SELECT id, accion, origen, clave_idempotencia, payload, estado, intentos,
                      ultimo_error, recibido_en, procesado_en
                 FROM bdp_avisos_inbox
                WHERE user_id = $1
                  AND ($2::text IS NULL OR estado = $2)
                ORDER BY recibido_en DESC
                LIMIT $3",
        )
        .bind(user_id)
        .bind(estado)
        .bind(limite.clamp(1, 200))
        .fetch_all(pool)
        .await
        .map_err(|error| format!("No se pudieron listar los avisos: {error}"))
    }

    /// Devuelve a la cola un aviso que quedó pendiente de revisión. F2 (el
    /// worker) es quien lo procesa; aquí solo se reabre.
    pub async fn reintentar(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<bool, String> {
        let filas = sqlx::query(
            "UPDATE bdp_avisos_inbox \
                SET estado = 'recibido', intentos = 0, ultimo_error = NULL \
              WHERE id = $1 AND user_id = $2 AND estado = 'requiere_revision'",
        )
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|error| format!("No se pudo reabrir el aviso: {error}"))?
        .rows_affected();

        Ok(filas > 0)
    }
}
