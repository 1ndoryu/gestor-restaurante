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
