/* [BKP-003] Motor de snapshots y auditoría BDP.
 * Gestiona snapshots (puntos de restauración) y audit log (traza inmutable).
 * Pre-write snapshots: selectivos, máximo 1 llamada adicional a BDP.
 * Restauración: solo datos locales de Glory (BDP no permite delete/update via API).
 * Todas las queries usan runtime sqlx (no macros compile-time) para compatibilidad SQLX_OFFLINE. */

use chrono::NaiveDateTime;
use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

use crate::models::ConfiguracionRestaurante;
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{
    BdpExportArticlesRequest, BdpExportCustomersRequest, BdpExportDepartmentsRequest,
    BdpGetEmployeesRequest, BdpGetOrderRequest, BdpGetRoomsTablesRequest, BdpOrderIdentifier,
};

/// Snapshot almacenado en la base de datos.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema, sqlx::FromRow)]
pub struct BdpSnapshot {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tipo: String,
    pub direccion: String,
    pub trigger_tipo: String,
    pub datos: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
    pub created_at: NaiveDateTime,
    pub expires_at: Option<NaiveDateTime>,
    pub notas: Option<String>,
}

/// Entrada del audit log.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema, sqlx::FromRow)]
pub struct BdpAuditEntry {
    pub id: Uuid,
    pub user_id: Uuid,
    pub operacion: String,
    pub direccion: String,
    pub snapshot_pre_id: Option<Uuid>,
    pub datos_enviados: Option<serde_json::Value>,
    pub resultado: String,
    pub datos_respuesta: Option<serde_json::Value>,
    pub error_mensaje: Option<String>,
    pub created_at: NaiveDateTime,
}

/// Resultado de una restauración.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct RestoreResult {
    pub snapshot_id: Uuid,
    pub tipo: String,
    pub registros_restaurados: u32,
    pub errores: u32,
    pub detalles: String,
}

pub struct BdpBackupService;

impl BdpBackupService {
    // =========================================================================
    // SNAPSHOT BDP COMPLETO — lee TODOS los endpoints de lectura
    // =========================================================================

    /// Snapshot completo de BDP. Lee todos los endpoints de lectura.
    /// Costo: 5 llamadas a BDP (artículos, clientes, departamentos, salones, empleados).
    pub async fn snapshot_bdp_completo(
        pool: &PgPool,
        user_id: Uuid,
        config: &ConfiguracionRestaurante,
        notas: Option<String>,
    ) -> Result<BdpSnapshot, String> {
        let client = BdpWeblinkClient::new(config);

        /* Login primero para validar credenciales */
        let _session = client
            .login()
            .await
            .map_err(|e| format!("Error login BDP: {e}"))?;

        /* Recolectar datos de cada endpoint */
        let articulos = Self::fetch_articles(&client, config).await;
        let clientes = Self::fetch_customers(&client).await;
        let departamentos = Self::fetch_departments(&client).await;
        let salones = Self::fetch_rooms(&client).await;
        let empleados = Self::fetch_employees(&client).await;

        let datos = serde_json::json!({
            "articulos": articulos,
            "clientes": clientes,
            "departamentos": departamentos,
            "salones": salones,
            "empleados": empleados,
        });

        let metadata = serde_json::json!({
            "endpoints": ["ExportArticles", "ExportCustomers", "ExportDepartments", "GetRoomsTables", "GetEmployees"],
        });

        /* Calcular expiración */
        let retention_days = Self::get_retention_days(pool, user_id).await;
        let expires_at = if retention_days > 0 {
            Some((chrono::Utc::now() + chrono::Duration::days(i64::from(retention_days))).naive_utc())
        } else {
            None
        };

        Self::insert_snapshot(
            pool,
            user_id,
            "completo",
            "bdp",
            "manual",
            datos,
            Some(metadata),
            expires_at,
            notas,
        )
        .await
    }

    // =========================================================================
    // SNAPSHOT PARCIAL BDP — solo los tipos seleccionados
    // =========================================================================

