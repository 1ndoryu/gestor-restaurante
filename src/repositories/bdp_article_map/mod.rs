// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [F1.4] Repositorio de mapeo artículos Glory → BDP.
 * CRUD completo + búsqueda por código para uso interno de bdp_sync.
 * [157A-7] F9.1: upsert_from_bdp() para sync enriquecida de catálogo. */

mod consultas;
mod conteos;
mod crud;
mod importacion;
mod stock;
mod tipos;

pub use tipos::{
    AjusteStockError, BdpArticleMapRepository, BdpArticleUpsertData, BdpArticleUpsertStatus,
};
