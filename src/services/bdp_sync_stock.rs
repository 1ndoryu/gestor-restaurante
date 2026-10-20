// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [D26b] Submódulo de stock del BDP. Va aparte de `bdp_sync_catalogo` porque es
 * otro dominio —aquí se lee el desglose por almacén, allí el catálogo y los
 * precios— y porque el fichero de catálogo pasaba ya del límite de 500 líneas. */

use sqlx::PgPool;
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;

use crate::repositories::BdpArticleMapRepository;
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{
    BdpCatalogSyncResult, BdpGetListStockRequest, BdpGetListStockResponse, BdpListStockItemRequest,
};

use super::bdp_sync::BdpSyncService;

/* [D26b] Tandas de 200 artículos por llamada a GetListStock: acota el payload
 * sin disparar una petición por artículo. */
const ARTICULOS_POR_TANDA_STOCK: usize = 200;

impl BdpSyncService {
    /* [D26b] Stock REAL por almacén. Hasta ahora `bdp_article_stock` solo se
     * rellenaba con el stock agregado de ExportArticles (almacén "0"/"General");
     * esta función da el llamador de producción que faltaba a `get_list_stock`
     * para escribir el desglose por almacén. Usa la misma fuente que
     * `sync_catalog` (`bdp_article_map`): solo los artículos con código BDP
     * numérico > 0, que son los consultables en BDP. */
    pub async fn refrescar_stock_almacen(
        client: &BdpWeblinkClient<'_>,
        pool: &PgPool,
        user_id: Uuid,
        almacen: i32,
    ) -> Result<BdpCatalogSyncResult, String> {
        let maps = BdpArticleMapRepository::listar(pool, user_id)
            .await
            .map_err(|e| format!("Error listando mapeos: {e}"))?;

        /* Código BDP numérico → código Glory. Los códigos no numéricos o ≤ 0
         * no existen en BDP y quedan fuera de la consulta. */
        let mut glory_por_codigo: HashMap<i64, &str> = HashMap::new();
        for map in &maps {
            if let Ok(code) = map.articulo_bdp_codigo.trim().parse::<i64>() {
                if code > 0 {
                    glory_por_codigo.insert(code, map.articulo_glory_codigo.as_str());
                }
            }
        }
        let total_bdp = glory_por_codigo.len();

        let warehouse_id = almacen.to_string();
        let warehouse_name = format!("Almacén {almacen}");

        let mut actualizados: u32 = 0;
        let mut sin_cambios: u32 = 0;
        let mut errores: u32 = 0;

        let codigos: Vec<i64> = glory_por_codigo.keys().copied().collect();
        for tanda in codigos.chunks(ARTICULOS_POR_TANDA_STOCK) {
            let articles = tanda
                .iter()
                .map(|&article| BdpListStockItemRequest { article, altern: 0 })
                .collect();
            let value = client
                .get_list_stock(&BdpGetListStockRequest {
                    store: almacen,
                    articles,
                })
                .await
                .map_err(|e| format!("Error GetListStock: {e}"))?;
            let resp: BdpGetListStockResponse = serde_json::from_value(value)
                .map_err(|e| format!("Error parseando GetListStock: {e}"))?;

            /* Sin líneas Y con error global = fallo del lote entero (p. ej.
             * almacén inexistente, [200200]); ahí sí se aborta. Un error global
             * junto a líneas válidas es el caso normal de un artículo malo
             * ([200204]) y NO debe tumbar el resto. */
            if resp.stock.is_empty() && !resp.error_message.is_empty() {
                return Err(format!(
                    "GetListStock del almacén {almacen}: {}",
                    resp.error_message
                ));
            }

            for linea in &resp.stock {
                /* Línea con error propio (artículo inexistente, [200201]): se
                 * salta y se cuenta, sin abortar el lote (comportamiento real
                 * verificado del endpoint). */
                if !linea.error_message.is_empty() {
                    warn!(
                        "[D26b] GetListStock artículo {} con error: {}",
                        linea.article, linea.error_message
                    );
                    errores += 1;
                    continue;
                }
                let Some(&glory) = glory_por_codigo.get(&linea.article) else {
                    /* Línea de un artículo que no pedimos: no hay mapeo local. */
                    continue;
                };
                match BdpArticleMapRepository::upsert_stock(
                    pool,
                    user_id,
                    glory,
                    &warehouse_id,
                    &warehouse_name,
                    linea.units,
                )
                .await
                {
                    Ok(true) => actualizados += 1,
                    Ok(false) => sin_cambios += 1,
                    Err(e) => {
                        warn!("[D26b] Error guardando stock de {glory}: {e}");
                        errores += 1;
                    }
                }
            }
        }

        info!(
            "[D26b] refrescar_stock_almacen completado (almacén {almacen}): {total_bdp} artículos → {actualizados} actualizados, {sin_cambios} sin cambios, {errores} errores"
        );

        Ok(BdpCatalogSyncResult {
            creados: 0,
            actualizados,
            sin_cambios,
            omitidos_ediciones_locales: 0,
            desactivados_localmente: 0,
            errores,
            total_bdp,
        })
    }
}

