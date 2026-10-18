/* [267A-9/F4.1] Resolución de artículos BDP: GetArticle, perfil, catálogo local
 * (bdp_article_map) y artículo por línea de venta. */

use serde_json::Value;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::models::{ConfiguracionRestaurante, VentaLinea};
use crate::services::bdp_sync::{BdpSyncService, ResolvedArticle};
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{BdpGetArticleRequest, BdpGetPosArticlesRequest};

impl BdpSyncService {
    /// Resuelve qué artículo BDP usar. Intenta:
    /// 1. Si `bdp_default_article_code` es numérico, enriquecer con `GetArticle`
    /// 2. Si no, buscar el primer artículo del perfil
    /// 3. [128A-1/F2] Resolver desde el catálogo local (`bdp_article_map`) antes
    ///    del fallback genérico: si existe mapeo local, usar sus datos
    ///    (descripción, precio, IVA) en vez del artículo genérico con código 0.
    pub(super) async fn resolve_article(
        client: &BdpWeblinkClient<'_>,
        pool: &PgPool,
        user_id: uuid::Uuid,
        config: &ConfiguracionRestaurante,
    ) -> ResolvedArticle {
        /* Intento 1: código configurado como número → intentar GetArticle para datos reales */
        if let Ok(code) = config.bdp_default_article_code.trim().parse::<i64>() {
            if code > 0 {
                /* [157A-9] F9.2: GetArticle enriquece nombre, precio e IVA desde BDP */
                match client
                    .get_article(&BdpGetArticleRequest { art_code: code })
                    .await
                {
                    Ok(value) => {
                        let article_data = value.get("ArticleData").unwrap_or(&value);
                        let name = article_data
                            .get("ArtDescription")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&config.bdp_default_article_name)
                            .to_string();
                        #[allow(clippy::cast_precision_loss)]
                        let price = article_data
                            .get("Price1")
                            .and_then(Value::as_f64)
                            .unwrap_or(0.0);
                        let vat_pct = article_data
                            .get("TAVPer")
                            .and_then(Value::as_f64)
                            .unwrap_or(10.0);
                        return ResolvedArticle {
                            id: code,
                            name,
                            price,
                            vat_pct,
                        };
                    }
                    Err(e) => {
                        warn!("[157A-9] GetArticle falló para código {code}, usando config: {e}");
                        return ResolvedArticle {
                            id: code,
                            name: config.bdp_default_article_name.clone(),
                            price: 0.0,
                            /* [R12] Usar iva_por_defecto de config en vez de 10.0 hardcodeado */
                            vat_pct: Self::decimal_to_f64(&config.iva_por_defecto),
                        };
                    }
                }
            }
        }

        /* Intento 2: buscar primer artículo del perfil */
        match client
            .get_pos_articles(&BdpGetPosArticlesRequest::first_page(
                config.bdp_items_profile_id,
                1,
            ))
            .await
        {
            Ok(value) => {
                let default_iva = Self::decimal_to_f64(&config.iva_por_defecto);
                if let Some(article) = Self::extract_first_article(&value, default_iva) {
                    return article;
                }
            }
            Err(e) => {
                warn!("[065A-5] Error buscando artículo BDP: {e}");
            }
        }

        /* Intento 3: [128A-1/F2] catálogo local. */
        if let Some(resolved) = Self::resolve_article_local(pool, user_id, config).await {
            return resolved;
        }

