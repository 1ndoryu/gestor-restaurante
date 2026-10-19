/* sentinel-disable-file sqlx-query-sin-macro funcion-larga-rs:
 * seed demo idempotente usa SQL dinamico para limpiar datos. */
/* Zonas y mesas del plano demo. Parte de seed (ver main.rs). */

use super::*;

/* 1 zona de sala + 6 mesas posicionadas. Retorna (zona_id, mesa_ids) */
pub(super) async fn seed_zonas_mesas(pool: &PgPool, user_id: Uuid) -> Result<(Uuid, Vec<Uuid>), sqlx::Error> {
    let zona_id: Uuid = sqlx::query_scalar(
        "INSERT INTO zonas_sala (user_id, nombre, orden, ancho, alto) \
         VALUES ($1, 'Salón principal', 0, 800, 600) RETURNING id",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let mut mesa_ids = Vec::with_capacity(6);
    /* (numero, pos_x, pos_y, forma, min_personas, max_personas) */
    let mesas: &[(i32, i32, i32, &str, i32, i32)] = &[
        (1, 50, 50, "cuadrada", 2, 4),
        (2, 200, 50, "cuadrada", 2, 4),
        (3, 350, 50, "redonda", 4, 6),
        (4, 50, 250, "cuadrada", 2, 2),
        (5, 200, 250, "redonda", 6, 8),
        (6, 350, 250, "cuadrada", 2, 4),
    ];
    for &(num, x, y, forma, min_p, max_p) in mesas {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO mesas (zona_id, numero, pos_x, pos_y, forma, min_personas, max_personas) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
        )
        .bind(zona_id)
        .bind(num)
        .bind(x)
        .bind(y)
        .bind(forma)
        .bind(min_p)
        .bind(max_p)
        .fetch_one(pool)
        .await?;
        mesa_ids.push(id);
    }
    println!("  1 zona + {} mesas insertadas.", mesa_ids.len());
    Ok((zona_id, mesa_ids))
}
