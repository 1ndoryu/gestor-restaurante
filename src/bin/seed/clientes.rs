/* sentinel-disable-file sqlx-query-sin-macro funcion-larga-rs:
 * seed demo idempotente usa SQL dinamico para limpiar datos. */
/* Clientes demo (datos y siembra). Parte de seed (ver main.rs). */

use super::*;

type ClienteDemo = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

const DATOS_CLIENTES_DEMO: &[ClienteDemo] = &[
    (
        "María",
        "García López",
        "612345678",
        "maria.garcia@email.com",
        "",
        "Frutos secos",
        "Vino tinto",
        "",
    ),
    (
        "Carlos",
        "Rodríguez",
        "623456789",
        "carlos.rod@email.com",
        "Deloitte",
        "",
        "Cerveza",
        "",
    ),
    ("Ana", "Martínez", "634567890", "", "", "Celiaca", "", ""),
    (
        "Pedro",
        "Sánchez Ruiz",
        "645678901",
        "pedro.s@email.com",
        "",
        "",
        "",
        "Ventana",
    ),
    (
        "Laura",
        "Fernández",
        "656789012",
        "laura.f@email.com",
        "",
        "Lactosa",
        "",
        "",
    ),
    (
        "Javier",
        "López Torres",
        "667890123",
        "",
        "",
        "",
        "",
        "Terraza",
    ),
    (
        "Carmen",
        "Ruiz",
        "678901234",
        "carmen.ruiz@email.com",
        "",
        "Vegetariana",
        "Agua con gas",
        "",
    ),
    (
        "Miguel",
        "Torres",
        "689012345",
        "",
        "Eventos Sol S.L.",
        "",
        "",
        "Salón privado",
    ),
];

/* 8 clientes demo con datos variados */
pub(super) async fn seed_clientes(pool: &PgPool, user_id: Uuid) -> Result<Vec<ClienteCreado>, sqlx::Error> {
    let mut clientes = Vec::with_capacity(DATOS_CLIENTES_DEMO.len());
    for &(nombre, apellidos, tel, email, empresa, alergias, pref_beb, pref_ubi) in
        DATOS_CLIENTES_DEMO
    {
        let id: Uuid = sqlx::query_scalar!(
            "INSERT INTO clientes (user_id, nombre, apellidos, telefono, email, empresa, \
             alergias, preferencias_bebida, preferencias_ubicacion) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
            user_id,
            nombre,
            apellidos,
            tel,
            email,
            empresa,
            alergias,
            pref_beb,
            pref_ubi
        )
        .fetch_one(pool)
        .await?;
        clientes.push(ClienteCreado {
            id,
            nombre: nombre.to_string(),
            apellidos: apellidos.to_string(),
        });
    }
    println!("  {} clientes insertados.", clientes.len());
    Ok(clientes)
}
