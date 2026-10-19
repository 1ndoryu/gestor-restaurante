// sentinel-disable-file sqlx-query-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [134A-1] Datos demo de operativa: trabajadores, reseñas, reglas de inactividad,
 * paredes y notificaciones. Módulo de `seed/main.rs` para mantener el archivo
 * bajo el umbral de god-object de Sentinel (800 líneas efectivas). */

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use sqlx::PgPool;
use uuid::Uuid;

use super::ReservaCreada;

/* [134A-1] 3 trabajadores demo con distintos cargos y permisos */
pub async fn seed_trabajadores(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    let password = std::env::var("SEED_TRABAJADOR_PASSWORD")
        .map_err(|_| "SEED_TRABAJADOR_PASSWORD requerido (ver .env.example)".to_string())?;
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Error al hashear password de trabajador demo: {e}"))?
        .to_string();

    /* (nombre, email, cargo) */
    let trabajadores: &[(&str, &str, &str)] = &[
        ("Sara López", "sara.lopez@demo.com", "Camarera"),
        ("Tomás Ruiz", "tomas.ruiz@demo.com", "Jefe de sala"),
        ("Marta Gil", "marta.gil@demo.com", "Hostess"),
    ];

    for &(nombre, email, cargo) in trabajadores {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO trabajadores (id, user_id, nombre, email, password_hash, cargo) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(user_id)
        .bind(nombre)
        .bind(email)
        .bind(&hash)
        .bind(cargo)
        .execute(pool)
        .await?;
        /* [169A-3] Demo con el defecto de operativa diaria (el dueño lo
         * reconfigura por trabajador desde la UI). Copia local de
         * `models::PERMISOS_DEFECTO_TRABAJADOR` (el bin no enlaza el lib). */
        for seccion in [
            "dashboard",
            "ventas",
            "gastos",
            "reservas",
            "calendario",
            "clientes",
            "canales",
            "no_shows",
            "plano_sala",
        ] {
            sqlx::query(
                "INSERT INTO permisos_trabajador (id, trabajador_id, seccion, permitido) \
                 VALUES ($1, $2, $3, true)",
            )
            .bind(Uuid::new_v4())
            .bind(id)
            .bind(seccion)
            .execute(pool)
            .await?;
        }
    }
    println!("  {} trabajadores insertados.", trabajadores.len());
    Ok(())
}

