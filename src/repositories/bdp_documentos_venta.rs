// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [L8/D28] Repositorio de documentos de venta BDP y de su cursor de lectura.
 * El lote y el cursor se escriben en una transacción: si falla algo, el cursor
 * no avanza y la siguiente ventana vuelve a pedir el mismo rango. */

use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

/// Documento ya validado y listo para guardar; `datos` conserva el JSON original.
pub struct DocumentoVentaLocal {
    pub bdp_documento_id: String,
    pub fecha: Option<NaiveDate>,
    pub datos: serde_json::Value,
}

/// Documento ya guardado, tal como lo sirve la API (sin el JSON crudo completo).
#[derive(Debug, Clone, serde::Serialize)]
pub struct DocumentoVentaGuardado {
    pub bdp_documento_id: String,
    pub perfil: i32,
    pub fecha: Option<NaiveDate>,
    /// Serie del documento si el JSON la trae; el resto de campos no está documentado.
    pub serie: Option<String>,
    pub numero: Option<String>,
    pub total: Option<serde_json::Value>,
    pub recibido_at: chrono::DateTime<chrono::Utc>,
    /// JSON original de BDP: la pantalla lo enseña en crudo al desplegar la fila.
    pub datos_bdp: serde_json::Value,
}

pub const DOMINIO_CURSOR_DOCUMENTOS: &str = "documentos";

pub struct BdpDocumentosVentaRepository;

impl BdpDocumentosVentaRepository {
    /// Último día cubierto por la lectura del perfil; `None` si nunca se ha leído.
    pub async fn leer_cursor(
        pool: &PgPool,
        user_id: Uuid,
        perfil: i32,
    ) -> Result<Option<NaiveDate>, sqlx::Error> {
        let hasta: Option<String> = sqlx::query_scalar(
            "SELECT posicion->>'hasta' FROM bdp_sync_cursor \
             WHERE user_id = $1 AND dominio = $2 AND perfil = $3",
        )
        .bind(user_id)
        .bind(DOMINIO_CURSOR_DOCUMENTOS)
        .bind(perfil.to_string())
        .fetch_optional(pool)
        .await?
        .flatten();
        // Un cursor corrupto se trata como ausente: la ventana inicial lo recupera sin huecos.
        Ok(hasta.and_then(|texto| NaiveDate::parse_from_str(&texto, "%Y-%m-%d").ok()))
    }

    /// Documentos guardados, del más reciente al más antiguo.
    ///
    /// Serie, número y total no están documentados por el distribuidor, así que
    /// salen del JSON crudo por las claves habituales: si un documento no las
    /// trae, la fila se sirve igual y la pantalla enseña el JSON.
    pub async fn listar(
        pool: &PgPool,
        user_id: Uuid,
        perfil: Option<i32>,
        limite: i64,
    ) -> Result<Vec<DocumentoVentaGuardado>, sqlx::Error> {
        /* sqlx sin macros devuelve la fila como tupla: el alias evita repetirla
         * y deja legible el orden de las columnas. */
        type Fila = (
            String,
            i32,
            Option<NaiveDate>,
            serde_json::Value,
            chrono::DateTime<chrono::Utc>,
        );
        let filas: Vec<Fila> = sqlx::query_as(concat!(
            "SELECT bdp_documento_id, perfil, fecha, datos_bdp, recibido_at ",
            "FROM bdp_documentos_venta ",
            "WHERE user_id = $1 AND ($2::int IS NULL OR perfil = $2) ",
            "ORDER BY fecha DESC NULLS LAST, recibido_at DESC ",
            "LIMIT $3",
        ))
        .bind(user_id)
        .bind(perfil)
        .bind(limite.clamp(1, 500))
        .fetch_all(pool)
        .await?;
        Ok(filas
            .into_iter()
            .map(
                |(bdp_documento_id, perfil, fecha, datos_bdp, recibido_at)| {
                    DocumentoVentaGuardado {
                        bdp_documento_id,
                        perfil,
                        fecha,
                        serie: primero_de(&datos_bdp, &["Serie", "Series", "SerieDocumento"]),
                        numero: numero_de(&datos_bdp),
                        total: total_de(&datos_bdp),
                        recibido_at,
                        datos_bdp,
                    }
                },
            )
            .collect())
    }

