/* [283A-20] Servicio de notificaciones.
 * Crea la notificación en BD y la entrega a las conexiones SSE del usuario
 * a través de NotifHub. */

use std::sync::{Mutex, MutexGuard};

use sqlx::PgPool;
use tokio::sync::mpsc::{self, error::TrySendError};
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::Notificacion;
use crate::repositories::NotificacionRepository;

/* Avisos pendientes por conexión. Si un cliente no los consume, se descartan:
 * la notificación ya está en BD y vuelve a verse al listar. */
const BUFFER_POR_SUSCRIPTOR: usize = 64;

/// Registro de conexiones SSE. Cada suscripción tiene su propio canal mpsc
/// acotado, así un cliente lento no retiene ni bloquea los avisos de los demás.
pub struct NotifHub {
    suscriptores: Mutex<Vec<(Uuid, mpsc::Sender<Notificacion>)>>,
}

impl Default for NotifHub {
    fn default() -> Self {
        Self::new()
    }
}

impl NotifHub {
    #[must_use]
    pub fn new() -> Self {
        Self {
            suscriptores: Mutex::new(Vec::new()),
        }
    }

    /// Registra una conexión SSE del usuario y devuelve su receptor.
    pub fn suscribir(&self, user_id: Uuid) -> mpsc::Receiver<Notificacion> {
        let (tx, rx) = mpsc::channel(BUFFER_POR_SUSCRIPTOR);
        self.lock().push((user_id, tx));
        rx
    }

    /// Envía el aviso a las conexiones del usuario y elimina las ya cerradas.
    pub fn emitir(&self, user_id: Uuid, notificacion: &Notificacion) {
        self.lock().retain(|(uid, tx)| {
            if *uid != user_id {
                return !tx.is_closed();
            }
            /* try_send no espera: un buffer lleno no frena al emisor */
            !matches!(
                tx.try_send(notificacion.clone()),
                Err(TrySendError::Closed(_))
            )
        });
    }

    fn lock(&self) -> MutexGuard<'_, Vec<(Uuid, mpsc::Sender<Notificacion>)>> {
        /* Un panic previo no invalida la lista de suscriptores */
        self.suscriptores
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

pub struct NotificacionService;

impl NotificacionService {
    /// Crea una notificación y la entrega a las conexiones SSE del usuario.
    /// Si no hay conexiones abiertas, la notificación se persiste igualmente en BD.
    pub async fn emitir(
        pool: &PgPool,
        hub: &NotifHub,
        user_id: Uuid,
        tipo: &str,
        titulo: &str,
        mensaje: &str,
    ) -> Result<Notificacion, AppError> {
        let notif = NotificacionRepository::crear(pool, user_id, tipo, titulo, mensaje).await?;
        hub.emitir(user_id, &notif);
        Ok(notif)
    }

    pub async fn listar(
        pool: &PgPool,
        user_id: Uuid,
        limite: i64,
    ) -> Result<Vec<Notificacion>, AppError> {
        NotificacionRepository::listar(pool, user_id, limite).await
    }

    pub async fn contar_no_leidas(pool: &PgPool, user_id: Uuid) -> Result<i64, AppError> {
        NotificacionRepository::contar_no_leidas(pool, user_id).await
    }

    pub async fn marcar_leida(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        NotificacionRepository::marcar_leida(pool, id, user_id).await
    }

    pub async fn marcar_todas_leidas(pool: &PgPool, user_id: Uuid) -> Result<u64, AppError> {
        NotificacionRepository::marcar_todas_leidas(pool, user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn aviso(user_id: Uuid) -> Notificacion {
        Notificacion {
            id: Uuid::new_v4(),
            user_id,
            tipo: "prueba".into(),
            titulo: "t".into(),
            mensaje: "m".into(),
            leida: false,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn solo_recibe_avisos_de_su_usuario() {
        let hub = NotifHub::new();
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let mut rx_a = hub.suscribir(a);
        let mut rx_b = hub.suscribir(b);
        hub.emitir(a, &aviso(a));
        assert!(rx_a.try_recv().is_ok());
        assert!(rx_b.try_recv().is_err());
    }

    #[test]
    fn descarta_suscripciones_cerradas() {
        let hub = NotifHub::new();
        let u = Uuid::new_v4();
        drop(hub.suscribir(u));
        hub.emitir(u, &aviso(u));
        assert!(hub.lock().is_empty());
    }
}
