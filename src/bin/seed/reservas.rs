/* sentinel-disable-file sqlx-query-sin-macro funcion-larga-rs:
 * seed demo idempotente usa SQL dinamico para limpiar datos. */
/* Reservas demo. Parte de seed (ver main.rs). */

use chrono::{Duration, NaiveDate};
use sqlx::PgPool;
use uuid::Uuid;

use super::{hora, ClienteCreado, ReservaCreada};

/* 14 dias x 6 reservas/dia = 84 reservas.
 * Dias pasados: ~70% completada, ~15% no_show, ~15% cancelada.
 * Hoy: mix de confirmada/pendiente/completada.
 * Futuros: 60% confirmada, 30% pendiente, 10% lista_espera. */
#[allow(
    clippy::too_many_arguments,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
pub(super) async fn seed_reservas(
    pool: &PgPool,
    user_id: Uuid,
    hoy: NaiveDate,
    canal_ids: &[Uuid],
    clientes: &[ClienteCreado],
    mesa_ids: &[Uuid],
) -> Result<Vec<ReservaCreada>, sqlx::Error> {
    let horas = [
        hora(13, 0),
        hora(13, 30),
        hora(14, 0),
        hora(20, 30),
        hora(21, 0),
        hora(21, 30),
    ];

    let mut reservas = Vec::new();
    let inicio = hoy - Duration::days(7);

    for dia in 0..14i64 {
        let fecha = inicio + Duration::days(dia);
        let es_pasado = fecha < hoy;

        for i in 0..6usize {
            let idx = (dia as usize * 6 + i) % 6;
            let personas = 2 + (i % 5) as i32;

            let (estado, no_show) = if es_pasado {
                match i % 7 {
                    0 => ("no_show", true),
                    6 => ("cancelada", false),
                    _ => ("completada", false),
                }
            } else if fecha == hoy {
                match i % 3 {
                    0 => ("confirmada", false),
                    1 => ("pendiente", false),
                    _ => ("completada", false),
                }
            } else {
                match i % 10 {
                    0 => ("lista_espera", false),
                    1 | 4 => ("pendiente", false),
                    _ => ("confirmada", false),
                }
            };

            /* [064A-1] Todas las reservas vinculadas a un cliente del CRM.
             * Antes solo ~50% tenían cliente_id (i % 2 == 0). Ahora todas lo tienen
             * para que la Ficha del Cliente aparezca siempre en reservas y ventas. */
            let c = &clientes[idx % clientes.len()];
            let (nombre_r, apellidos_r, cliente_id) =
                (c.nombre.as_str(), c.apellidos.as_str(), Some(c.id));

            /* Asignar mesa FK a 4 de cada 6 reservas */
            let mesa_id = if i < 4 {
                Some(mesa_ids[i % mesa_ids.len()])
            } else {
                None
            };
            let num_mesa: Option<i32> = if i < 4 { Some((i as i32) + 1) } else { None };

            let canal_id = Some(canal_ids[idx % canal_ids.len()]);
            let telefono = format!("6{:08}", 10_000_000 + dia * 100 + i as i64);
            let hora_reserva = horas[i].ok_or_else(|| {
                sqlx::Error::Protocol(format!("hora demo invalida en indice {i}"))
            })?;

            let id: Uuid = sqlx::query_scalar!(
                "INSERT INTO reservas (user_id, fecha, hora, nombre_cliente, apellidos_cliente, \
                 num_personas, estado, notas, telefono, canal_id, cliente_id, no_show, mesa_id, num_mesa) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) RETURNING id",
                user_id,
                fecha,
                hora_reserva,
                nombre_r,
                apellidos_r,
                personas,
                estado,
                if i == 0 { "Mesa junto a ventana" } else { "" },
                &telefono,
                canal_id,
                cliente_id,
                no_show,
                mesa_id,
                num_mesa
            )
            .fetch_one(pool)
            .await?;

            reservas.push(ReservaCreada {
                id,
                fecha,
                estado: estado.to_string(),
                cliente_id,
            });
        }
    }
    println!(
        "  {} reservas insertadas (7 pasados + hoy + 6 futuros).",
        reservas.len()
    );
    Ok(reservas)
}
