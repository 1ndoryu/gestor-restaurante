/* [XT1-1] Throttling global de peticiones BDP.
 *
 * Los TPV BDP suelen correr en hardware local modesto. Si varios usuarios u
 * operaciones disparan llamadas concurrentes, el TPV puede saturarse y empezar
 * a devolver timeouts en cascada. Este módulo limita la concurrencia por
 * destino BDP con un semáforo por `base_url`.
 *
 * Diseño:
 * - `BdpThrottleManager` mantiene un semáforo por `base_url` del restaurante.
 * - Cada semáforo se crea con `max_concurrent` permits, por defecto 2.
 * - Si no hay permits disponibles, se rechaza con BdpWeblinkError::Throttled.
 * - Se aplica a lecturas y escrituras BDP (excepto login, para evitar
 *   doble-throttle en peticiones autenticadas).
 */

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Límite por defecto de peticiones BDP concurrentes por destino.
const DEFAULT_MAX_CONCURRENT: usize = 2;

/// Throttle manager global para todos los destinos BDP.
pub struct BdpThrottleManager {
    per_target: Mutex<HashMap<String, Arc<Semaphore>>>,
    max_concurrent: usize,
}

impl Default for BdpThrottleManager {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_CONCURRENT)
    }
}

impl BdpThrottleManager {
    #[must_use]
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            per_target: Mutex::new(HashMap::new()),
            max_concurrent: max_concurrent.max(1),
        }
    }

    /* Recupera el mapa de semaforos aunque el mutex este envenenado por un panic ajeno.
     * El contenido es un cache de semaforos por destino, no un invariante que el panic
     * pueda dejar a medias, y entrar en panico aqui dejaria sin throttle a todos los
     * destinos BDP en lugar de degradar una sola peticion. */
    fn bloquear_mapa(&self) -> MutexGuard<'_, HashMap<String, Arc<Semaphore>>> {
        self.per_target
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn semaphore_for(&self, base_url: &str) -> Arc<Semaphore> {
        /* Normalizar la clave para evitar duplicados por trailing slash. */
        let key = base_url.trim().trim_end_matches('/').to_lowercase();
        let mut store = self.bloquear_mapa();
        if let Some(semaphore) = store.get(&key) {
            return semaphore.clone();
        }
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));
        store.insert(key, semaphore.clone());
        semaphore
    }

    /* [21AB-5c] Guillermo (22/11): «las modificaciones rápidas no pueden perderse
     * ni obligar al usuario a repetirlas». Rechazar al instante convertía el
     * propio `GetOrder` de una modificación en un `Throttled` cuando sus
     * peticiones hermanas ocupaban los dos permits, y el operador veía un error
     * por haber pulsado dos veces. La espera es acotada: si en `espera` no hay
     * hueco, sigue siendo `Throttled` (backpressure real, no cola infinita). */
    pub async fn acquire_esperando(
        &self,
        base_url: &str,
        espera: std::time::Duration,
    ) -> Result<BdpThrottleGuard, String> {
        let semaphore = self.semaphore_for(base_url);
        match tokio::time::timeout(espera, semaphore.acquire_owned()).await {
            Ok(Ok(permit)) => Ok(BdpThrottleGuard { _permit: permit }),
            Ok(Err(_)) => Err("semáforo BDP cerrado".to_string()),
            Err(_) => Err(format!(
                "BDP concurrent request limit reached (espera agotada tras {} ms)",
                espera.as_millis()
            )),
        }
    }

    /// Peticiones activas para un destino dado.
    pub fn active_requests(&self, base_url: &str) -> usize {
        let key = base_url.trim().trim_end_matches('/').to_lowercase();
        let store = self.bloquear_mapa();
        store.get(&key).map_or(0, |semaphore| {
            self.max_concurrent - semaphore.available_permits()
        })
    }

    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }
}

/// Guard que libera el permit al salir del scope.
pub struct BdpThrottleGuard {
    _permit: OwnedSemaphorePermit,
}

/// Instancia global de throttle BDP.
///
/// En el futuro podemos inicializarla desde variables de entorno o desde
/// `configuracion_restaurante` para permitir tuning por restaurante.
pub static BDP_THROTTLE: LazyLock<BdpThrottleManager> = LazyLock::new(BdpThrottleManager::default);
