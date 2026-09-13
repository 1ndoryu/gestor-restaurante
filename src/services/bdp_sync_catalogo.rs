// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [F4.3] Submodulo de catalogo BDP (sync_catalog/sync_prices/sync_tables). Codigo movido byte-identico desde bdp_sync.rs. */

use rust_decimal::prelude::Decimal;
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::repositories::{BdpArticleMapRepository, BdpArticleUpsertStatus};
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{
    BdpCatalogSyncResult, BdpExportArticleItem, BdpExportArticlesRequest,
    BdpExportArticlesResponse, BdpGetPricesArticlesRequest, BdpGetPricesArticlesResponse,
    BdpGetRoomsTablesRequest, BdpGetRoomsTablesResponse,
};

use super::bdp_sync::BdpSyncService;

impl BdpSyncService {
    /* [157A-7] F9.1: sync_catalog — Sincroniza catálogo completo BDP → Glory.
     * Llama a ExportArticles, parsea respuesta tipada, hace upsert enriquecido
     * en bdp_article_map. Devuelve resumen de creados/actualizados/sin_cambios/errores.
     * NO requiere auth BDP en modo mock — se puede testear sin conexión. */
    /// [128A-1/F2] Aplica un upsert BDP a `bdp_article_map` respetando las
    /// reglas M6/M7 y propaga el stock al almacén por defecto. Devuelve el
    /// estado del upsert para que `sync_catalog` lo contabilice.
    /// [128A-1/F2][F2-3] La propagación de stock es responsabilidad de
    /// `upsert_from_bdp` (una sola escritura por artículo, solo para filas no
    /// omitidas). Aquí NO se escribe stock de nuevo: hacerlo duplicaba la
    /// escritura y pisaba `bdp_article_stock` para filas `Omitido*`.
    async fn aplicar_upsert(
        pool: &PgPool,
        user_id: Uuid,
        code: &str,
        upsert_data: &crate::repositories::BdpArticleUpsertData<'_>,
    ) -> Result<crate::repositories::BdpArticleUpsertStatus, String> {
        let status = crate::repositories::BdpArticleMapRepository::upsert_from_bdp(
            pool,
            user_id,
            upsert_data,
        )
        .await
        .map_err(|e| format!("[157A-7] Error upsert artículo BDP {code}: {e}"))?;

        Ok(status)
    }

    /* [F4.4] Lectura del catálogo remoto: ExportArticles con fallback de
     * perfil H-Q1-03. Devuelve los artículos y la fuente usada. */
    async fn obtener_articulos_catalogo(
        client: &BdpWeblinkClient<'_>,
        type_price: i32,
    ) -> Result<(Vec<BdpExportArticleItem>, &'static str), String> {
        /* 1. Llamar ExportArticles. [200109] (incidente 2026-09-05): si un
         * artículo web quedó con datos de validación rotos (p. ej. un alta de
         * prueba con WebArticle=true que el export no puede validar), el BDP
         * responde HTTP 200 con ErrorMessage `[200109]-ALGUNO DE LOS
         * ARTÍCULOS CONTIENE ERRORES DE VALIDACIÓN`. Antes esto rompía
         * sync-catalog para SIEMPRE (no hay DeleteArticle; el Modify del BDP
         * tampoco pudo neutralizarlo — NRE con payload mínimo). La vía de
         * escape es el mismo fallback de perfil H-Q1-03: GetPOSList del perfil
         * de items sí devuelve el catálogo operativo. Solo este error concreto
         * cae al fallback; cualquier otro error de ExportArticles (red, HTTP,
         * parseo) sigue abortando para no enmascarar problemas reales. */
        let articles_json = client
            .export_articles(&BdpExportArticlesRequest::all_web_articles(type_price))
            .await;

        /* [200109] (incidente 2026-09-05): si un artículo web quedó con datos
         * de validación rotos que el export no puede validar, el BDP responde
         * HTTP 200 con ErrorMessage `[200109]`. Marcar el motivo permite
         * distinguir este caso del vacío legítimo de ExportArticles en el
         * fallback de perfil (ver bloque H-Q1-03). */
        let mut export_fallo_200109 = false;
        let mut articles = match articles_json {
            Ok(json) => {
                let response: BdpExportArticlesResponse = serde_json::from_value(json)
                    .map_err(|e| format!("Error parseando ExportArticles: {e}"))?;
                response.articles
            }
            Err(e) => {
                let msg = e.to_string();
                if !msg.contains("[200109]") {
                    return Err(format!("Error ExportArticles: {msg}"));
                }
                export_fallo_200109 = true;
                warn!(
                    "[200109] ExportArticles con error de validación de artículo web → \
                     sync_catalog se cae al fallback de perfil H-Q1-03"
                );
                Vec::new()
            }
        };

        let mut fuente = "ExportArticles";

        /* [039A-1/H-Q1-03][200109] El BDP real devuelve ExportArticles vacío
         * (o con error [200109]) cuando no puede servir el catálogo web
         * (sin artículos web contratados, o uno roto), aunque el perfil de
         * items (el que CreateOrder puede referenciar) sí tenga artículos.
         * Si la lista llega vacía, se importa el universo del perfil vía
         * GetPOSList en vez de dejar el catálogo Glory silenciosamente vacío.
         * Si el fallback de perfil falla, el sync falla alto: nunca cerrar
         * verde con 0 sin saber si el BDP no tiene catálogo o no se pudo leer
         * (si el perfil devuelve vacío legítimo, se continúa con 0 y el aviso
         * queda en el log, igual que antes de [200109]). */
        if articles.is_empty() {
            match client.list_profile_articles().await {
                Ok(perfil) if !perfil.is_empty() => {
                    info!(
                        "[039A-1/H-Q1-03] ExportArticles vacío → catálogo importado desde \
                         GetPOSList del perfil ({} artículos)",
                        perfil.len()
                    );
                    articles = perfil;
                    fuente = "GetPOSList perfil (fallback H-Q1-03)";
                }
                Ok(_) => {
                    if export_fallo_200109 {
                        /* [200109] con perfil también vacío: sabemos que hay al
                         * menos un artículo web roto (por eso falló ExportArticles),
                         * así que cerrar verde con 0 ocultaría la corrupción. */
                        return Err(
                            "[200109] ExportArticles con error de validación y el perfil \
                             no expone artículos: hay un artículo web roto que no se puede \
                             leer"
                                .to_string(),
                        );
                    }
                    info!(
                        "[039A-1/H-Q1-03] ExportArticles vacío y perfil sin artículos: \
                         el BDP no expone catálogo"
                    );
                }
                Err(e) => {
                    return Err(format!(
                        "ExportArticles devolvió 0 y el fallback de perfil falló: {e}"
                    ));
                }
            }
        }
        Ok((articles, fuente))
    }