    /// Último intento de lectura del perfil (éxito o error). El disparo automático
    /// lo usa como reloj: la cadencia configurada se cuenta desde aquí, no desde el
    /// último éxito, para que un BDP caído no provoque una llamada cada pasada.
    pub async fn leer_ultimo_intento(
        pool: &PgPool,
        user_id: Uuid,
        perfil: i32,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>, sqlx::Error> {
        sqlx::query_scalar(concat!(
            "SELECT ultimo_intento FROM bdp_sync_cursor ",
            "WHERE user_id = $1 AND dominio = $2 AND perfil = $3",
        ))
        .bind(user_id)
        .bind(DOMINIO_CURSOR_DOCUMENTOS)
        .bind(perfil.to_string())
        .fetch_optional(pool)
        .await
    }

    /// Guarda el lote (upsert por id BDP) y avanza el cursor a `hasta`, todo en una transacción.
    pub async fn guardar_lote(
        pool: &PgPool,
        user_id: Uuid,
        perfil: i32,
        documentos: &[DocumentoVentaLocal],
        hasta: NaiveDate,
    ) -> Result<usize, sqlx::Error> {
        let mut tx = pool.begin().await?;
        for doc in documentos {
            sqlx::query(
                "INSERT INTO bdp_documentos_venta \
                 (id, user_id, perfil, bdp_documento_id, fecha, datos_bdp) \
                 VALUES ($1, $2, $3, $4, $5, $6) \
                 ON CONFLICT (user_id, perfil, bdp_documento_id) DO UPDATE SET \
                 fecha = EXCLUDED.fecha, datos_bdp = EXCLUDED.datos_bdp, recibido_at = NOW()",
            )
            .bind(Uuid::new_v4())
            .bind(user_id)
            .bind(perfil)
            .bind(&doc.bdp_documento_id)
            .bind(doc.fecha)
            .bind(&doc.datos)
            .execute(&mut *tx)
            .await?;
        }
        Self::escribir_cursor(&mut tx, user_id, perfil, Some(hasta), None).await?;
        tx.commit().await?;
        Ok(documentos.len())
    }

    /// Anota un intento fallido sin mover la posición del cursor.
    pub async fn registrar_error(
        pool: &PgPool,
        user_id: Uuid,
        perfil: i32,
        mensaje: &str,
    ) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;
        Self::escribir_cursor(&mut tx, user_id, perfil, None, Some(mensaje)).await?;
        tx.commit().await
    }

