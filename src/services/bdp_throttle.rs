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
use std::sync::{Arc, LazyLock, Mutex};

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
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            per_target: Mutex::new(HashMap::new()),
            max_concurrent: max_concurrent.max(1),
        }
    }

    fn semaphore_for(&self, base_url: &str) -> Arc<Semaphore> {
        /* Normalizar la clave para evitar duplicados por trailing slash. */
        let key = base_url.trim().trim_end_matches('/').to_lowercase();
        let mut store = self.per_target.lock().expect("throttle map poisoned");
        if let Some(semaphore) = store.get(&key) {
            return semaphore.clone();
        }
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));
        store.insert(key, semaphore.clone());
        semaphore
    }

    /// Intenta adquirir un permit para una petición BDP a `base_url`.
    /// Si no hay permits disponibles, devuelve error sin bloquear.
    pub fn acquire(&self, base_url: &str) -> Result<BdpThrottleGuard, String> {
        let semaphore = self.semaphore_for(base_url);
        let permit = semaphore
            .try_acquire_owned()
            .map_err(|_| "BDP concurrent request limit reached".to_string())?;
        Ok(BdpThrottleGuard { _permit: permit })
    }

    /// Peticiones activas para un destino dado.
    pub fn active_requests(&self, base_url: &str) -> usize {
        let key = base_url.trim().trim_end_matches('/').to_lowercase();
        let store = self.per_target.lock().expect("throttle map poisoned");
        store
            .get(&key)
            .map(|semaphore| self.max_concurrent - semaphore.available_permits())
            .unwrap_or(0)
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
pub static BDP_THROTTLE: LazyLock<BdpThrottleManager> =
    LazyLock::new(|| BdpThrottleManager::default());