    pub async fn sync_catalog(
        client: &BdpWeblinkClient<'_>,
        pool: &PgPool,
        user_id: Uuid,
        type_price: i32,
    ) -> Result<crate::services::bdp_weblink_catalog::BdpCatalogSyncResult, String> {
        use crate::services::bdp_weblink_catalog::BdpCatalogSyncResult;

        let (articles, fuente) = Self::obtener_articulos_catalogo(client, type_price).await?;

        let total_bdp = articles.len();
        let mut creados: u32 = 0;
        let mut actualizados: u32 = 0;
        let mut sin_cambios: u32 = 0;
        let mut omitidos_ediciones_locales: u32 = 0;
        let mut desactivados_localmente: u32 = 0;
        let mut errores: u32 = 0;

        /* 3. Upsert cada artículo */
        for art in &articles {
            let Some(code) = art.art_code() else {
                errores += 1;
                continue;
            };

            let descripcion = art.description().to_string();
            let precio = art.price1.unwrap_or(Decimal::ZERO);
            let iva = art.tax1.unwrap_or(Decimal::ZERO);
            let dept = art.department.unwrap_or(0);
            let fam = art.family.unwrap_or(0);
            let subfam = art.subfamily.unwrap_or(0);
            let barcode = art.bar_code.as_deref().unwrap_or("");

            let upsert_data = crate::repositories::BdpArticleUpsertData {
                bdp_code: code,
                descripcion: &descripcion,
                precio_tarifa1: precio,
                iva_pct: iva,
                departamento: dept,
                familia: fam,
                subfamilia: subfam,
                activo: art.active,
                barcode,
                /* [237A-4] Stock actual del artículo — viene de PricesTableDataType
                 * en la respuesta de ExportArticles. Si el módulo de almacén no
                 * está activo, current_stock será None y queda en 0. */
                stock_actual: art.effective_stock().unwrap_or(Decimal::ZERO),
            };

            match Self::aplicar_upsert(pool, user_id, code, &upsert_data).await {
                Ok(BdpArticleUpsertStatus::Creado) => creados += 1,
                Ok(BdpArticleUpsertStatus::Actualizado) => actualizados += 1,
                Ok(BdpArticleUpsertStatus::SinCambios) => sin_cambios += 1,
                /* [128A-1/F2][M6] El import no pisa ediciones locales. */
                Ok(BdpArticleUpsertStatus::OmitidoLocalDirty) => {
                    omitidos_ediciones_locales += 1;
                }
                /* [128A-1/F2][M7] El import no reactiva artículos
                 * desactivados localmente. */
                Ok(BdpArticleUpsertStatus::OmitidoDesactivado) => {
                    desactivados_localmente += 1;
                }
                Err(e) => {
                    warn!("{e}");
                    errores += 1;
                }
            }
        }

        /* [237A-4] Info si ningún artículo trajo stock — probablemente el módulo
         * de almacén de BDP no está activo. Se usa info! en vez de warn! para
         * evitar spam: es un estado esperado si el módulo no está contratado.
         * Con el fallback de perfil (H-Q1-03) GetPOSList no trae CurrentStock
         * por diseño: ese caso no dispara el aviso de ExportArticles. */
        let stock_populado = articles.iter().any(|a| a.effective_stock().is_some());
        if !stock_populado && total_bdp > 0 && fuente == "ExportArticles" {
            info!(
                "[237A-4] Ningún artículo de ExportArticles trajo CurrentStock. \
                 Si el módulo de almacén de BDP no está activo, la columna Stock \
                 mostrará 0 para todos los artículos."
            );
        }

        info!(
            "[157A-7] sync_catalog completado ({fuente}): {} artículos BDP → {creados} creados, {actualizados} actualizados, {sin_cambios} sin cambios, {omitidos_ediciones_locales} omitidos por edición local, {desactivados_localmente} desactivados localmente, {errores} errores, stock_disponible={stock_populado}",
            total_bdp
        );

        Ok(BdpCatalogSyncResult {
            creados,
            actualizados,
            sin_cambios,
            omitidos_ediciones_locales,
            desactivados_localmente,
            errores,
            total_bdp,
        })
    }

