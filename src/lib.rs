#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod config;
pub mod errors;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod repositories;
pub mod services;

use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::broadcast;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::models::{NotificacionEvent, ResumenEconomico};
use crate::services::ServicioModoOperacion;

/// Clave (restaurante, año, mes) → (instante de cálculo, resumen).
/// [169A-5/D2.2] Alias para no disparar `clippy::type_complexity`.
pub type ResumenCacheKey = (Uuid, i32, u32);
/// Valor cacheado del resumen económico con su instante de cálculo.
pub type ResumenCacheValor = (Instant, ResumenEconomico);
/// Caché en memoria del resumen económico, compartida por clones de `AppState`.
pub type ResumenCache = Arc<RwLock<HashMap<ResumenCacheKey, ResumenCacheValor>>>;

/// Clave (restaurante, endpoint) → (instante de cálculo, JSON serializado).
/// [179A-1/F1] Solo se cachea el combo default de cada listado (1ª página,
/// sin filtros ni ordenación): una entrada por (usuario, endpoint) acota la
/// memoria y cubre el tráfico lector caliente (UI abre siempre el default).
/// Clavea por `user_id` (= restaurante, igual que el resumen): jamás sale a
/// un borde compartido sin `Authorization` delante (CF ve DYNAMIC con token).
pub type ListadosCacheKey = (Uuid, &'static str);
/// Valor cacheado de un listado default con su instante de cálculo.
pub type ListadosCacheValor = (Instant, Vec<u8>);
/// Caché en memoria de listados default, compartida por clones de `AppState`.
pub type ListadosCache = Arc<RwLock<HashMap<ListadosCacheKey, ListadosCacheValor>>>;

/// Estado compartido de la aplicación — accesible desde handlers y middleware
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
    pub config: AppConfig,
    /// Canal broadcast para notificaciones SSE en tiempo real
    pub notif_tx: broadcast::Sender<NotificacionEvent>,
    /// [128A-1/F1] Conmutador de modo operativo BDP (cache TTL por proceso).
    pub modo_operacion: ServicioModoOperacion,
    /* [169A-5/D2.2] Caché en memoria del resumen económico por
     * (restaurante, año, mes). El `sub` del JWT es el propietario también
     * para trabajadores (`tid` aparte), así que la clave `user_id` equivale
     * a restaurante: todos sus usuarios ven el mismo resumen. TTL 30 s +
     * invalidación en escrituras de ventas/gastos (handlers). Un nodo,
     * memoria local: sin Redis a propósito. */
    pub resumen_cache: ResumenCache,
    /* [179A-1/F1] Caché en memoria de listados default por (restaurante,
     * endpoint). TTL 15 s + invalidación en escrituras del endpoint (handlers);
     * el TTL cubre escrituras de fondo (sync BDP/Haddock) igual que en D2. */
    pub listados_cache: ListadosCache,
}
