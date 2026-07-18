/* [BKP-002] Servicio de exploración segura de BDP.
 * Lee endpoints de solo lectura para inventariar qué hay en BDP.
 * NO modifica NADA en BDP. Solo lectura.
 * Endpoint: GET /api/bdp/explorar */

use serde::Serialize;
use tracing::warn;

use crate::models::ConfiguracionRestaurante;
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{
    BdpExportArticlesRequest, BdpExportCustomersRequest, BdpExportDepartmentsRequest,
    BdpGetEmployeesRequest, BdpGetRoomsTablesRequest,
};

/// Resultado de la exploración completa de BDP.
/// Contiene el conteo de registros por categoría y metadatos.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct BdpExploracionResultado {
    /// Cantidad de artículos encontrados en BDP
    pub articulos: ExploracionCategoria,
    /// Cantidad de clientes encontrados en BDP
    pub clientes: ExploracionCategoria,
    /// Cantidad de departamentos encontrados en BDP
    pub departamentos: ExploracionCategoria,
    /// Cantidad de salones encontrados en BDP
    pub salones: ExploracionCategoria,
    /// Cantidad de empleados encontrados en BDP
    pub empleados: ExploracionCategoria,
    /// Resumen general
    pub resumen: String,
    /// Timestamp de la exploración
    pub explorado_at: chrono::NaiveDateTime,
}

/// Resultado parcial de una categoría de exploración.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ExploracionCategoria {
    /// Cantidad de registros encontrados
    pub cantidad: usize,
    /// Estado de la consulta
    pub estado: String,
    /// Mensaje de error si falló
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ExploracionCategoria {
    fn ok(cantidad: usize) -> Self {
        Self {
            cantidad,
            estado: "ok".to_string(),
            error: None,
        }
    }

    fn err(msg: &str) -> Self {
        Self {
            cantidad: 0,
            estado: "error".to_string(),
            error: Some(msg.to_string()),
        }
    }
}

pub struct BdpExplorerService;

impl BdpExplorerService {
    /// Explora BDP completo usando SOLO endpoints de lectura.
    /// NO modifica NADA. Seguro para llamar en cualquier momento.
    /* [187A-1] Exploración de solo lectura mantenida como secuencia explícita:
     * cada endpoint se captura de forma independiente y nunca aborta los demás. */
    #[allow(clippy::too_many_lines)]
    pub async fn explorar_bdp_completo(
        config: &ConfiguracionRestaurante,
    ) -> BdpExploracionResultado {
        let client = BdpWeblinkClient::new(config);

        /* Artículos: ExportArticles con rango máximo (type_price=1 para IVA incluido) */
        let articulos = match client
            .export_articles(&BdpExportArticlesRequest::all_web_articles(1))
            .await
        {
            Ok(val) => {
                /* BDP puede usar "Articles", "ArticlesListData" o "ArticleListData" */
                let count = val
                    .get("ArticlesListData")
                    .or_else(|| val.get("ArticleListData"))
                    .or_else(|| val.get("Articles"))
                    .and_then(serde_json::Value::as_array)
                    .map_or(0, std::vec::Vec::len);
                ExploracionCategoria::ok(count)
            }
            Err(e) => {
                warn!("Exploración BDP - artículos falló: {e}");
                ExploracionCategoria::err(&format!("{e}"))
            }
        };

        /* Clientes: ExportCustomers con rango completo */
        let clientes = match client
            .export_customers(&BdpExportCustomersRequest::default())
            .await
        {
            Ok(val) => {
                let count = val
                    .get("Customers")
                    .and_then(serde_json::Value::as_array)
                    .map_or(0, std::vec::Vec::len);
                ExploracionCategoria::ok(count)
            }
            Err(e) => {
                warn!("Exploración BDP - clientes falló: {e}");
                ExploracionCategoria::err(&format!("{e}"))
            }
        };

        /* Departamentos: ExportDepartments con rango completo */
        let departamentos = match client
            .export_departments(&BdpExportDepartmentsRequest::default())
            .await
        {
            Ok(val) => {
                let count = val
                    .get("Departments")
                    .or_else(|| val.get("Department"))
                    .and_then(serde_json::Value::as_array)
                    .map_or(0, std::vec::Vec::len);
                ExploracionCategoria::ok(count)
            }
            Err(e) => {
                warn!("Exploración BDP - departamentos falló: {e}");
                ExploracionCategoria::err(&format!("{e}"))
            }
        };

        /* Salones: GetRoomsTables (todos los salones) */
        let salones = match client
            .get_rooms_tables(&BdpGetRoomsTablesRequest::default())
            .await
        {
            Ok(val) => {
                let count = val
                    .get("Rooms")
                    .and_then(serde_json::Value::as_array)
                    .map_or(0, std::vec::Vec::len);
                ExploracionCategoria::ok(count)
            }
            Err(e) => {
                warn!("Exploración BDP - salones falló: {e}");
                ExploracionCategoria::err(&format!("{e}"))
            }
        };

        /* Empleados: GetEmployees (sin filtro) */
        let empleados = match client
            .get_employees(&BdpGetEmployeesRequest {
                ids: vec![],
                only_salespeople: None,
            })
            .await
        {
            Ok(val) => {
                let count = val
                    .get("Employees")
                    .or_else(|| val.get("Employee"))
                    .and_then(serde_json::Value::as_array)
                    .map_or(0, std::vec::Vec::len);
                ExploracionCategoria::ok(count)
            }
            Err(e) => {
                warn!("Exploración BDP - empleados falló: {e}");
                ExploracionCategoria::err(&format!("{e}"))
            }
        };

        let errores = [&articulos, &clientes, &departamentos, &salones, &empleados]
            .iter()
            .filter(|c| c.estado == "error")
            .count();

        let resumen = if errores == 0 {
            format!(
                "BDP explorado: {} artículos, {} clientes, {} departamentos, {} salones, {} empleados",
                articulos.cantidad,
                clientes.cantidad,
                departamentos.cantidad,
                salones.cantidad,
                empleados.cantidad,
            )
        } else {
            format!(
                "BDP explorado con {errores} errores. Artículos: {}, Clientes: {}",
                articulos.cantidad, clientes.cantidad,
            )
        };

        BdpExploracionResultado {
            articulos,
            clientes,
            departamentos,
            salones,
            empleados,
            resumen,
            explorado_at: chrono::Utc::now().naive_utc(),
        }
    }
}
