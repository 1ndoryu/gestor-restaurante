/* Router principal: CORS, tracing, Swagger UI, SPA y rutas /api.
 * Extraído de handlers/mod.rs. */

use axum::Router;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use super::openapi::ApiDoc;
use super::{
    admin, api_keys, auth, bdp_article_map, bdp_avisos, bdp_backup, bdp_catalogo,
    bdp_customer_sync, bdp_menu_local, bdp_purchase_note, bdp_push, campanas, canales_reserva,
    chatbot, clientes, configuracion, dashboard, errores, etiquetas, gastos, health, inactividad,
    modo_operacion, notificaciones, plano_sala, plantillas_whatsapp, recordatorios, resenas,
    reservas, trabajadores, ventas,
};
use crate::AppState;

/* [S16-H2] Límite máximo del body de peticiones: 2 MB.
 * Previene ataques de denegación de servicio mediante payloads excesivos. */
const MAX_BODY_SIZE: usize = 2 * 1024 * 1024;

/// Crea el router principal con CORS, tracing, Swagger UI y todas las rutas.
///
/// [149A-1/P1.3] El conmutador entra por parámetro: `main` comparte UNA instancia
/// entre los handlers y el poller. Con una instancia por lado, los fallos hacia el
/// BDP que registra el poller (el que más llama) no llegaban nunca a la API: la
/// degradación M2 era invisible para la UI y cada mitad podía degradarse sola.
pub fn create_router(
    pool: sqlx::PgPool,
    config: crate::config::AppConfig,
    modo_operacion: crate::services::ServicioModoOperacion,
) -> Router {
    /* [283A-20] Registro de conexiones SSE para notificaciones */
    let notif_hub = std::sync::Arc::new(crate::services::NotifHub::new());

    let state = AppState {
        pool,
        jwt_secret: config.jwt_secret.clone(),
        config,
        notif_hub,
        modo_operacion,
        resumen_cache: std::sync::Arc::new(tokio::sync::RwLock::new(
            std::collections::HashMap::new(),
        )),
        listados_cache: std::sync::Arc::new(tokio::sync::RwLock::new(
            std::collections::HashMap::new(),
        )),
    };

    /* [303A-2] CORS: restringir orígenes en producción.
     * CORS_ORIGINS env var define orígenes permitidos separados por coma.
     * Si no se define (dev local), se permite todo para no romper desarrollo.
     * En producción, definir CORS_ORIGINS=http://restaurante.wandori.us */
    let cors = match std::env::var("CORS_ORIGINS") {
        Ok(origins) if !origins.is_empty() => {
            let allowed: Vec<_> = origins
                .split(',')
                .filter_map(|o| o.trim().parse().ok())
                .collect();
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(allowed))
                .allow_methods(Any)
                .allow_headers(Any)
        }
        _ => CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any),
    };

    /* [263A-20] En produccion, servir el frontend SPA desde ./static.
     * El fallback_service reenvía rutas no-API al index.html para client-side routing. */
    let spa_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "static".to_string());
    let spa_fallback =
        ServeDir::new(&spa_dir).fallback(ServeFile::new(format!("{spa_dir}/index.html")));

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .nest("/api", api_routes())
        .fallback_service(spa_fallback)
        .layer(RequestBodyLimitLayer::new(MAX_BODY_SIZE)) /* [S16-H2] */
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

fn api_routes() -> Router<AppState> {
    Router::new()
        .merge(health::routes())
        .merge(auth::routes())
        .merge(ventas::routes())
        .merge(gastos::routes())
        .merge(reservas::routes())
        .merge(clientes::routes())
        .merge(etiquetas::routes())
        .merge(canales_reserva::routes())
        .merge(dashboard::routes())
        .merge(plano_sala::routes())
        .merge(configuracion::routes())
        .merge(modo_operacion::routes())
        .merge(bdp_article_map::routes())
        .merge(bdp_avisos::routes())
        .merge(bdp_avisos::public_routes())
        .merge(bdp_backup::routes())
        .merge(bdp_catalogo::routes())
        .merge(bdp_customer_sync::routes())
        .merge(bdp_menu_local::routes())
        .merge(bdp_purchase_note::routes())
        .merge(bdp_push::routes())
        .merge(campanas::routes())
        .merge(plantillas_whatsapp::routes())
        .merge(recordatorios::routes())
        .merge(chatbot::routes())
        .merge(api_keys::routes())
        .merge(notificaciones::routes())
        .merge(errores::routes())
        .merge(admin::routes())
        .merge(trabajadores::routes())
        .merge(resenas::routes())
        .merge(resenas::public_routes())
        .merge(inactividad::routes())
}
