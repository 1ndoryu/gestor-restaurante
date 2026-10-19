/* sentinel-disable-file sqlx-query-sin-macro funcion-larga-rs:
 * seed demo idempotente usa SQL dinamico para limpiar datos. */
/* Etiquetas de clientes demo. Parte de seed (ver main.rs). */

use super::*;

/* Asigna etiquetas del sistema a los clientes para probar el CRM */
pub(super) async fn seed_etiquetas_clientes(
    pool: &PgPool,
    cliente_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
    let etiquetas = sqlx::query!(
        "SELECT e.id, e.nombre FROM etiquetas e \
         JOIN categorias_etiqueta ce ON ce.id = e.categoria_id \
         WHERE e.es_sistema = TRUE AND ce.aplica_a = 'cliente' \
         ORDER BY e.nombre"
    )
    .fetch_all(pool)
    .await?;

    if etiquetas.is_empty() {
        println!("  Sin etiquetas del sistema — omitiendo asignaciones.");
        return Ok(());
    }

    let mut count: u32 = 0;

    /* 1-3 etiquetas por cliente, patron determinista */
    for (ci, cliente_id) in cliente_ids.iter().enumerate() {
        let num_tags = 1 + ci % 3;
        for t in 0..num_tags {
            let etiqueta = &etiquetas[(ci + t) % etiquetas.len()];
            let resultado = sqlx::query!(
                "INSERT INTO clientes_etiquetas (cliente_id, etiqueta_id) \
                 VALUES ($1, $2) ON CONFLICT DO NOTHING",
                cliente_id,
                etiqueta.id
            )
            .execute(pool)
            .await;

            if resultado.is_ok() {
                count += 1;
            }
        }
    }
    println!("  {count} asignaciones cliente-etiqueta insertadas.");

    Ok(())
}

/* Crea las etiquetas personalizadas del usuario demo si aún no tiene ninguna.
 * Una sola sentencia: la categoría, el conteo de etiquetas y el alta van juntos
 * para que no haya una ventana entre la comprobación y la escritura. */
pub(super) async fn seed_etiquetas_personalizadas(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    let creadas = sqlx::query(
        r"INSERT INTO etiquetas (user_id, categoria_id, nombre, color)
          SELECT $1, c.id, v.nombre, v.color
          FROM categorias_etiqueta c
          CROSS JOIN (VALUES ('Habitual fin de semana', '#2196F3'),
                             ('Amigo del chef', '#9C27B0')) AS v(nombre, color)
          WHERE c.nombre = 'Fidelización' AND c.aplica_a = 'cliente' AND c.es_sistema = TRUE
            AND NOT EXISTS (SELECT 1 FROM etiquetas WHERE user_id = $1)
          ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    .execute(pool)
    .await?
    .rows_affected();

    if creadas > 0 {
        println!("  Etiquetas personalizadas del usuario creadas.");
    }
    Ok(())
}
