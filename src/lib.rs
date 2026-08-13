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
use tokio::sync::broadcast;

use crate::config::AppConfig;
use crate::models::NotificacionEvent;
use crate::services::ServicioModoOperacion;

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
}