    /* [157A-9] F9.3: Refresh de precios de artículos ya mapeados.
     * Consulta GetPricesArticles para cada artículo mapeado y actualiza precio_tarifa1.
     * Devuelve conteo de actualizados/errores. */
    pub async fn sync_prices(
        client: &BdpWeblinkClient<'_>,
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<BdpCatalogSyncResult, String> {
        let maps = BdpArticleMapRepository::listar(pool, user_id)
            .await
            .map_err(|e| format!("Error listando mapeos: {e}"))?;

        let total_bdp = maps.len();
        let mut actualizados: u32 = 0;
        let mut sin_cambios: u32 = 0;
        let mut omitidos_ediciones_locales: u32 = 0;
        let mut desactivados_localmente: u32 = 0;
        let mut errores: u32 = 0;

        for map in &maps {
            /* [128A-1/F2][M6/M7] sync_prices tampoco debe pisar ediciones
             * locales ni reactivar artículos desactivados localmente. */
            if map.local_dirty {
                omitidos_ediciones_locales += 1;
                continue;
            }
            if !map.activo {
                desactivados_localmente += 1;
                continue;
            }
            let code: i64 = if let Ok(c) = map.articulo_bdp_codigo.parse() {
                c
            } else {
                sin_cambios += 1;
                continue;
            };

            match client
                .get_prices_articles(&BdpGetPricesArticlesRequest { art_code: code })
                .await
            {
                Ok(value) => {
                    let resp: BdpGetPricesArticlesResponse = match serde_json::from_value(value) {
                        Ok(r) => r,
                        Err(e) => {
                            warn!("[157A-9] Error parseando precios BDP para {code}: {e}");
                            errores += 1;
                            continue;
                        }
                    };

                    if !resp.error_message.is_empty() {
                        warn!(
                            "[157A-9] BDP error en precios para {code}: {}",
                            resp.error_message
                        );
                        errores += 1;
                        continue;
                    }

                    let new_price = resp.prices.first().copied().unwrap_or(Decimal::ZERO);
                    /* [AUDIT-9.1] No aplicar precios negativos. Precio 0 se permite
                     * (puede ser un artículo de cortesía o servicio gratuito). */
                    if new_price < Decimal::ZERO {
                        warn!(
                            "[157A-9] BDP devolvió precio negativo {} para artículo {code}; ignorando",
                            new_price
                        );
                        errores += 1;
                        continue;
                    }
                    if (new_price - map.precio_tarifa1).abs() > Decimal::new(1, 4) {
                        /* Precio cambió — actualizar directamente via SQL */
                        match sqlx::query(
                            "UPDATE bdp_article_map SET precio_tarifa1 = $1, ultima_sync_at = NOW(), updated_at = NOW() \
                             WHERE id = $2",
                        )
                        .bind(new_price)
                        .bind(map.id)
                        .execute(pool)
                        .await
                        {
                            Ok(_) => actualizados += 1,
                            Err(e) => {
                                warn!("[157A-9] Error actualizando precio de {code}: {e}");
                                errores += 1;
                            }
                        }
                    } else {
                        sin_cambios += 1;
                    }
                }
                Err(e) => {
                    warn!("[157A-9] Error GetPricesArticles para {code}: {e}");
                    errores += 1;
                }
            }
        }

        info!(
            "[157A-9] sync_prices completado: {total_bdp} artículos → {actualizados} precios actualizados, {sin_cambios} sin cambios, {omitidos_ediciones_locales} omitidos por edición local, {desactivados_localmente} desactivados localmente, {errores} errores"
        );

        Ok(BdpCatalogSyncResult {
            creados: 0,
            actualizados,
            sin_cambios,
            omitidos_ediciones_locales,
            desactivados_localmente,
            errores,
            total_bdp,
        })
    }

    /* [157A-9] F9.4: Sincroniza salones/mesas de BDP al plano de sala de Glory.
     * Consulta GetRoomsTables → crea/actualiza ZonaSala por cada Room y Mesa por cada table.
     * Devuelve conteo de zonas y mesas procesadas. */
    pub async fn sync_tables(
        client: &BdpWeblinkClient<'_>,
        pool: &PgPool,
        user_id: Uuid,
        aplicar: bool,
    ) -> Result<SyncTablesResult, String> {
        let resp_value = client
            .get_rooms_tables(&BdpGetRoomsTablesRequest::default())
            .await
            .map_err(|e| format!("Error consultando salones BDP: {e}"))?;

        let resp: BdpGetRoomsTablesResponse = serde_json::from_value(resp_value)
            .map_err(|e| format!("Error parseando respuesta GetRoomsTables: {e}"))?;

        if !resp.error_message.is_empty() {
            return Err(format!("BDP error: {}", resp.error_message));
        }

        let mut zonas_creadas: u32 = 0;
        let mut mesas_creadas: u32 = 0;

        for room in &resp.rooms {
            /* Buscar o crear zona por nombre del salón */
            let zonas = crate::repositories::PlanoSalaRepository::listar_zonas(pool, user_id)
                .await
                .map_err(|e| format!("Error listando zonas: {e}"))?;

            let existing_zone = zonas.iter().find(|z| z.nombre == room.name).cloned();
            if existing_zone.is_none() && !aplicar {
                zonas_creadas += 1;
                mesas_creadas += u32::try_from(room.tables.len()).unwrap_or(u32::MAX);
                continue;
            }
            let zona = if let Some(existing) = existing_zone {
                existing
            } else {
                let created = crate::repositories::PlanoSalaRepository::crear_zona(
                    pool, user_id, &room.name, room.id, 800, 600,
                )
                .await
                .map_err(|e| format!("Error creando zona '{}': {e}", room.name))?;
                zonas_creadas += 1;
                created
            };

            /* Crear mesas que no existan aún en la zona */
            for &table_num in &room.tables {
                let existing =
                    crate::repositories::PlanoSalaRepository::buscar_mesa_por_zona_numero(
                        pool,
                        user_id,
                        &zona.nombre,
                        table_num,
                    )
                    .await
                    .map_err(|e| format!("Error buscando mesa {table_num}: {e}"))?;

                if existing.is_none() {
                    /* [157A-9] crear_mesa recibe CrearMesaRequest que incluye zona_id */
                    let mesa_index = i32::try_from(mesas_creadas).unwrap_or(i32::MAX);
                    let mesa_req = crate::models::CrearMesaRequest {
                        zona_id: zona.id,
                        numero: table_num,
                        pos_x: Some(20 + (mesa_index % 8) * 80),
                        pos_y: Some(20 + (mesa_index / 8) * 80),
                        ancho: Some(60),
                        alto: Some(60),
                        forma: Some("cuadrada".to_string()),
                        min_personas: Some(2),
                        max_personas: Some(4),
                    };
                    if aplicar {
                        crate::repositories::PlanoSalaRepository::crear_mesa(pool, &mesa_req)
                            .await
                            .map_err(|e| format!("Error creando mesa {table_num}: {e}"))?;
                    }
                    mesas_creadas += 1;
                }
            }
        }

        info!(
            "[157A-9] sync_tables completado: {} salones BDP → {zonas_creadas} zonas nuevas, {mesas_creadas} mesas nuevas",
            resp.rooms.len()
        );

        Ok(SyncTablesResult {
            salones_bdp: u32::try_from(resp.rooms.len()).unwrap_or(u32::MAX),
            zonas_creadas,
            mesas_creadas,
            applied: aplicar,
        })
    }
}

/// Resultado del sync de mesas BDP → Glory (F9.4).
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct SyncTablesResult {
    pub salones_bdp: u32,
    pub zonas_creadas: u32,
    pub mesas_creadas: u32,
    pub applied: bool,
}
