/* [F1.5] Handlers CRUD para mapeos de artículos Glory → BDP.
 * GET    /api/bdp/article-maps              — listar todos los mapeos
 * POST   /api/bdp/article-maps              — crear/upsert un mapeo
 * POST   /api/bdp/article-stock/ajustar      — ajuste manual de stock local (F3)
 * POST   /api/bdp/article-maps/import-catalog — importar catálogo desde BDP
 * POST   /api/bdp/article-maps/sync-catalog   — sync enriquecida F9.1
 * POST   /api/bdp/article-maps/sync-prices    — refresh precios F9.3
 * PATCH  /api/bdp/article-maps/:id          — actualizar parcialmente
 * DELETE /api/bdp/article-maps/:id          — eliminar un mapeo
 * POST   /api/bdp/sync-tables               — sync mesas BDP F9.4
 * GET    /api/bdp/menus/:id                 — definición menú F9.5
 * GET    /api/bdp/fastfoods/:id             — definición fastfood F9.5
 * GET    /api/bdp/packs/:id                 — definición pack F9.5
 * [147A-F5.7] import-catalog conecta con BDP Weblink para rellenar mapeos.
 * [157A-7] F9.1: sync-catalog sincroniza catálogo completo con datos enriquecidos.
 * [157A-9] F9.3-F9.5: sync-prices, sync-tables, menús/fastfoods/packs. */

mod catalogo;
mod stock;
mod sync;

pub use catalogo::*;
pub use stock::*;
pub use sync::*;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::{get, patch};
use axum::{Json, Router};
use rust_decimal::Decimal;
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::{respuesta_cacheable, AuthUser, VisibilidadCache};
use crate::models::UserRole;
use crate::models::{
    ActualizarBdpArticleMapRequest, AjustarBdpArticleStockRequest, BdpArticleMap, BdpArticleStock,
    TraspasoStockRequest,
    BdpConteoInventario, ConteoInventarioCreado, CrearBdpArticleMapRequest,
    CrearConteoInventarioRequest, RegistrarInventarioRequest,
};
use crate::repositories::{AjusteStockError, BdpArticleMapRepository};
use crate::services::bdp_weblink_catalog::{
    BdpGetFastfoodRequest, BdpGetMenuRequest, BdpGetPackRequest, BdpStockInfoEntry,
};
use crate::services::{
    payload_crear_articulo, payload_inventario, payload_modificar_articulo, payload_regularizacion,
    verificar_permiso, AccionPermiso, BdpCatalogSyncResult, BdpPushService, BdpSyncService,
    BdpWeblinkClient, ConfiguracionService, SyncTablesResult,
};
use crate::AppState;

use super::bdp_guard::exigir_modo_bdp; /* [039A-1] Guard compartido (N1). */
use super::rate_limit::{limitador_escrituras, limitar};

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct SyncTablesRequest {
    #[serde(default)]
    pub aplicar: bool,
    pub confirmacion: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/bdp/article-maps",
            get(listar_article_maps).post(crear_article_map),
        )
        .route("/bdp/article-stock", get(listar_article_stock))
        /* [039A-1/H-P1-01] El ajuste vive en /ajustar (header, utoipa y
         * frontend coinciden); el POST de la base quedaba huérfano. */
        .route(
            "/bdp/article-stock/ajustar",
            axum::routing::post(ajustar_stock),
        )
        /* [D26d] Traspaso entre almacenes: rechaza artículos no inventariables. */
        .route(
            "/bdp/article-stock/traspasar",
            axum::routing::post(traspasar_stock),
        )
        .route("/bdp/inventario", axum::routing::post(registrar_inventario))
        /* [208A-2/C3] Conteos persistidos (D3/D4): listar, guardar+aplicar, retomar. */
        .route(
            "/bdp/inventario/conteos",
            get(listar_conteos_inventario).post(crear_conteo_inventario),
        )
        .route(
            "/bdp/inventario/conteos/:id",
            get(obtener_conteo_inventario),
        )
        .route(
            "/bdp/article-maps/import-catalog",
            axum::routing::post(importar_catalogo),
        )
        .route(
            "/bdp/article-maps/sync-catalog",
            axum::routing::post(sync_catalog),
        )
        /* [157A-9] F9.3-F9.5: endpoints de precios, mesas, menús */
        .route(
            "/bdp/article-maps/sync-prices",
            axum::routing::post(sync_prices),
        )
        .route("/bdp/sync-tables", axum::routing::post(sync_tables))
        .route("/bdp/menus/:id", get(get_menu_definition))
        .route("/bdp/fastfoods/:id", get(get_fastfood_definition))
        .route("/bdp/packs/:id", get(get_pack_definition))
        .route(
            "/bdp/article-maps/:id",
            patch(actualizar_article_map).delete(eliminar_article_map),
        )
        /* Tope por IP de las escrituras de este módulo (ver rate_limit.rs). */
        .layer(axum::middleware::from_fn_with_state(
            limitador_escrituras(),
            limitar,
        ))
}
