/* sentinel-disable-file sqlx-query-sin-macro funcion-larga-rs:
 * seed demo idempotente usa SQL dinamico para limpiar datos. */
/* Campañas, plantillas y recordatorios demo. Parte de seed (ver main.rs). */

use sqlx::PgPool;
use uuid::Uuid;

/* 3 campañas de marketing en distintos estados */
#[allow(clippy::type_complexity)]
pub(super) async fn seed_campanas(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    /* (nombre, descripcion, canales, segmento, mensaje, estado, total_dest, total_env, total_fall) */
    let campanas: &[(&str, &str, &[&str], &str, &str, &str, i32, i32, i32)] = &[
        (
            "Promoción verano 2026",
            "Campaña para recuperar clientes inactivos",
            &["sms", "email"],
            "sin_3m",
            "¡Te echamos de menos! Vuelve este verano y disfruta de un 15% de descuento en tu próxima reserva.",
            "borrador",
            0, 0, 0,
        ),
        (
            "Menú degustación mayo",
            "Promoción nuevo menú degustación primavera",
            &["email"],
            "habitual",
            "Hola, hemos preparado un nuevo menú degustación de temporada. Como cliente habitual, tienes acceso prioritario.",
            "enviada",
            42, 38, 4,
        ),
        (
            "Recordatorio San Valentín",
            "Campaña cancelada — se planificó tarde",
            &["sms", "whatsapp"],
            "todos",
            "Celebra San Valentín con nosotros. Menú especial para 2 por 65€. Reservas: 912345678",
            "cancelada",
            0, 0, 0,
        ),
    ];

    for &(nombre, desc, canales, segmento, mensaje, estado, t_dest, t_env, t_fall) in campanas {
        let canales_arr: Vec<String> = canales.iter().map(|c| (*c).to_string()).collect();
        sqlx::query!(
            "INSERT INTO campanas (user_id, nombre, descripcion_interna, cuerpo_mensaje, \
             canales, segmento, estado, total_destinatarios, total_enviados, total_fallidos) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            user_id,
            nombre,
            desc,
            mensaje,
            &canales_arr,
            segmento,
            estado,
            t_dest,
            t_env,
            t_fall
        )
        .execute(pool)
        .await?;
    }
    println!("  {} campañas insertadas.", campanas.len());
    Ok(())
}

/* 3 plantillas WhatsApp en distintos estados */
#[allow(clippy::type_complexity)]
pub(super) async fn seed_plantillas_whatsapp(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    /* (nombre, categoria, idioma, cuerpo, estado, meta_template_id, meta_razon_rechazo) */
    let plantillas: &[(&str, &str, &str, &str, &str, Option<&str>, Option<&str>)] = &[
        (
            "promocion_verano",
            "MARKETING",
            "es",
            "¡Hola {{1}}! Este verano disfruta de un 15% de descuento. Te esperamos. Reserva en {{2}}.",
            "aprobada",
            Some("tpl_abc123"),
            None,
        ),
        (
            "confirmacion_reserva",
            "UTILITY",
            "es",
            "Hola {{1}}, tu reserva para {{2}} personas el {{3}} a las {{4}} está confirmada. ¡Te esperamos!",
            "borrador",
            None,
            None,
        ),
        (
            "oferta_navidad",
            "MARKETING",
            "es",
            "¡Felices fiestas {{1}}! Celebra la Navidad con nuestro menú especial por 45€/persona. Reserva: {{2}}",
            "rechazada",
            Some("tpl_xyz789"),
            Some("El contenido promocional no cumple la política de precios de Meta."),
        ),
    ];

    for &(nombre, categoria, idioma, cuerpo, estado, meta_id, razon) in plantillas {
        sqlx::query!(
            "INSERT INTO plantillas_whatsapp (user_id, nombre, categoria, idioma, cuerpo_mensaje, \
             estado, meta_template_id, meta_razon_rechazo) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            user_id,
            nombre,
            categoria,
            idioma,
            cuerpo,
            estado,
            meta_id,
            razon
        )
        .execute(pool)
        .await?;
    }
    println!("  {} plantillas WhatsApp insertadas.", plantillas.len());
    Ok(())
}

/* 3 reglas de recordatorio automatico */
pub(super) async fn seed_reglas_recordatorio(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    /* (nombre, horas_antes, canal, mensaje, activa) */
    let reglas: &[(&str, i32, &str, &str, bool)] = &[
        (
            "Recordatorio día anterior",
            24,
            "sms",
            "Hola {nombre}, te recordamos tu reserva mañana a las {hora}. ¡Te esperamos!",
            true,
        ),
        (
            "Confirmación 2h antes",
            2,
            "whatsapp",
            "Hola {nombre}, tu mesa para {num_personas} está lista hoy a las {hora}. ¿Confirmas asistencia?",
            true,
        ),
        (
            "Recordatorio email semanal",
            168,
            "email",
            "Hola {nombre}, tienes una reserva programada para el {fecha} a las {hora}. Si necesitas cambios contáctanos.",
            false,
        ),
    ];

    for &(nombre, horas, canal, mensaje, activa) in reglas {
        sqlx::query!(
            "INSERT INTO reglas_recordatorio (user_id, nombre, horas_antes, canal, mensaje_plantilla, activa) \
             VALUES ($1, $2, $3, $4, $5, $6)",
            user_id,
            nombre,
            horas,
            canal,
            mensaje,
            activa
        )
        .execute(pool)
        .await?;
    }
    println!("  {} reglas de recordatorio insertadas.", reglas.len());
    Ok(())
}