    /// Snapshot parcial de BDP. Solo los tipos de datos seleccionados.
    /// Tipos válidos: 'articulos', 'clientes', 'departamentos', 'salones', 'empleados'
    pub async fn snapshot_bdp_parcial(
        pool: &PgPool,
        user_id: Uuid,
        config: &ConfiguracionRestaurante,
        tipos: &[String],
        notas: Option<String>,
    ) -> Result<BdpSnapshot, String> {
        let client = BdpWeblinkClient::new(config);
        let _session = client
            .login()
            .await
            .map_err(|e| format!("Error login BDP: {e}"))?;

        let mut datos = serde_json::json!({});
        let mut endpoints_used = Vec::new();

        for tipo in tipos {
            match tipo.as_str() {
                "articulos" => {
                    let val = Self::fetch_articles(&client, config).await;
                    datos["articulos"] = val;
                    endpoints_used.push("ExportArticles");
                }
                "clientes" => {
                    let val = Self::fetch_customers(&client).await;
                    datos["clientes"] = val;
                    endpoints_used.push("ExportCustomers");
                }
                "departamentos" => {
                    let val = Self::fetch_departments(&client).await;
                    datos["departamentos"] = val;
                    endpoints_used.push("ExportDepartments");
                }
                "salones" => {
                    let val = Self::fetch_rooms(&client).await;
                    datos["salones"] = val;
                    endpoints_used.push("GetRoomsTables");
                }
                "empleados" => {
                    let val = Self::fetch_employees(&client).await;
                    datos["empleados"] = val;
                    endpoints_used.push("GetEmployees");
                }
                other => {
                    warn!("Tipo de snapshot desconocido: {other}");
                }
            }
        }

        let metadata = serde_json::json!({
            "endpoints": endpoints_used,
            "tipos_solicitados": tipos,
        });

        let retention_days = Self::get_retention_days(pool, user_id).await;
        let expires_at = if retention_days > 0 {
            Some((chrono::Utc::now() + chrono::Duration::days(i64::from(retention_days))).naive_utc())
        } else {
            None
        };

        Self::insert_snapshot(
            pool,
            user_id,
            &format!("parcial_{}", tipos.join("_")),
            "bdp",
            "manual",
            datos,
            Some(metadata),
            expires_at,
            notas,
        )
        .await
    }

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
        let mut datos = serde_json::json!({});

        for tipo in tipos {
            match tipo.as_str() {
                "ventas" => {
                    let val: serde_json::Value = sqlx::query_scalar(
                        r#"SELECT COALESCE(json_agg(row_to_json(v)), '[]'::json)
                        FROM (
                            SELECT id, user_id, cliente_id, canal, total, estado,
                                   bdp_synced, bdp_order_id, bdp_order_status,
                                   bdp_sync_error, bdp_invoiced
                            FROM ventas
                            WHERE user_id = $1
                            ORDER BY created_at DESC
                            LIMIT 5000
                        ) v"#,
                    )
                    .bind(user_id)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error exportando ventas Glory: {e}"))?;
                    datos["ventas"] = val;
                }
                "clientes" => {
                    let val: serde_json::Value = sqlx::query_scalar(
                        r#"SELECT COALESCE(json_agg(row_to_json(c)), '[]'::json)
                        FROM (
                            SELECT id, user_id, nombre, email, telefono,
                                   bdp_customer_code, bdp_synced, bdp_synced_at, bdp_sync_error
                            FROM clientes
                            WHERE user_id = $1
                            ORDER BY created_at DESC
                            LIMIT 5000
                        ) c"#,
                    )
                    .bind(user_id)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error exportando clientes Glory: {e}"))?;
                    datos["clientes"] = val;
                }
                "mapeos" => {
                    let val: serde_json::Value = sqlx::query_scalar(
                        r#"SELECT COALESCE(json_agg(row_to_json(m)), '[]'::json)
                        FROM (
                            SELECT id, user_id, articulo_glory_codigo, articulo_bdp_codigo,
                                   articulo_bdp_nombre, descripcion, precio_tarifa1, iva_pct,
                                   departamento, familia, subfamilia, activo, barcode
                            FROM bdp_article_map
                            WHERE user_id = $1
                            ORDER BY articulo_bdp_nombre
                            LIMIT 10000
                        ) m"#,
                    )
                    .bind(user_id)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error exportando mapeos Glory: {e}"))?;
                    datos["mapeos"] = val;
                }
                other => {
                    warn!("Tipo de snapshot Glory desconocido: {other}");
                }
            }
        }

