/* sentinel-disable-file sqlx-query-sin-macro funcion-larga-rs:
 * seed demo idempotente usa SQL dinamico para limpiar datos. */
/* Configuración base y canales de reserva. Parte de seed (ver main.rs). */

use sqlx::PgPool;
use uuid::Uuid;

/* Configuracion base del restaurante */
pub(super) async fn seed_configuracion(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO configuracion_restaurante (user_id, nombre_restaurante, iva_por_defecto, \
         reserva_telefono_obligatorio, reserva_nombre_obligatorio, \
         google_review_url, telefono_restaurante, url_reservas) \
         VALUES ($1, 'Demo Restaurante', 10.00, true, true, \
         'https://g.page/r/demo-restaurante/review', '912345678', 'https://demo-restaurante.com/reservar') \
         ON CONFLICT (user_id) DO UPDATE SET \
         nombre_restaurante = 'Demo Restaurante', \
         google_review_url = 'https://g.page/r/demo-restaurante/review', \
         telefono_restaurante = '912345678', \
         url_reservas = 'https://demo-restaurante.com/reservar'",
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    println!("  Configuración del restaurante creada.");
    Ok(())
}

/* 4 canales de reserva */
pub(super) async fn seed_canales(pool: &PgPool, user_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
    let nombres = ["Teléfono", "WhatsApp", "Web", "Walk-in"];
    let mut ids = Vec::with_capacity(nombres.len());
    for nombre in &nombres {
        let id: Uuid = sqlx::query_scalar!(
            "INSERT INTO canales_reserva (user_id, nombre) VALUES ($1, $2) RETURNING id",
            user_id,
            *nombre
        )
        .fetch_one(pool)
        .await?;
        ids.push(id);
    }
    println!("  {} canales insertados.", ids.len());
    Ok(ids)
}
