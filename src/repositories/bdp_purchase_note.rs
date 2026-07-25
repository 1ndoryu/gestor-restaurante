/* [247A-11] Repositorio de albaranes de compra BDP (solo lectura).
 * CRUD de cache local y upsert desde respuesta BDP. */

use sqlx::Arguments;
use sqlx::PgPool;
use std::fmt::Write as _;
use uuid::Uuid;

use crate::models::{BdpPurchaseNote, BdpPurchaseNoteListParams};

pub struct BdpPurchaseNoteRepository;

impl BdpPurchaseNoteRepository {
    /// Lista los albaranes de compra de un usuario, opcionalmente filtrados.
    pub async fn listar(
        pool: &PgPool,
        user_id: Uuid,
        params: &BdpPurchaseNoteListParams,
    ) -> Result<Vec<BdpPurchaseNote>, sqlx::Error> {
        let mut query = String::from(
            "SELECT * FROM bdp_purchase_notes WHERE user_id = $1",
        );
        let mut args = sqlx::postgres::PgArguments::default();
        let _ = args.add(user_id);
        let mut param_idx: usize = 1;

        if let Some(proveedor) = &params.proveedor {
            param_idx += 1;
            let _ = write!(
                query,
                " AND (codigo_proveedor ILIKE ${param_idx} OR nombre_proveedor ILIKE ${param_idx})"
            );
            let _ = args.add(format!("%{proveedor}%"));
        }
        if let Some(ref desde) = params.fecha_desde {
            param_idx += 1;
            let _ = write!(query, " AND fecha >= ${param_idx}");
            let _ = args.add(desde.clone());
        }
        if let Some(ref hasta) = params.fecha_hasta {
            param_idx += 1;
            let _ = write!(query, " AND fecha <= ${param_idx}");
            let _ = args.add(hasta.clone());
        }

        query.push_str(" ORDER BY fecha DESC NULLS LAST, serie, numero");

        sqlx::query_as_with(&query, args)
            .fetch_all(pool)
            .await
    }

    /// Inserta o actualiza un albarán a partir de los datos devueltos por BDP.
    /// La clave natural es (`user_id`, `serie`, `numero`).
    pub async fn upsert_from_bdp(
        pool: &PgPool,
        user_id: Uuid,
        note: &crate::services::bdp_weblink_catalog::BdpPurchaseNoteData,
    ) -> Result<bool, sqlx::Error> {
        let serie = note.serie_albaran.as_deref().unwrap_or("");
        let numero = note.num_albaran.as_deref().unwrap_or("");
        let codigo_proveedor = note.cod_proveedor.as_ref().map(std::string::ToString::to_string);
        let nombre_proveedor = note.nom_proveedor.as_deref().unwrap_or("");
        let total = note.total_albaran;
        let fecha = note.fecha_albaran.as_deref().and_then(parse_fecha_bdp);

        let result = sqlx::query(
            "INSERT INTO bdp_purchase_notes \
                (id, user_id, serie, numero, fecha, codigo_proveedor, nombre_proveedor, total, datos_bdp, ultima_sync_at, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW(), NOW()) \
             ON CONFLICT (user_id, serie, numero) DO UPDATE SET \
                fecha = EXCLUDED.fecha, \
                codigo_proveedor = EXCLUDED.codigo_proveedor, \
                nombre_proveedor = EXCLUDED.nombre_proveedor, \
                total = EXCLUDED.total, \
                datos_bdp = EXCLUDED.datos_bdp, \
                ultima_sync_at = NOW(), \
                updated_at = NOW()",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(serie)
        .bind(numero)
        .bind(fecha)
        .bind(codigo_proveedor)
        .bind(nombre_proveedor)
        .bind(total)
        .bind(serde_json::to_value(note).unwrap_or(serde_json::json!({})))
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}

/// Parsea fechas que BDP devuelve como "2021-07-27T00:00:00" u otro formato.
/// Normaliza espacios antes de intentar el parseo.
fn parse_fecha_bdp(value: &str) -> Option<chrono::NaiveDate> {
    let cleaned = value
        .split_once('T')
        .map_or(value, |(date, _)| date)
        .replace(' ', "");
    chrono::NaiveDate::parse_from_str(&cleaned, "%Y-%m-%d").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fecha_bdp_handles_iso_date() {
        assert_eq!(
            parse_fecha_bdp("2021-07-27T00:00:00"),
            chrono::NaiveDate::from_ymd_opt(2021, 7, 27)
        );
    }

    #[test]
    fn parse_fecha_bdp_handles_date_with_spaces() {
        assert_eq!(
            parse_fecha_bdp("2021- 07 - 27T00:00:00"),
            chrono::NaiveDate::from_ymd_opt(2021, 7, 27)
        );
    }

    #[test]
    fn parse_fecha_bdp_returns_none_for_invalid() {
        assert!(parse_fecha_bdp("not-a-date").is_none());
    }
}
