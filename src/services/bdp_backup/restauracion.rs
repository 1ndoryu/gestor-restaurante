use sqlx::PgPool;
use uuid::Uuid;

use super::{BdpBackupService, RestoreResult};

// =========================================================================
// RESTAURACIÓN (solo datos locales de Glory)
// =========================================================================

impl BdpBackupService {
    /// Restaura datos de Glory desde un snapshot.
    /// NOTA: BDP no permite delete/update via API — solo podemos restaurar datos locales.
    /* [187A-1] Restauración legacy limitada a datos locales; mantener ambos
     * recorridos juntos facilita contar y reportar fallos parciales sin ocultarlos. */
    pub async fn restaurar_glory(
        pool: &PgPool,
        snapshot_id: Uuid,
        user_id: Uuid,
    ) -> Result<RestoreResult, String> {
        let snapshot = Self::obtener_snapshot(pool, snapshot_id)
            .await?
            .ok_or_else(|| "Snapshot no encontrado".to_string())?;

        if snapshot.user_id != user_id {
            return Err("No autorizado: el snapshot pertenece a otro usuario".to_string());
        }

        if snapshot.direccion != "glory" {
            return Err(format!(
                "Solo se pueden restaurar snapshots de Glory (este es de {})",
                snapshot.direccion
            ));
        }

        let mut registros_restaurados: u32 = 0;
        let mut errores: u32 = 0;
        let mut detalles = Vec::new();

        /* [207A-3] S14-H1: Envolver toda la restauración en una transacción
         * atómica. Si falla cualquier UPDATE intermedio, se revierte todo
         * para evitar estado parcial (mapeos restaurados pero clientes no). */
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| format!("Error iniciando tx de restauración: {e}"))?;

        /* Restaurar mapeos de artículos */
        {
            let (n, e, d) = Self::restaurar_mapeos(&mut tx, &snapshot.datos, user_id).await?;
            registros_restaurados += n;
            errores += e;
            detalles.extend(d);
        }

        /* Restaurar campos BDP de clientes */
        {
            let (n, e, d) = Self::restaurar_clientes(&mut tx, &snapshot.datos, user_id).await?;
            registros_restaurados += n;
            errores += e;
            detalles.extend(d);
        }

        tx.commit()
            .await
            .map_err(|e| format!("Error confirmando restauración: {e}"))?;

        let detalles_str = if detalles.is_empty() {
            format!("Restaurados {registros_restaurados} registros sin errores")
        } else {
            format!(
                "Restaurados {registros_restaurados} registros, {errores} errores. {}",
                detalles.join("; ")
            )
        };

        Ok(RestoreResult {
            snapshot_id,
            tipo: snapshot.tipo,
            registros_restaurados,
            errores,
            detalles: detalles_str,
        })
    }

    /* Restaura los mapeos de artículos del snapshot dentro de la tx abierta.
     * Devuelve (registros, errores, detalles) para que restaurar_glory los
     * sume y no exceda el límite de líneas efectivas por función. */
    async fn restaurar_mapeos(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        datos: &serde_json::Value,
        user_id: Uuid,
    ) -> Result<(u32, u32, Vec<String>), String> {
        let mut registros = 0;
        let mut errores = 0;
        let mut detalles = Vec::new();

        if let Some(mapeos) = datos.get("mapeos").and_then(serde_json::Value::as_array) {
            for mapeo in mapeos {
                let Some(id) = mapeo.get("id").and_then(serde_json::Value::as_str) else {
                    continue;
                };
                let Ok(id) = Uuid::parse_str(id) else {
                    continue;
                };
                let descripcion = mapeo.get("descripcion").and_then(serde_json::Value::as_str);
                let precio = mapeo
                    .get("precio_tarifa1")
                    .and_then(serde_json::Value::as_f64);
                let iva = mapeo.get("iva_pct").and_then(serde_json::Value::as_f64);
                let activo = mapeo.get("activo").and_then(serde_json::Value::as_bool);

                let r = sqlx::query(
                    r"UPDATE bdp_article_map
                    SET descripcion = COALESCE($3, descripcion),
                        precio_tarifa1 = COALESCE($4, precio_tarifa1),
                        iva_pct = COALESCE($5, iva_pct),
                        activo = COALESCE($6, activo)
                    WHERE id = $1 AND user_id = $2",
                )
                .bind(id)
                .bind(user_id)
                .bind(descripcion)
                .bind(precio.and_then(|p| rust_decimal::Decimal::try_from(p).ok()))
                .bind(iva.and_then(|i| rust_decimal::Decimal::try_from(i).ok()))
                .bind(activo)
                .execute(&mut **tx)
                .await
                .map_err(|e| format!("Error restaurando mapeo {id}: {e}"))?;

                if r.rows_affected() > 0 {
                    registros += 1;
                } else {
                    detalles.push(format!("Mapeo {id} no encontrado"));
                    errores += 1;
                }
            }
        }

        Ok((registros, errores, detalles))
    }

    /* Restaura los campos BDP de clientes dentro de la tx abierta. */
    async fn restaurar_clientes(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        datos: &serde_json::Value,
        user_id: Uuid,
    ) -> Result<(u32, u32, Vec<String>), String> {
        let mut registros = 0;
        let mut errores = 0;
        let mut detalles = Vec::new();

        if let Some(clientes) = datos.get("clientes").and_then(serde_json::Value::as_array) {
            for cliente in clientes {
                let Some(id) = cliente.get("id").and_then(serde_json::Value::as_str) else {
                    continue;
                };
                let Ok(id) = Uuid::parse_str(id) else {
                    continue;
                };
                let bdp_code = cliente
                    .get("bdp_customer_code")
                    .and_then(serde_json::Value::as_i64)
                    .and_then(|code| i32::try_from(code).ok());

                let r = sqlx::query(
                    r"UPDATE clientes
                    SET bdp_customer_code = COALESCE($3, bdp_customer_code)
                    WHERE id = $1 AND user_id = $2",
                )
                .bind(id)
                .bind(user_id)
                .bind(bdp_code)
                .execute(&mut **tx)
                .await
                .map_err(|e| format!("Error restaurando cliente {id}: {e}"))?;

                if r.rows_affected() > 0 {
                    registros += 1;
                } else {
                    detalles.push(format!("Cliente {id} no encontrado"));
                    errores += 1;
                }
            }
        }

        Ok((registros, errores, detalles))
    }
}