#[cfg(test)]
mod tests_refrescar_stock_almacen {
    use super::*;
    use crate::services::bdp_weblink::tests::config;
    /* Solo lo usan las aserciones de este módulo; fuera de `test` sobra. */
    use rust_decimal::prelude::Decimal;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /* [D26b] BDP simulado: login y GetListStock con una línea buena y otra con
     * error de artículo. El error global [200204] convive con líneas válidas. */
    async fn montar_bdp() -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/Auth/Login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ErrorMessage": "",
                "AuthSession": { "Token": "token-bdp", "ExpiresIn_InSecconds": 3540 }
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/API/Warehouse/GetListStock"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Stock": [
                    { "Article": 1001, "Altern": 0, "Units": 5.0, "ErrorMessage": "" },
                    { "Article": 9999, "Altern": 0, "Units": 0.0,
                      "ErrorMessage": "[200201]-EL CÓDIGO DE ARTÍCULO 9999 NO EXISTE" }
                ],
                "ErrorMessage": "[200204]-ALGUNO DE LOS ARTÍCULOS CONTIENE ERRORES DE VALIDACIÓN"
            })))
            .mount(&server)
            .await;
        server
    }

    /* [D26b] La línea con error se salta y se cuenta, sin abortar el lote; la
     * línea buena sí escribe el stock del almacén pedido. */
    #[sqlx::test(migrations = "./migrations")]
    async fn refrescar_stock_salta_lineas_con_error(pool: PgPool) {
        let server = montar_bdp().await;
        let user_id = Uuid::new_v4();
        /* bdp_article_map exige un usuario real (FK user_id). */
        sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(format!("stock-almacen-{user_id}@example.com"))
            .bind("argon2_hash_placeholder")
            .execute(&pool)
            .await
            .unwrap();
        /* Dos mapeos: uno con código BDP válido y otro inexistente en el BDP. */
        for (glory, bdp) in [("ART-BUENO", "1001"), ("ART-MALO", "9999")] {
            sqlx::query(
                "INSERT INTO bdp_article_map (user_id, articulo_glory_codigo, articulo_bdp_codigo) \
                 VALUES ($1, $2, $3)",
            )
            .bind(user_id)
            .bind(glory)
            .bind(bdp)
            .execute(&pool)
            .await
            .unwrap();
        }

        let cfg = config(server.uri());
        let client = BdpWeblinkClient::new(&cfg);
        let res = BdpSyncService::refrescar_stock_almacen(&client, &pool, user_id, 1)
            .await
            .unwrap();

        assert_eq!(res.total_bdp, 2);
        assert_eq!(res.actualizados, 1);
        assert_eq!(res.errores, 1, "la línea mala se cuenta, no aborta el lote");

        let (warehouse_id, warehouse_name, stock): (String, String, Decimal) = sqlx::query_as(
            "SELECT warehouse_id, warehouse_name, stock FROM bdp_article_stock \
             WHERE user_id = $1 AND articulo_glory_codigo = $2",
        )
        .bind(user_id)
        .bind("ART-BUENO")
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(warehouse_id, "1");
        assert_eq!(warehouse_name, "Almacén 1");
        assert_eq!(stock, Decimal::new(5, 0));

        let malo: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM bdp_article_stock WHERE user_id = $1 AND articulo_glory_codigo = $2",
        )
        .bind(user_id)
        .bind("ART-MALO")
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(malo, 0, "el artículo con error no escribe stock");
    }
}