        let metadata = serde_json::json!({
            "tipos_solicitados": tipos,
            "source": "glory_local_db",
        });

        let retention_days = Self::get_retention_days(pool, user_id).await;
        let expires_at = if retention_days > 0 {
            Some((chrono::Utc::now() + chrono::Duration::days(i64::from(retention_days))).naive_utc())
        } else {
            None
        };

        Self::insert_snapshot(
            pool,
            user_id,
            &format!("glory_{}", tipos.join("_")),
            "glory",
            "manual",
            datos,
            Some(metadata),
            expires_at,
            notas,
        )
        .await
    }

    // =========================================================================
    // PRE-WRITE AUDIT LOG — selectivo, máximo 1 llamada BDP
    // =========================================================================

    /// Registra una operación de escritura en el audit log.
    /// Para AddOrderPayment/InvoiceOrder, hace 1 GetOrder para snapshot del estado actual.
    /// Para CreateOrder/CreateCustomer, solo registra los datos enviados (0 llamadas extra).
    pub async fn registrar_escritura(
        pool: &PgPool,
        user_id: Uuid,
        operacion: &str,
        datos_enviados: &serde_json::Value,
        config: &ConfiguracionRestaurante,
        bdp_order_id: Option<i64>,
    ) -> Result<Option<Uuid>, String> {
        let auto_backup = Self::get_auto_backup(pool, user_id).await;
        if !auto_backup {
            return Ok(None);
        }

        /* Para AddOrderPayment e InvoiceOrder: snapshot del estado actual de la comanda */
        let snapshot_id = if operacion == "add_payment" || operacion == "invoice" {
            if let Some(order_id) = bdp_order_id {
                match Self::snapshot_order_state(config, order_id).await {
                    Ok(snapshot) => {
                        let snap = Self::insert_snapshot(
                            pool,
                            user_id,
                            "pre_write_order",
                            "bdp",
                            "pre_write",
                            snapshot,
                            Some(serde_json::json!({"operacion": operacion, "order_id": order_id})),
                            None,
                            None,
                        )
                        .await?;
                        Some(snap.id)
                    }
                    Err(e) => {
                        warn!("Pre-write snapshot falló para order {order_id}: {e}. Continuando sin snapshot.");
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        /* Insertar entrada en audit log — runtime query para compatibilidad SQLX_OFFLINE */
        let entry_id: Uuid = sqlx::query_scalar(
            r#"INSERT INTO bdp_audit_log (user_id, operacion, direccion, snapshot_pre_id, datos_enviados, resultado)
            VALUES ($1, $2, 'glory_to_bdp', $3, $4, 'pendiente')
            RETURNING id"#,
        )
        .bind(user_id)
        .bind(operacion)
        .bind(snapshot_id)
        .bind(datos_enviados)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Error insertando audit log: {e}"))?;

        Ok(Some(entry_id))
    }

    /// Actualiza el resultado de una entrada de auditoría.
    pub async fn actualizar_resultado(
        pool: &PgPool,
        audit_id: Uuid,
        resultado: &str,
        datos_respuesta: Option<&serde_json::Value>,
        error_mensaje: Option<&str>,
    ) -> Result<(), String> {
        sqlx::query(
            r#"UPDATE bdp_audit_log
            SET resultado = $2, datos_respuesta = $3, error_mensaje = $4
            WHERE id = $1"#,
        )
        .bind(audit_id)
        .bind(resultado)
        .bind(datos_respuesta)
        .bind(error_mensaje)
        .execute(pool)
        .await
        .map_err(|e| format!("Error actualizando audit log: {e}"))?;

        Ok(())
    }

    // =========================================================================
    // CONSULTAS
    // =========================================================================

    /// Lista snapshots del usuario, ordenados por fecha descendente.
    pub async fn listar_snapshots(
        pool: &PgPool,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<BdpSnapshot>, String> {
        sqlx::query_as::<_, BdpSnapshot>(
            r#"SELECT id, user_id, tipo, direccion, trigger_tipo, datos, metadata,
                      created_at, expires_at, notas
            FROM bdp_snapshots
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2"#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error listando snapshots: {e}"))
    }

    /// Obtiene un snapshot por ID.
    pub async fn obtener_snapshot(
        pool: &PgPool,
        snapshot_id: Uuid,
    ) -> Result<Option<BdpSnapshot>, String> {
        sqlx::query_as::<_, BdpSnapshot>(
            r#"SELECT id, user_id, tipo, direccion, trigger_tipo, datos, metadata,
                      created_at, expires_at, notas
            FROM bdp_snapshots
            WHERE id = $1"#,
        )
        .bind(snapshot_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Error obteniendo snapshot: {e}"))
    }

    /// Elimina un snapshot.
    pub async fn eliminar_snapshot(
        pool: &PgPool,
        snapshot_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, String> {
        let result = sqlx::query(
            r#"DELETE FROM bdp_snapshots WHERE id = $1 AND user_id = $2"#,
        )
        .bind(snapshot_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| format!("Error eliminando snapshot: {e}"))?;

        Ok(result.rows_affected() > 0)
    }

    /// Lista entradas del audit log.
    pub async fn listar_audit(
        pool: &PgPool,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<BdpAuditEntry>, String> {
        sqlx::query_as::<_, BdpAuditEntry>(
            r#"SELECT id, user_id, operacion, direccion, snapshot_pre_id,
                      datos_enviados, resultado, datos_respuesta, error_mensaje, created_at
            FROM bdp_audit_log
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2"#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error listando audit log: {e}"))
    }

    // =========================================================================
    // RESTAURACIÓN (solo datos locales de Glory)
    // =========================================================================

    /// Restaura datos de Glory desde un snapshot.
    /// NOTA: BDP no permite delete/update via API — solo podemos restaurar datos locales.
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

        /* Restaurar mapeos de artículos */
        if let Some(mapeos) = snapshot.datos.get("mapeos").and_then(serde_json::Value::as_array) {
            for mapeo in mapeos {
                let Some(id) = mapeo.get("id").and_then(serde_json::Value::as_str) else {
                    continue;
                };
                let Ok(id) = Uuid::parse_str(id) else {
                    continue;
                };
                let descripcion = mapeo.get("descripcion").and_then(serde_json::Value::as_str);
                let precio = mapeo.get("precio_tarifa1").and_then(serde_json::Value::as_f64);
                let iva = mapeo.get("iva_pct").and_then(serde_json::Value::as_f64);
                let activo = mapeo.get("activo").and_then(serde_json::Value::as_bool);

                let result = sqlx::query(
                    r#"UPDATE bdp_article_map
                    SET descripcion = COALESCE($3, descripcion),
                        precio_tarifa1 = COALESCE($4, precio_tarifa1),
                        iva_pct = COALESCE($5, iva_pct),
                        activo = COALESCE($6, activo)
                    WHERE id = $1 AND user_id = $2"#,
                )
                .bind(id)
                .bind(user_id)
                .bind(descripcion)
                .bind(precio.and_then(|p| rust_decimal::Decimal::try_from(p).ok()))
                .bind(iva.and_then(|i| rust_decimal::Decimal::try_from(i).ok()))
                .bind(activo)
                .execute(pool)
                .await;

                match result {
                    Ok(r) if r.rows_affected() > 0 => registros_restaurados += 1,
                    Ok(_) => {
                        detalles.push(format!("Mapeo {id} no encontrado"));
                        errores += 1;
                    }
                    Err(e) => {
                        detalles.push(format!("Mapeo {id}: {e}"));
                        errores += 1;
                    }
                }
            }
        }

        /* Restaurar campos BDP de clientes */
        if let Some(clientes) = snapshot.datos.get("clientes").and_then(serde_json::Value::as_array) {
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
                    .map(|c| c as i32);

                let result = sqlx::query(
                    r#"UPDATE clientes
                    SET bdp_customer_code = COALESCE($3, bdp_customer_code)
                    WHERE id = $1 AND user_id = $2"#,
                )
                .bind(id)
                .bind(user_id)
                .bind(bdp_code)
                .execute(pool)
                .await;

                match result {
                    Ok(r) if r.rows_affected() > 0 => registros_restaurados += 1,
                    Ok(_) => errores += 1,
                    Err(e) => {
                        detalles.push(format!("Cliente {id}: {e}"));
                        errores += 1;
                    }
                }
            }
        }

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

    // =========================================================================
    // LIMPIEZA DE SNAPSHOTS EXPIRADOS
    // =========================================================================

    /// Elimina snapshots expirados.
    pub async fn limpiar_expirados(pool: &PgPool) -> Result<u64, String> {
        let result = sqlx::query(
            r#"DELETE FROM bdp_snapshots WHERE expires_at IS NOT NULL AND expires_at < NOW()"#,
        )
        .execute(pool)
        .await
        .map_err(|e| format!("Error limpiando snapshots expirados: {e}"))?;

        Ok(result.rows_affected())
    }

    // =========================================================================
    // HELPERS PRIVADOS
    // =========================================================================

    async fn insert_snapshot(
        pool: &PgPool,
        user_id: Uuid,
        tipo: &str,
        direccion: &str,
        trigger_tipo: &str,
        datos: serde_json::Value,
        metadata: Option<serde_json::Value>,
        expires_at: Option<NaiveDateTime>,
        notas: Option<String>,
    ) -> Result<BdpSnapshot, String> {
        sqlx::query_as::<_, BdpSnapshot>(
            r#"INSERT INTO bdp_snapshots (user_id, tipo, direccion, trigger_tipo, datos, metadata, expires_at, notas)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, user_id, tipo, direccion, trigger_tipo, datos, metadata,
                      created_at, expires_at, notas"#,
        )
        .bind(user_id)
        .bind(tipo)
        .bind(direccion)
        .bind(trigger_tipo)
        .bind(datos)
        .bind(metadata)
        .bind(expires_at)
        .bind(notas)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Error insertando snapshot: {e}"))
    }

    async fn snapshot_order_state(
        config: &ConfiguracionRestaurante,
        order_id: i64,
    ) -> Result<serde_json::Value, String> {
        let client = BdpWeblinkClient::new(config);
        let order_data = client
            .get_order(&BdpGetOrderRequest {
                order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            })
            .await
            .map_err(|e| format!("Error obteniendo estado de comanda {order_id}: {e}"))?;
        Ok(order_data)
    }

    async fn get_retention_days(pool: &PgPool, user_id: Uuid) -> i32 {
        sqlx::query_scalar::<_, i32>(
            r#"SELECT COALESCE(bdp_backup_retention_days, 30)
            FROM configuracion_restaurante WHERE user_id = $1"#,
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .unwrap_or(30)
    }

    async fn get_auto_backup(pool: &PgPool, user_id: Uuid) -> bool {
        sqlx::query_scalar::<_, bool>(
            r#"SELECT COALESCE(bdp_auto_backup_before_write, true)
            FROM configuracion_restaurante WHERE user_id = $1"#,
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .unwrap_or(true)
    }

    async fn fetch_articles(
        client: &BdpWeblinkClient<'_>,
        config: &ConfiguracionRestaurante,
    ) -> serde_json::Value {
        client
            .export_articles(&BdpExportArticlesRequest::all_web_articles(config.bdp_pos_id))
            .await
            .unwrap_or_else(|e| {
                warn!("Snapshot articles error: {e}");
                serde_json::json!(null)
            })
    }

    async fn fetch_customers(client: &BdpWeblinkClient<'_>) -> serde_json::Value {
        client
            .export_customers(&BdpExportCustomersRequest::default())
            .await
            .unwrap_or_else(|e| {
                warn!("Snapshot customers error: {e}");
                serde_json::json!(null)
            })
    }

    async fn fetch_departments(client: &BdpWeblinkClient<'_>) -> serde_json::Value {
        client
            .export_departments(&BdpExportDepartmentsRequest::default())
            .await
            .unwrap_or_else(|e| {
                warn!("Snapshot departments error: {e}");
                serde_json::json!(null)
            })
    }

    async fn fetch_rooms(client: &BdpWeblinkClient<'_>) -> serde_json::Value {
        client
            .get_rooms_tables(&BdpGetRoomsTablesRequest::default())
            .await
            .unwrap_or_else(|e| {
                warn!("Snapshot rooms error: {e}");
                serde_json::json!(null)
            })
    }

    async fn fetch_employees(client: &BdpWeblinkClient<'_>) -> serde_json::Value {
        client
            .get_employees(&BdpGetEmployeesRequest {
                ids: vec![],
                only_salespeople: None,
            })
            .await
            .unwrap_or_else(|e| {
                warn!("Snapshot employees error: {e}");
                serde_json::json!(null)
            })
    }
}
