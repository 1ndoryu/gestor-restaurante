use sqlx::PgPool;
use uuid::Uuid;

use super::tipos::NuevoSnapshot;
use super::{BdpBackupService, BdpSnapshot};
use crate::models::ConfiguracionRestaurante;
use crate::services::bdp_weblink::BdpWeblinkClient;

/* Snapshots de lectura contra BDP. Cada tipo llama a su endpoint de lectura
 * y el resultado se persiste con la huella de la conexión usada. */
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
        let articulos = Self::fetch_articles(&client, config).await?;
        let clientes = Self::fetch_customers(&client).await?;
        let departamentos = Self::fetch_departments(&client).await?;
        let salones = Self::fetch_rooms(&client).await?;
        let empleados = Self::fetch_employees(&client).await?;

        let datos = serde_json::json!({
            "articulos": articulos,
            "clientes": clientes,
            "departamentos": departamentos,
            "salones": salones,
            "empleados": empleados,
        });

        /* [048A-7] Traza en metadata si salones vino ausente (subscripción BDP no
         * activada): se guarda [] y queda documentado para el arming/auditoría. */
        let salones_ausentes = datos["salones"].as_array().is_some_and(Vec::is_empty);
        let metadata = serde_json::json!({
            "endpoints": ["ExportArticles", "ExportCustomers", "ExportDepartments", "GetRoomsTables", "GetEmployees"],
            "salones_ausentes": salones_ausentes,
            "motivo_salones": if salones_ausentes {
                "Subscripción BDP no activada (GetRoomsTables); salones guardados como []"
            } else {
                ""
            },
        });

        /* Calcular expiración */
        let retention_days = Self::get_retention_days(pool, user_id).await?;
        let expires_at = if retention_days > 0 {
            Some(chrono::Utc::now() + chrono::Duration::days(i64::from(retention_days)))
        } else {
            None
        };

        Self::insert_snapshot(
            pool,
            user_id,
            NuevoSnapshot {
                tipo: "completo",
                direccion: "bdp",
                trigger_tipo: "manual",
                datos,
                metadata: Some(metadata),
                target_base_url: Some(Self::canonical_target(config)?),
                connection_fingerprint: Some(Self::connection_fingerprint(config)?),
                expires_at,
                notas,
            },
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
        const VALID_TYPES: &[&str] = &[
            "articulos",
            "clientes",
            "departamentos",
            "salones",
            "empleados",
        ];
        if tipos.is_empty()
            || tipos
                .iter()
                .any(|tipo| !VALID_TYPES.contains(&tipo.as_str()))
            || tipos.iter().collect::<std::collections::HashSet<_>>().len() != tipos.len()
        {
            return Err(
                "Snapshot parcial bloqueado: tipos vacíos, repetidos o desconocidos".to_string(),
            );
        }
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
                    let val = Self::fetch_articles(&client, config).await?;
                    datos["articulos"] = val;
                    endpoints_used.push("ExportArticles");
                }
                "clientes" => {
                    let val = Self::fetch_customers(&client).await?;
                    datos["clientes"] = val;
                    endpoints_used.push("ExportCustomers");
                }
                "departamentos" => {
                    let val = Self::fetch_departments(&client).await?;
                    datos["departamentos"] = val;
                    endpoints_used.push("ExportDepartments");
                }
                "salones" => {
                    let val = Self::fetch_rooms(&client).await?;
                    datos["salones"] = val;
                    endpoints_used.push("GetRoomsTables");
                }
                "empleados" => {
                    let val = Self::fetch_employees(&client).await?;
                    datos["empleados"] = val;
                    endpoints_used.push("GetEmployees");
                }
                _ => unreachable!("los tipos se validaron antes de contactar BDP"),
            }
        }

        let metadata = serde_json::json!({
            "endpoints": endpoints_used,
            "tipos_solicitados": tipos,
        });

        let retention_days = Self::get_retention_days(pool, user_id).await?;
        let expires_at = if retention_days > 0 {
            Some(chrono::Utc::now() + chrono::Duration::days(i64::from(retention_days)))
        } else {
            None
        };

        // El tipo depende de los tipos pedidos: se construye antes para prestarlo al snapshot.
        let tipo = format!("parcial_{}", tipos.join("_"));
        Self::insert_snapshot(
            pool,
            user_id,
            NuevoSnapshot {
                tipo: &tipo,
                direccion: "bdp",
                trigger_tipo: "manual",
                datos,
                metadata: Some(metadata),
                target_base_url: Some(Self::canonical_target(config)?),
                connection_fingerprint: Some(Self::connection_fingerprint(config)?),
                expires_at,
                notas,
            },
        )
        .await
    }
}
