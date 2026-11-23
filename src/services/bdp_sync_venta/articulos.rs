// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
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
    ///
    /// [21AB-5b] Una línea **invitada** nunca se sustituye en silencio: si no
    /// hay equivalencia BDP, el envío se rechaza con el nombre del artículo.
    /// Antes caía al artículo por defecto y el ticket del TPV mostraba otro
    /// producto al que el cliente pidió (caso real: `1003`, mapeo desactivado).
    pub(super) async fn resolve_line_articles(
        pool: &PgPool,
        user_id: uuid::Uuid,
        lineas: &[VentaLinea],
        default_article_id: i64,
    ) -> Result<Vec<i64>, String> {
        let mut ids = Vec::with_capacity(lineas.len());
        for linea in lineas {
            let resuelto =
                Self::resolve_line_article(pool, user_id, &linea.articulo_codigo).await?;
            match resuelto {
                Some(id) => ids.push(id),
                None if linea.invitacion => {
                    return Err(format!(
                        "Invitación bloqueada: «{}» no tiene equivalencia activa en BDP. \
                         El ticket mostraría otro artículo. Marca el artículo en el catálogo \
                         BDP o retira la invitación.",
                        linea.descripcion
                    ));
                }
                None => ids.push(default_article_id),
            }
        }
        Ok(ids)
    }

    /* [21AB-5b] Código BDP de una línea, o `None` si no hay equivalencia
     * utilizable (sin mapeo, desactivado, no numérico o error de lectura). El
     * fallback al artículo por defecto lo decide quien llama: las líneas
     * normales lo admiten, las invitadas no. */
    async fn resolve_line_article(
        pool: &PgPool,
        user_id: uuid::Uuid,
        articulo_codigo: &str,
    ) -> Result<Option<i64>, String> {
        if articulo_codigo.is_empty() {
            return Ok(None);
        }
        match crate::repositories::BdpArticleMapRepository::buscar_por_codigo(
            pool,
            user_id,
            articulo_codigo,
        )
        .await
        {
            /* [20AB] Mapeo desactivado = artículo que no se vende (mismo
             * criterio que `resolve_article_local`). */
            Ok(Some(map)) if !map.activo => {
                info!("[F2.8] Mapeo '{articulo_codigo}' desactivado: sin equivalencia BDP");
                Ok(None)
            }
            Ok(Some(map)) => match map.articulo_bdp_codigo.trim().parse::<i64>() {
                Ok(code) if code > 0 => Ok(Some(code)),
                _ => {
                    info!(
                        "[F2.8] Código BDP '{}' no numérico para '{articulo_codigo}'",
                        map.articulo_bdp_codigo
                    );
                    Ok(None)
                }
            },
            Ok(None) => Ok(None),
            Err(e) => {
                /* Un fallo de lectura no se convierte en «no hay mapeo»: la
                 * línea invitada se bloquea y la normal cae al defecto. */
                warn!("[F2.8] Error buscando mapeo BDP para '{articulo_codigo}': {e}");
                Ok(None)
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CrearBdpArticleMapRequest, VentaLinea};
    use crate::repositories::BdpArticleMapRepository;
    use chrono::Utc;
    use rust_decimal::Decimal;

    /* [20AB] Un mapeo desactivado no puede seguir decidiendo el artículo BDP:
     * la línea salía con un código que BDP ya no reconoce (el demo respondía
     * `[300016]-ARTÍCULO INEXISTENTE` y la comanda no llegaba al TPV). Debe
     * caer al artículo por defecto, igual que un código sin mapeo. */
    #[sqlx::test(migrations = "./migrations")]
    async fn mapeo_desactivado_cae_al_articulo_por_defecto(pool: PgPool) {
        let user_id = uuid::Uuid::new_v4();
        let email = format!("bdp-sync-{user_id}@example.com");
        sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(&email)
            .bind("argon2_hash_placeholder")
            .execute(&pool)
            .await
            .expect("create_test_user failed");

        for (glory, bdp, activo) in [("VIVO", "1001", true), ("RETIRADO", "4031", false)] {
            BdpArticleMapRepository::crear(
                &pool,
                user_id,
                &CrearBdpArticleMapRequest {
                    articulo_glory_codigo: glory.into(),
                    articulo_bdp_codigo: Some(bdp.into()),
                    articulo_bdp_nombre: Some(glory.into()),
                    descripcion: None,
                    precio_tarifa1: None,
                    iva_pct: None,
                    departamento: None,
                    familia: None,
                    subfamilia: None,
                    activo: Some(activo),
                    barcode: None,
                },
            )
            .await
            .expect("crear mapeo");
        }

        let linea = |codigo: &str| VentaLinea {
            id: uuid::Uuid::new_v4(),
            venta_id: uuid::Uuid::new_v4(),
            articulo_codigo: codigo.into(),
            descripcion: codigo.into(),
            cantidad: Decimal::ONE,
            precio_unitario: Decimal::ONE,
            iva_pct: Decimal::from(10),
            descuento: Decimal::ZERO,
            invitacion: false,
            comentario: String::new(),
            created_at: Utc::now(),
        };
        let lineas = vec![linea("VIVO"), linea("RETIRADO"), linea("")];

        let ids = BdpSyncService::resolve_line_articles(&pool, user_id, &lineas, 9999)
            .await
            .expect("las líneas normales admiten el artículo por defecto");
        assert_eq!(ids, vec![1001, 9999, 9999]);
    }

    /* [21AB-5b] Guillermo (22/11): el ticket debe mostrar el artículo pedido.
     * Una invitada sin equivalencia BDP se bloquea; antes salía el artículo
     * por defecto y el TPV imprimía otro producto (caso real `1003`). */
    #[sqlx::test(migrations = "./migrations")]
    async fn invitada_sin_equivalencia_se_bloquea_y_no_cae_al_defecto(pool: PgPool) {
        let user_id = uuid::Uuid::new_v4();
        let email = format!("bdp-invitada-{user_id}@example.com");
        sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(&email)
            .bind("argon2_hash_placeholder")
            .execute(&pool)
            .await
            .expect("create_test_user failed");

        BdpArticleMapRepository::crear(
            &pool,
            user_id,
            &CrearBdpArticleMapRequest {
                articulo_glory_codigo: "1003".into(),
                articulo_bdp_codigo: Some("1003".into()),
                articulo_bdp_nombre: Some("CAFE CORTADO".into()),
                descripcion: None,
                precio_tarifa1: None,
                iva_pct: None,
                departamento: None,
                familia: None,
                subfamilia: None,
                activo: Some(false),
                barcode: None,
            },
        )
        .await
        .expect("crear mapeo desactivado");

        let linea = VentaLinea {
            id: uuid::Uuid::new_v4(),
            venta_id: uuid::Uuid::new_v4(),
            articulo_codigo: "1003".into(),
            descripcion: "CAFE CORTADO".into(),
            cantidad: Decimal::ONE,
            precio_unitario: Decimal::ONE,
            iva_pct: Decimal::from(10),
            descuento: Decimal::ZERO,
            invitacion: true,
            comentario: String::new(),
            created_at: Utc::now(),
        };

        let error = BdpSyncService::resolve_line_articles(&pool, user_id, &[linea], 9999)
            .await
            .expect_err("una invitada sin equivalencia no puede salir con el defecto");
        assert!(
            error.contains("CAFE CORTADO") && error.contains("Invitación bloqueada"),
            "el aviso debe nombrar el artículo: {error}"
        );
    }
}
