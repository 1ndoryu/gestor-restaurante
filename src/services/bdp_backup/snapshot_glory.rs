use sqlx::PgPool;
use uuid::Uuid;

use super::tipos::NuevoSnapshot;
use super::{BdpBackupService, BdpSnapshot};

impl BdpBackupService {
    // =========================================================================
    // SNAPSHOT GLORY — exporta tablas locales de Glory
    // =========================================================================

    /// Snapshot de datos locales de Glory (query local, 0 llamadas BDP).
    /// Tipos válidos: 'ventas', 'clientes', 'mapeos'
    pub async fn snapshot_glory(
        pool: &PgPool,
        user_id: Uuid,
        tipos: &[String],
        notas: Option<String>,
    ) -> Result<BdpSnapshot, String> {
        const VALID_TYPES: &[&str] = &["ventas", "clientes", "mapeos"];
        if tipos.is_empty()
            || tipos
                .iter()
                .any(|tipo| !VALID_TYPES.contains(&tipo.as_str()))
            || tipos.iter().collect::<std::collections::HashSet<_>>().len() != tipos.len()
        {
            return Err(
                "Snapshot Glory bloqueado: tipos vacíos, repetidos o desconocidos".to_string(),
            );
        }
        let datos = Self::exportar_datos_glory(pool, user_id, tipos).await?;

        let metadata = serde_json::json!({
            "tipos_solicitados": tipos,
            "source": "glory_local_db",
        });

        let retention_days = Self::get_retention_days(pool, user_id).await?;
        let expires_at = if retention_days > 0 {
            Some(chrono::Utc::now() + chrono::Duration::days(i64::from(retention_days)))
        } else {
            None
        };

        // El tipo depende de los tipos pedidos: se construye antes para prestarlo al snapshot.
        let tipo = format!("glory_{}", tipos.join("_"));
        Self::insert_snapshot(
            pool,
            user_id,
            NuevoSnapshot {
                tipo: &tipo,
                direccion: "glory",
                trigger_tipo: "manual",
                datos,
                metadata: Some(metadata),
                target_base_url: None,
                connection_fingerprint: None,
                expires_at,
                notas,
            },
        )
        .await
    }

    /* Exporta cada tipo pedido desde la base local, con su límite de filas.
     * Va aparte para que snapshot_glory solo orqueste validación, metadata y
     * persistencia. */
    async fn exportar_datos_glory(
        pool: &PgPool,
        user_id: Uuid,
        tipos: &[String],
    ) -> Result<serde_json::Value, String> {
        let mut datos = serde_json::json!({});

        for tipo in tipos {
            match tipo.as_str() {
                "ventas" => {
                    let val: serde_json::Value = sqlx::query_scalar(
                        r"SELECT COALESCE(json_agg(row_to_json(v)), '[]'::json)
                        FROM (
                            SELECT id, user_id, cliente_id, canal,
                                   importe_base, importe_iva,
                                   (importe_base + importe_iva) AS total,
                                   propina, fecha, turno, metodo_pago,
                                   anulada, facturada_local, factura_numero,
                                   bdp_synced, bdp_order_id, bdp_order_status,
                                   bdp_sync_error, bdp_invoiced
                            FROM ventas
                            WHERE user_id = $1
                            ORDER BY created_at DESC
                            LIMIT 5000
                        ) v",
                    )
                    .bind(user_id)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error exportando ventas Glory: {e}"))?;
                    datos["ventas"] = val;
                }
                "clientes" => {
                    let val: serde_json::Value = sqlx::query_scalar(
                        r"SELECT COALESCE(json_agg(row_to_json(c)), '[]'::json)
                        FROM (
                            SELECT id, user_id, nombre, email, telefono,
                                   bdp_customer_code, bdp_synced, bdp_synced_at, bdp_sync_error
                            FROM clientes
                            WHERE user_id = $1
                            ORDER BY created_at DESC
                            LIMIT 5000
                        ) c",
                    )
                    .bind(user_id)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error exportando clientes Glory: {e}"))?;
                    datos["clientes"] = val;
                }
                "mapeos" => {
                    let val: serde_json::Value = sqlx::query_scalar(
                        r"SELECT COALESCE(json_agg(row_to_json(m)), '[]'::json)
                        FROM (
                            SELECT id, user_id, articulo_glory_codigo, articulo_bdp_codigo,
                                   articulo_bdp_nombre, descripcion, precio_tarifa1, iva_pct,
                                   departamento, familia, subfamilia, activo, barcode
                            FROM bdp_article_map
                            WHERE user_id = $1
                            ORDER BY articulo_bdp_nombre
                            LIMIT 10000
                        ) m",
                    )
                    .bind(user_id)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error exportando mapeos Glory: {e}"))?;
                    datos["mapeos"] = val;
                }
                _ => unreachable!("los tipos Glory se validaron antes de consultar la base"),
            }
        }

        Ok(datos)
    }
}