    /// Upsert del cursor. Con `hasta` marca éxito; con `error` marca el fallo y conserva la posición.
    async fn escribir_cursor(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        user_id: Uuid,
        perfil: i32,
        hasta: Option<NaiveDate>,
        error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let hasta_texto = hasta.map(|fecha| fecha.format("%Y-%m-%d").to_string());
        sqlx::query(
            "INSERT INTO bdp_sync_cursor \
             (id, user_id, dominio, perfil, posicion, ultimo_exito, ultimo_intento, ultimo_error) \
             VALUES ($1, $2, $3, $4, jsonb_build_object('hasta', $5::text), \
                     CASE WHEN $5::text IS NULL THEN NULL ELSE NOW() END, NOW(), $6) \
             ON CONFLICT (user_id, dominio, perfil) DO UPDATE SET \
             posicion = CASE WHEN $5::text IS NULL THEN bdp_sync_cursor.posicion \
                             ELSE EXCLUDED.posicion END, \
             ultimo_exito = CASE WHEN $5::text IS NULL THEN bdp_sync_cursor.ultimo_exito \
                                 ELSE NOW() END, \
             ultimo_intento = NOW(), \
             ultimo_error = EXCLUDED.ultimo_error",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(DOMINIO_CURSOR_DOCUMENTOS)
        .bind(perfil.to_string())
        .bind(hasta_texto)
        .bind(error)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}

/// Primer valor no vacío bajo cualquiera de las claves, como texto.
/* Los campos reales del perfil 1 del BDP local (comprobado 2026-10-21) son
 * `Serie`, `Numero` y `Total_Documento`, y los números llegan como decimales
 * (`12.0`): sin esto la pantalla enseñaba «12.0» y un total vacío. */
fn numero_de(datos: &serde_json::Value) -> Option<String> {
    primero_de(datos, &["Numero", "Number", "DocumentNumber"])
        .map(|texto| texto.strip_suffix(".0").unwrap_or(&texto).to_string())
}

fn total_de(datos: &serde_json::Value) -> Option<serde_json::Value> {
    ["Total_Documento", "Total", "Importe"]
        .iter()
        .find_map(|clave| datos.get(*clave).filter(|valor| !valor.is_null()).cloned())
}

fn primero_de(datos: &serde_json::Value, claves: &[&str]) -> Option<String> {
    claves.iter().find_map(|clave| match datos.get(*clave)? {
        serde_json::Value::String(texto) if !texto.trim().is_empty() => {
            Some(texto.trim().to_string())
        }
        serde_json::Value::Number(numero) => Some(numero.to_string()),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn dia(texto: &str) -> NaiveDate {
        NaiveDate::parse_from_str(texto, "%Y-%m-%d").unwrap()
    }

    fn lote(ids: &[&str]) -> Vec<DocumentoVentaLocal> {
        ids.iter()
            .map(|id| DocumentoVentaLocal {
                bdp_documento_id: id.to_string(),
                fecha: Some(dia("2026-10-19")),
                datos: json!({"Id": id}),
            })
            .collect()
    }

    /* [D28-auto-R] Reimportar una ventana solapada no duplica filas y avanza el cursor. */
    #[sqlx::test(migrations = "./migrations")]
    async fn guardar_lote_es_idempotente_y_avanza_cursor(pool: PgPool) {
        let user_id = Uuid::new_v4();
        assert_eq!(
            BdpDocumentosVentaRepository::leer_cursor(&pool, user_id, 3)
                .await
                .unwrap(),
            None
        );

        BdpDocumentosVentaRepository::guardar_lote(
            &pool,
            user_id,
            3,
            &lote(&["1", "2"]),
            dia("2026-10-19"),
        )
        .await
        .unwrap();
        BdpDocumentosVentaRepository::guardar_lote(
            &pool,
            user_id,
            3,
            &lote(&["2", "3"]),
            dia("2026-10-20"),
        )
        .await
        .unwrap();

        let filas: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM bdp_documentos_venta WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(filas, 3);
        assert_eq!(
            BdpDocumentosVentaRepository::leer_cursor(&pool, user_id, 3)
                .await
                .unwrap(),
            Some(dia("2026-10-20"))
        );
    }

    /* [D28-auto-R] Un fallo registrado no mueve la posición: la siguiente lectura repite el rango. */
    #[sqlx::test(migrations = "./migrations")]
    async fn registrar_error_conserva_la_posicion_del_cursor(pool: PgPool) {
        let user_id = Uuid::new_v4();
        BdpDocumentosVentaRepository::guardar_lote(&pool, user_id, 3, &[], dia("2026-10-19"))
            .await
            .unwrap();
        BdpDocumentosVentaRepository::registrar_error(&pool, user_id, 3, "BDP caído")
            .await
            .unwrap();

        assert_eq!(
            BdpDocumentosVentaRepository::leer_cursor(&pool, user_id, 3)
                .await
                .unwrap(),
            Some(dia("2026-10-19"))
        );
        let error: Option<String> = sqlx::query_scalar(
            "SELECT ultimo_error FROM bdp_sync_cursor WHERE user_id = $1 AND dominio = 'documentos'",
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(error.as_deref(), Some("BDP caído"));
    }

    /* Perfiles distintos llevan cursores independientes. */
    #[sqlx::test(migrations = "./migrations")]
    async fn cursores_por_perfil_son_independientes(pool: PgPool) {
        let user_id = Uuid::new_v4();
        BdpDocumentosVentaRepository::guardar_lote(&pool, user_id, 3, &[], dia("2026-10-19"))
            .await
            .unwrap();
        assert_eq!(
            BdpDocumentosVentaRepository::leer_cursor(&pool, user_id, 4)
                .await
                .unwrap(),
            None
        );
    }
}