/* [134A-1] 4 reseñas demo: 2 respondidas, 1 pendiente, 1 redirigida a Google */
pub async fn seed_resenas(
    pool: &PgPool,
    user_id: Uuid,
    reservas: &[ReservaCreada],
    cliente_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
    /* Tomar reservas completadas para vincular reseñas */
    let completadas: Vec<&ReservaCreada> = reservas
        .iter()
        .filter(|r| r.estado == "completada")
        .take(4)
        .collect();

    if completadas.is_empty() {
        println!("  Sin reservas completadas para vincular reseñas.");
        return Ok(());
    }

    /* (puntuacion: Option, comentario: Option, redirigido_google) */
    let configs: &[(Option<i16>, Option<&str>, bool)] = &[
        (
            Some(5),
            Some("Excelente servicio, volveremos sin duda."),
            true,
        ),
        (
            Some(4),
            Some("Muy buena comida, el ambiente es agradable."),
            false,
        ),
        (Some(3), None, false),
        (None, None, false),
    ];

    let mut count: u32 = 0;
    for (i, reserva) in completadas.iter().enumerate() {
        let id = Uuid::new_v4();
        let token = format!("demo-token-{i:04}");
        let (puntuacion, comentario, redirigido) = configs[i % configs.len()];
        let cliente_id = reserva.cliente_id.or_else(|| cliente_ids.get(i).copied());

        if puntuacion.is_some() {
            sqlx::query(
                "INSERT INTO resenas (id, user_id, reserva_id, cliente_id, token, \
                 puntuacion, comentario, redirigido_google, respondida_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW()) ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(user_id)
            .bind(reserva.id)
            .bind(cliente_id)
            .bind(&token)
            .bind(puntuacion)
            .bind(comentario.unwrap_or("")) /* evitar NULL explícito — usar '' como el DEFAULT */
            .bind(redirigido)
            .execute(pool)
            .await?;
        } else {
            sqlx::query(
                "INSERT INTO resenas (id, user_id, reserva_id, cliente_id, token) \
                 VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(user_id)
            .bind(reserva.id)
            .bind(cliente_id)
            .bind(&token)
            .execute(pool)
            .await?;
        }
        count += 1;
    }
    println!("  {count} reseñas insertadas.");
    Ok(())
}

/* [134A-1] 3 reglas de inactividad con distintos canales y períodos */
pub async fn seed_inactividad(
    pool: &PgPool,
    user_id: Uuid,
    _canal_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
    /* (nombre, dias_inactividad, canal, mensaje_plantilla) */
    let reglas: &[(&str, i32, &str, &str)] = &[
        (
            "Clientes sin visita en 30 días",
            30,
            "whatsapp",
            "Hola {nombre}, te echamos de menos. ¿Te apetece reservar una mesa esta semana?",
        ),
        (
            "Clientes inactivos 60 días",
            60,
            "sms",
            "Hola {nombre}, hace tiempo que no te vemos. ¡Vuelve y disfruta de nuestro menú!",
        ),
        (
            "Reactivación trimestral",
            90,
            "email",
            "Estimado/a {nombre}, te recordamos que puedes reservar tu mesa en cualquier momento.",
        ),
    ];

    for &(nombre, dias, canal, mensaje) in reglas {
        sqlx::query(
            "INSERT INTO reglas_inactividad (user_id, nombre, dias_inactividad, canal, mensaje_plantilla) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(user_id)
        .bind(nombre)
        .bind(dias)
        .bind(canal)
        .bind(mensaje)
        .execute(pool)
        .await?;
    }
    println!("  {} reglas de inactividad insertadas.", reglas.len());
    Ok(())
}

/* [134A-1] 2 paredes demo para decorar el plano de sala */
pub async fn seed_paredes(pool: &PgPool, zona_id: Uuid) -> Result<(), sqlx::Error> {
    /* [134A-8] Pared = bar horizontal: ancho = largo, alto = grosor (10). Rotación orienta. */
    let paredes: &[(i32, i32, i32, i32, f64, &str)] = &[
        (100, 50, 150, 10, 90.0, "#6b7280"), /* vertical (rotada 90°) */
        (50, 200, 200, 10, 0.0, "#6b7280"),  /* horizontal */
    ];

    for &(x, y, ancho, alto, rotacion, color) in paredes {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO paredes_sala (id, zona_id, pos_x, pos_y, ancho, alto, rotacion, color) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(zona_id)
        .bind(x)
        .bind(y)
        .bind(ancho)
        .bind(alto)
        .bind(rotacion)
        .bind(color)
        .execute(pool)
        .await?;
    }
    println!("  {} paredes insertadas.", paredes.len());
    Ok(())
}

/* 3 notificaciones demo para probar el panel */
pub async fn seed_notificaciones(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    let notis: &[(&str, &str, &str)] = &[
        (
            "nueva_reserva",
            "Nueva reserva recibida",
            "Elena Díaz ha reservado para 4 personas mañana a las 21:00.",
        ),
        (
            "no_show",
            "No-show detectado",
            "Sergio Muñoz no se presentó a su reserva de hoy a las 13:30.",
        ),
        (
            "campana_completada",
            "Campaña enviada",
            "La campaña 'Menú degustación mayo' se envió a 38 destinatarios con 4 fallos.",
        ),
    ];

    for &(tipo, titulo, mensaje) in notis {
        sqlx::query(
            "INSERT INTO notificaciones (user_id, tipo, titulo, mensaje) VALUES ($1, $2, $3, $4)",
        )
        .bind(user_id)
        .bind(tipo)
        .bind(titulo)
        .bind(mensaje)
        .execute(pool)
        .await?;
    }
    println!("  {} notificaciones insertadas.", notis.len());
    Ok(())
}
