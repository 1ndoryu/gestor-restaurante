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

    /* [179A-1/F4-revisado] Pool 10 → 20: la compuerta de medición se cumplió
     * tras F1-F3 (`50k-u500-f2f3.stats.jsonl`: pg p50 40 % / max 55 %,
     * app p50 47 % — CPU libre en ambos; el colapso de u1000 apunta a cola
     * del pool, no a pg). Si pg vuelve a >80 % en la re-medición, revertir. */
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(20)
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

    /* [149A-1/P1.3] Una sola instancia del conmutador para todo el proceso: la
     * comparten los handlers (API/badge) y el poller, que es quien registra los
     * fallos hacia el BDP. Antes cada uno tenía la suya y la degradación M2 no
     * cruzaba de una mitad a la otra. */
    let modo_operacion = ServicioModoOperacion::new();
    let app = handlers::create_router(pool.clone(), config, modo_operacion.clone());

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
    let bdp_poll_modo = modo_operacion.clone();
    let bdp_poll_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            if let Err(error) =
                BdpOrderPollerService::poll_due(&bdp_poll_pool, &bdp_poll_modo).await
            {
                tracing::warn!("Scheduler BDP: {error}");
            }
        }
    });

    /* [D28a] Importación automática de albaranes de compra. El intervalo y la ventana
     * vienen del entorno; BDP_ALBARANES_AUTO_SEGUNDOS=0 la desactiva. */
    lanzar_albaranes_automaticos(pool.clone(), modo_operacion.clone());
    /* [L8/D28-auto-R] Lectura incremental de documentos de venta: apagada por defecto. */
    lanzar_documentos_automaticos(pool.clone(), modo_operacion.clone());

    /* [D23a] Buzón de avisos del TPV: se procesa en bucle, no solo al arrancar, porque
     * el TPV avisa con el backend en marcha y un `finalizar` sin procesar deja la
     * venta sin marcar como facturada. La primera pasada cubre los avisos que
     * esperaron con el backend parado. */
    lanzar_avisos_automaticos(pool.clone(), modo_operacion.clone());
    /* [D-auto-push/D24-auto] Cola de envíos y catálogo programado (opt-in). */
    lanzar_push_automatico(pool.clone());
    lanzar_maestros_automaticos(pool.clone(), modo_operacion.clone());
    /* [D24-auto-2] Clientes, plano de sala y departamentos, cada uno con su
     * cadencia por cuenta. Comparten bucle: el grano es el mismo y así una sola
     * pasada decide qué toca de los tres. */
    lanzar_cadencias_maestros(pool.clone(), modo_operacion.clone());

    /* [303A-2] Graceful shutdown: señal SIGTERM/Ctrl+C cierra conexiones limpiamente */
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    /* [por que] El limitador por IP usa el peer de la conexión como último recurso: sin
     * esta línea todas las peticiones sin cabecera de proxy compartirían una sola clave. */
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
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

/* [L8/D28-auto-R] Lectura automática de documentos de venta. La cadencia y el
 * perfil son de cada cuenta (Configuración), no del entorno: el bucle solo mira
 * cada `GRANO_BUCLE_SEGUNDOS` qué cuenta tiene el turno vencido y llama a BDP por
 * esa. `0` segundos la deja apagada sin reiniciar la app. */
fn lanzar_documentos_automaticos(pool: sqlx::PgPool, modo: ServicioModoOperacion) {
    use glory_backend::services::bdp_documentos_venta as documentos;

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(
            documentos::GRANO_BUCLE_SEGUNDOS,
        ));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            let procesados = documentos::sincronizar_vencidas(&pool, &modo).await;
            if procesados > 0 {
                tracing::info!("[D28-auto] Documentos de venta guardados: {procesados}");
            }
        }
    });
}

/* [D28a] Importación automática de albaranes de compra. El intervalo y la ventana
 * vienen del entorno; BDP_ALBARANES_AUTO_SEGUNDOS=0 la desactiva. */