        /* Fallback: genérico */
        ResolvedArticle {
            id: 0,
            name: config.bdp_default_article_name.clone(),
            price: 0.0,
            /* [R12] Usar iva_por_defecto de config en vez de 10.0 hardcodeado */
            vat_pct: Self::decimal_to_f64(&config.iva_por_defecto),
        }
    }

    /// [128A-1/F2] Intento 3 de `resolve_article`: si
    /// `bdp_default_article_code` está configurado, busca el mapeo local con
    /// ese código Glory (numérico o alfanumérico) y usa sus datos
    /// (descripción, precio, IVA) en vez del fallback genérico. Respeta
    /// `activo`: un mapeo desactivado no se vende. El id BDP del artículo se
    /// toma del mapeo local cuando el código configurado no es numérico.
    pub(crate) async fn resolve_article_local(
        pool: &PgPool,
        user_id: uuid::Uuid,
        config: &ConfiguracionRestaurante,
    ) -> Option<ResolvedArticle> {
        let code = config.bdp_default_article_code.trim();
        if code.is_empty() || code.eq_ignore_ascii_case("GLORY") {
            return None;
        }
        let config_code_numeric = code.parse::<i64>().ok().filter(|id| *id > 0);
        let map = match crate::repositories::BdpArticleMapRepository::buscar_por_codigo(
            pool, user_id, code,
        )
        .await
        {
            Ok(Some(map)) => map,
            Ok(None) => return None,
            Err(e) => {
                warn!("[128A-1/F2] Error resolviendo artículo local '{code}': {e}");
                return None;
            }
        };
        if !map.activo {
            /* Mapeo local existe pero está desactivado → usar fallback
             * genérico (el artículo no se vende). */
            return None;
        }
        let name = if map.descripcion.is_empty() {
            map.articulo_bdp_nombre.clone()
        } else {
            map.descripcion.clone()
        };
        let default_iva = Self::decimal_to_f64(&config.iva_por_defecto);
        let iva_pct = if map.iva_pct > rust_decimal::Decimal::ZERO {
            Self::decimal_to_f64(&map.iva_pct)
        } else {
            default_iva
        };
        let price = Self::decimal_to_f64(&map.precio_tarifa1);
        /* [128A-1/F2] El id BDP puede venir del código configurado (numérico,
         * semántica clásica) o del código BDP del mapeo local cuando el código
         * configurado es alfanumérico. Si ninguno es numérico, no hay un
         * ArticleId BDP válido → fallback genérico. */
        let id = match config_code_numeric {
            Some(id) => id,
            None => match map.articulo_bdp_codigo.trim().parse::<i64>() {
                Ok(id) if id > 0 => id,
                _ => return None,
            },
        };
        Some(ResolvedArticle {
            id,
            name,
            price,
            vat_pct: iva_pct,
        })
    }

    /// [F2.8] Resuelve el artículo BDP para cada línea de venta consultando `bdp_article_map`.
    /// Devuelve un Vec<i64> paralelo a `lineas` con el ID del artículo BDP.
    /// Si una línea no tiene mapeo, usa `default_article_id`.
    pub(super) async fn resolve_line_articles(
        pool: &PgPool,
        user_id: uuid::Uuid,
        lineas: &[VentaLinea],
        default_article_id: i64,
    ) -> Vec<i64> {
        let mut ids = Vec::with_capacity(lineas.len());
        for linea in lineas {
            let resolved = if linea.articulo_codigo.is_empty() {
                default_article_id
            } else {
                match crate::repositories::BdpArticleMapRepository::buscar_por_codigo(
                    pool,
                    user_id,
                    &linea.articulo_codigo,
                )
                .await
                {
                    Ok(Some(map)) => {
                        /* El código BDP puede ser numérico (ID directo) o texto (requeriría lookup).
                         * Por ahora solo soportamos códigos numéricos. */
                        match map.articulo_bdp_codigo.trim().parse::<i64>() {
                            Ok(code) if code > 0 => code,
                            _ => {
                                info!(
                                    "[F2.8] Código BDP '{}' no numérico para línea '{}', usando default",
                                    map.articulo_bdp_codigo, linea.descripcion
                                );
                                default_article_id
                            }
                        }
                    }
                    Ok(None) => {
                        /* Sin mapeo — usar artículo default */
                        default_article_id
                    }
                    Err(e) => {
                        warn!(
                            "[F2.8] Error buscando mapeo BDP para código '{}': {e}",
                            linea.articulo_codigo
                        );
                        default_article_id
                    }
                }
            };
            ids.push(resolved);
        }
        ids
    }

    /* [R12] Parámetro default_iva_pct: IVA fallback del config, no 10.0 hardcodeado. */
    pub(crate) fn extract_first_article(
        value: &Value,
        default_iva_pct: f64,
    ) -> Option<ResolvedArticle> {
        let items = value
            .get("ArticlesListData")
            .or_else(|| value.get("ArticleListData"))
            .or_else(|| value.get("Articles"))
            .and_then(|v| v.as_array())?;

        let item = items.first()?;
        #[allow(clippy::cast_possible_truncation)]
        let id = item
            .get("ArtCode")
            .or_else(|| item.get("Id"))
            .or_else(|| item.get("Code"))
            .and_then(Value::as_i64)
            .or_else(|| {
                item.get("ArtCode")
                    .or_else(|| item.get("Id"))
                    .and_then(Value::as_f64)
                    .map(|f| f as i64)
            })?;

        let name = item
            .get("ArtDescription")
            .or_else(|| item.get("Description"))
            .or_else(|| item.get("Name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Artículo BDP")
            .to_string();

        #[allow(clippy::cast_precision_loss)]
        let price = item
            .get("Price1")
            .or_else(|| item.get("Price"))
            .and_then(Value::as_f64)
            .or_else(|| item.get("Price1").and_then(Value::as_i64).map(|i| i as f64))
            .unwrap_or(0.0);

        /* [R12] Usar default_iva_pct de config en vez de 10.0 hardcodeado */
        let vat_pct = item
            .get("TAVPer")
            .or_else(|| item.get("VatPct"))
            .and_then(Value::as_f64)
            .unwrap_or(default_iva_pct);

        (id > 0).then_some(ResolvedArticle {
            id,
            name,
            price,
            vat_pct,
        })
    }
}
