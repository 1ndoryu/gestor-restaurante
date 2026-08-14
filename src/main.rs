use glory_backend::config::AppConfig;
use glory_backend::handlers;
use glory_backend::services::{
    BdpBootstrapOutcome, BdpConfigBootstrapService, BdpOrderPollerService, RecordatorioService,
    ServicioModoOperacion,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("glory_backend=debug,tower_http=debug")
            }),
        )
        .init();

    let config = AppConfig::from_env()?;

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .connect(&config.database_url)
        .await?;

    /* Fallo cerrado: arrancar con un esquema parcial podría omitir índices de
     * identidad, armado o agenda BDP. Si una migración falla (por ejemplo por
     * códigos de cliente históricos duplicados), se reconcilia antes de servir. */
    sqlx::migrate!().run(&pool).await?;
    tracing::info!("Migraciones aplicadas correctamente");

    match BdpConfigBootstrapService::apply_from_env(&pool).await? {
        BdpBootstrapOutcome::Disabled => {
            tracing::info!("Bootstrap BDP dirigido no configurado");
        }
        BdpBootstrapOutcome::AlreadyApplied { user_id } => {
            tracing::info!(%user_id, "Bootstrap BDP ya estaba aplicado");
        }
        BdpBootstrapOutcome::Applied { user_id } => {
            tracing::info!(%user_id, "Bootstrap BDP aplicado en solo lectura");
        }
    }

    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("Servidor iniciando en {addr}");
    tracing::info!("Swagger UI disponible en http://{addr}/swagger-ui/");

    let app = handlers::create_router(pool.clone(), config);

    /* [263A-25] Background scheduler: verifica recordatorios pendientes cada 60s.
     * El ciclo busca reservas que coincidan con reglas activas y registra envíos.
     * [303A-2] Se usa un Mutex para evitar ciclos solapados si ejecutar_ciclo tarda >60s. */
    let scheduler_pool = pool.clone();
    let scheduler_handle = tokio::spawn(async move {
        let running = std::sync::Arc::new(tokio::sync::Mutex::new(()));
        let mut interval = tokio::time::interval(std::time::Duration::from_mins(1));
        loop {
            interval.tick().await;
            let guard = running.try_lock();
            if guard.is_err() {
                tracing::debug!("Scheduler: ciclo anterior aún en curso, saltando");
                continue;
            }
            match RecordatorioService::ejecutar_ciclo(&scheduler_pool).await {
                Ok(n) if n > 0 => tracing::debug!("Scheduler: {n} recordatorios procesados"),
                Err(e) => tracing::warn!("Scheduler error: {e}"),
                _ => {}
            }
        }
    });

    /* Polling BDP separado y opt-in. La configuración nace deshabilitada y
     * poll_due usa un claim PostgreSQL para evitar duplicados entre instancias. */
    let bdp_poll_pool = pool.clone();
    let bdp_poll_modo = ServicioModoOperacion::new();
    let bdp_poll_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            if let Err(error) = BdpOrderPollerService::poll_due(&bdp_poll_pool, &bdp_poll_modo).await
            {
                tracing::warn!("Scheduler BDP: {error}");
            }
        }
    });

    /* [303A-2] Graceful shutdown: señal SIGTERM/Ctrl+C cierra conexiones limpiamente */
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            tracing::info!("Señal de apagado recibida, cerrando servidor...");
        })
        .await?;

    scheduler_handle.abort();
    bdp_poll_handle.abort();
    pool.close().await;
    tracing::info!("Pool cerrado, servidor detenido limpiamente.");

    Ok(())
}