fn lanzar_albaranes_automaticos(pool: sqlx::PgPool, modo: ServicioModoOperacion) {
    use glory_backend::services::bdp_purchase_sync as albaranes;

    let segundos = albaranes::segundos_automaticos();
    if segundos == 0 {
        return;
    }
    let dias = albaranes::dias_automaticos();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(segundos));
        /* Si un ciclo tarda más que el intervalo, el siguiente se retrasa en vez de
         * encadenar ráfagas de llamadas a BDP. */
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            let importados = albaranes::sincronizar_automatico(&pool, &modo, dias).await;
            if importados > 0 {
                tracing::info!("[D28a] Albaranes importados automáticamente: {importados}");
            }
        }
    });
}

/* [10AA-3] Procesador del buzón de avisos del TPV. El intervalo viene del entorno;
 * BDP_AVISOS_AUTO_SEGUNDOS=0 lo desactiva (y entonces solo queda el endpoint manual). */
fn lanzar_avisos_automaticos(pool: sqlx::PgPool, modo: ServicioModoOperacion) {
    use glory_backend::services::bdp_avisos;

    let segundos = bdp_avisos::segundos_automaticos();
    if segundos == 0 {
        tracing::warn!(
            "[10AA-3] Procesador del buzón de avisos desactivado (BDP_AVISOS_AUTO_SEGUNDOS=0)"
        );
        return;
    }
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(segundos));
        /* Igual que en los albaranes: si una pasada tarda más que el intervalo, la
         * siguiente se retrasa en vez de encadenar ráfagas contra BDP. */
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            let procesados = bdp_avisos::procesar_pendientes(&pool, &modo).await;
            if procesados > 0 {
                tracing::info!("[10AA-3] Avisos del buzón procesados: {procesados}");
            }
        }
    });
}

/* [D-auto-push] Barrido de la cola Glory → BDP. Quién envía lo manda
 * `push_modalidad` de cada cuenta (solo las `automatico`); el bucle no decide
 * nada más. 0 filas no hace nada: no hay log ni estado que tocar. */
fn lanzar_push_automatico(pool: sqlx::PgPool) {
    use glory_backend::services::bdp_push::SEGUNDOS_PUSH_AUTOMATICO;
    use glory_backend::services::BdpPushFlushService;

    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(SEGUNDOS_PUSH_AUTOMATICO));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            let enviadas = BdpPushFlushService::sincronizar_cuentas_automaticas(&pool).await;
            if enviadas > 0 {
                tracing::info!("[D-auto-push] Filas enviadas a BDP: {enviadas}");
            }
        }
    });
}

/* [D24-auto] Lectura programada del catálogo de maestros (artículos, precios y
 * stock agregado). La cadencia es de cada cuenta y `0` la deja apagada. */
fn lanzar_maestros_automaticos(pool: sqlx::PgPool, modo: ServicioModoOperacion) {
    use glory_backend::services::bdp_maestros_auto as maestros;

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(
            maestros::GRANO_BUCLE_SEGUNDOS,
        ));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            let leidos = maestros::sincronizar_vencidas(&pool, &modo).await;
            if leidos > 0 {
                tracing::info!("[D24-auto] Maestros del BDP leídos: {leidos}");
            }
        }
    });
}

/* [D24-auto-2] Clientes, plano de sala y departamentos. Cada maestro tiene su
 * propia cadencia y su propia marca de última lectura; el bucle los recorre en
 * serie dentro de una pasada para no abrir tres conexiones al TPV a la vez. */
fn lanzar_cadencias_maestros(pool: sqlx::PgPool, modo: ServicioModoOperacion) {
    use glory_backend::services::bdp_cadencias_maestros as cadencias;

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(
            cadencias::GRANO_BUCLE_SEGUNDOS,
        ));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            for maestro in [
                cadencias::Maestro::Clientes,
                cadencias::Maestro::Plano,
                cadencias::Maestro::Departamentos,
            ] {
                cadencias::sincronizar_vencidas(&pool, &modo, maestro).await;
            }
        }
    });
}
