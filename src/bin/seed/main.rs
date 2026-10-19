/* sentinel-disable-file sqlx-query-sin-macro funcion-larga-rs:
 * seed demo idempotente usa SQL dinamico para limpiar datos. */
/* [039A-1] Sin panic: todas las funciones propagan el error con `?` y `main` devuelve
 * `Result`, de modo que un fallo de siembra sale por stderr con codigo distinto de 0
 * en lugar de abortar el proceso con un panic opaco a mitad del seed. */
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let db_url = std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL requerido".to_string())?;
    let pool = PgPool::connect(&db_url).await?;

    println!("Conectado a la base de datos.");

    let email_env = std::env::var("SEED_DEMO_EMAIL")
        .map_err(|_| "SEED_DEMO_EMAIL requerido (ver .env.example)".to_string())?;
    let email = email_env.as_str();
    let password = std::env::var("SEED_DEMO_PASSWORD")
        .map_err(|_| "SEED_DEMO_PASSWORD requerido (ver .env.example)".to_string())?;

    let salt = SaltString::generate(&mut OsRng);
    /* `password_hash::Error` no implementa `std::error::Error` (el crate no activa esa
     * feature), asi que se convierte a `String` para poder propagarlo con `?`. */
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Error al hashear password demo: {e}"))?
        .to_string();

    let user_id: Uuid = sqlx::query_scalar!(
        "INSERT INTO users (email, password_hash, nombre) VALUES ($1, $2, $3) \
         ON CONFLICT (email) DO UPDATE SET password_hash = $2 \
         RETURNING id",
        email,
        &hash,
        "Demo Admin"
    )
    .fetch_one(&pool)
    .await?;

    println!("Usuario demo: {email} (id: {user_id})");

    limpiar_datos(&pool, user_id).await?;

    let hoy = Local::now().date_naive();

    seed_configuracion(&pool, user_id).await?;
    let canal_ids = seed_canales(&pool, user_id).await?;
    let clientes = seed_clientes(&pool, user_id).await?;
    let cliente_ids: Vec<Uuid> = clientes.iter().map(|c| c.id).collect();
    let zona_id = seed_zonas_mesas(&pool, user_id).await?;
    let mesa_ids = zona_id.1;
    let zona_id = zona_id.0;
    let reserva_ids = seed_reservas(&pool, user_id, hoy, &canal_ids, &clientes, &mesa_ids).await?;
    seed_ventas(&pool, user_id, hoy, &reserva_ids, &cliente_ids).await?;
    seed_gastos(&pool, user_id, hoy).await?;
    seed_etiquetas_clientes(&pool, &cliente_ids).await?;
    seed_etiquetas_personalizadas(&pool, user_id).await?;
    seed_campanas(&pool, user_id).await?;
    seed_plantillas_whatsapp(&pool, user_id).await?;
    seed_reglas_recordatorio(&pool, user_id).await?;
    seed_notificaciones(&pool, user_id).await?;
    seed_trabajadores(&pool, user_id).await?;
    seed_resenas(&pool, user_id, &reserva_ids, &cliente_ids).await?;
    seed_inactividad(&pool, user_id, &canal_ids).await?;
    seed_paredes(&pool, zona_id).await?;

    println!("\nSeed completado exitosamente.");
    Ok(())
}
/* [044A-5] Script de seed reescrito — datos breves y actualizados.
 * 14 dias (7 pasados + hoy + 6 futuros), ~6 items/dia por entidad.
 * Relaciones correctas: reservas → clientes, mesas, canales; ventas → reservas.
 * Crea: usuario demo, configuracion, canales, clientes, zona+mesas,
 *   reservas, ventas, gastos, etiquetas, campanas, plantillas, reglas, notificaciones.
 * Idempotente: limpia datos del usuario demo antes de insertar.
 * Uso: cargo run --bin seed */

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use chrono::{Duration, Local, NaiveDate, NaiveTime};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

mod operativa;
use operativa::{
    seed_inactividad, seed_notificaciones, seed_paredes, seed_resenas, seed_trabajadores,
};

mod clientes;
mod tipos;
mod mesas;
mod reservas;
mod finanzas;
mod etiquetas;
mod mensajeria;

use clientes::*;
use tipos::*;
use mesas::*;
use reservas::*;
use finanzas::*;
use etiquetas::*;
use mensajeria::*;

/* Limpia todos los datos del usuario demo respetando FKs */
async fn limpiar_datos(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    let sentencias = [
        "DELETE FROM combinacion_mesa_items WHERE combinacion_id IN (SELECT id FROM combinaciones_mesas WHERE user_id = $1)",
        "DELETE FROM combinaciones_mesas WHERE user_id = $1",
        "DELETE FROM campana_destinatarios WHERE campana_id IN (SELECT id FROM campanas WHERE user_id = $1)",
        "DELETE FROM recordatorios_enviados WHERE regla_id IN (SELECT id FROM reglas_recordatorio WHERE user_id = $1)",
        "DELETE FROM notificaciones WHERE user_id = $1",
        "DELETE FROM reglas_recordatorio WHERE user_id = $1",
        "DELETE FROM campanas WHERE user_id = $1",
        "DELETE FROM plantillas_whatsapp WHERE user_id = $1",
        "DELETE FROM clientes_etiquetas WHERE cliente_id IN (SELECT id FROM clientes WHERE user_id = $1)",
        "DELETE FROM reservas_etiquetas WHERE reserva_id IN (SELECT id FROM reservas WHERE user_id = $1)",
        "DELETE FROM ventas WHERE user_id = $1",
        "DELETE FROM gastos WHERE user_id = $1",
        "DELETE FROM reservas WHERE user_id = $1",
        "DELETE FROM clientes WHERE user_id = $1",
        "DELETE FROM canales_reserva WHERE user_id = $1",
        "DELETE FROM mesas WHERE zona_id IN (SELECT id FROM zonas_sala WHERE user_id = $1)",
        "DELETE FROM paredes_sala WHERE zona_id IN (SELECT id FROM zonas_sala WHERE user_id = $1)",
        "DELETE FROM zonas_sala WHERE user_id = $1",
        "DELETE FROM permisos_trabajador WHERE trabajador_id IN (SELECT id FROM trabajadores WHERE user_id = $1)",
        "DELETE FROM trabajadores WHERE user_id = $1",
        "DELETE FROM envios_inactividad WHERE regla_id IN (SELECT id FROM reglas_inactividad WHERE user_id = $1)",
        "DELETE FROM reglas_inactividad WHERE user_id = $1",
        "DELETE FROM resenas WHERE user_id = $1",
        "DELETE FROM api_keys WHERE user_id = $1",
    ];
    for sql in &sentencias {
        sqlx::query(sql).bind(user_id).execute(pool).await?;
    }
    println!("Datos anteriores limpiados.");
    Ok(())
}

/* `from_hms_opt` solo rechaza combinaciones que no forman una hora real; las de este
 * binario si lo son, asi que se devuelve el `Option` en lugar de entrar en panico. */
fn hora(hora: u32, minuto: u32) -> Option<NaiveTime> {
    NaiveTime::from_hms_opt(hora, minuto, 0)
}

/* Datos de cada cliente creado, para vincular reservas con nombre real */
struct ClienteCreado {
    id: Uuid,
    nombre: String,
    apellidos: String,
}

/* Datos de cada reserva creada, para vincular ventas despues */
struct ReservaCreada {
    id: Uuid,
    fecha: NaiveDate,
    estado: String,
    cliente_id: Option<Uuid>,
}
