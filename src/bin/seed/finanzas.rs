/* sentinel-disable-file sqlx-query-sin-macro funcion-larga-rs:
 * seed demo idempotente usa SQL dinamico para limpiar datos. */
/* Ventas y gastos demo. Parte de seed (ver main.rs). */

use super::*;

/* Ventas vinculadas a reservas completadas + 2 extras sin reserva */
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub(super) async fn seed_ventas(
    pool: &PgPool,
    user_id: Uuid,
    hoy: NaiveDate,
    reservas: &[ReservaCreada],
    cliente_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
    let turnos = ["manana", "mediodia", "noche"];
    let canales_venta = ["comedor", "barra", "terraza", "delivery"];
    let metodos = ["efectivo", "tarjeta", "transferencia"];
    let descripciones = [
        "Menú del día",
        "Cena grupo",
        "Tapas variadas",
        "Menú ejecutivo",
        "Comida terraza",
        "Brunch dominical",
    ];

    let mut count: u32 = 0;

    /* Una venta por cada reserva completada en fecha pasada o de hoy */
    for r in reservas
        .iter()
        .filter(|r| r.estado == "completada" && r.fecha <= hoy)
    {
        let idx = count as usize;
        let base = Decimal::from(45 + (idx * 17) % 200);
        let iva = (base * Decimal::new(10, 2)).round_dp(2);
        let comensales = 2 + (idx % 6) as i32;

        sqlx::query!(
            "INSERT INTO ventas (user_id, fecha, comensales, descripcion, iva_porcentaje, \
             turno, canal, metodo_pago, importe_base, importe_iva, reserva_id, cliente_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            user_id,
            r.fecha,
            comensales,
            descripciones[idx % descripciones.len()],
            Decimal::from(10),
            turnos[idx % turnos.len()],
            canales_venta[idx % canales_venta.len()],
            metodos[idx % metodos.len()],
            base,
            iva,
            Some(r.id),
            r.cliente_id
        )
        .execute(pool)
        .await?;
        count += 1;
    }

    /* 2 ventas extras sin reserva (walk-in / delivery) */
    for i in 0..2u32 {
        let fecha = hoy - Duration::days(i64::from(i));
        let base = Decimal::from(30 + i * 25);
        let iva = (base * Decimal::new(10, 2)).round_dp(2);

        sqlx::query!(
            "INSERT INTO ventas (user_id, fecha, comensales, descripcion, iva_porcentaje, \
             turno, canal, metodo_pago, importe_base, importe_iva, reserva_id, cliente_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NULL, $11)",
            user_id,
            fecha,
            2 + i as i32,
            "Pedido delivery",
            Decimal::from(10),
            "mediodia",
            "delivery",
            "tarjeta",
            base,
            iva,
            Some(cliente_ids[i as usize % cliente_ids.len()])
        )
        .execute(pool)
        .await?;
        count += 1;
    }
    println!("  {count} ventas insertadas.");
    Ok(())
}

/* Gastos: 4/dia x 8 dias pasados = 32 gastos */
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::needless_range_loop
)]
pub(super) async fn seed_gastos(pool: &PgPool, user_id: Uuid, hoy: NaiveDate) -> Result<(), sqlx::Error> {
    /* Si la tabla de categorias estuviera vacia el seed seguiria con categoria NULL;
     * un fallo real del SELECT ya no se oculta con `unwrap_or_default`. */
    let categorias: Vec<Uuid> =
        sqlx::query_scalar!("SELECT id FROM categorias_gasto ORDER BY nombre")
            .fetch_all(pool)
            .await?;

    let tipos = ["factura", "albaran", "ticket"];
    let metodos = ["efectivo", "tarjeta", "transferencia"];
    let proveedores = [
        "Distribuciones López",
        "Carnes Ibéricas S.L.",
        "Pescados del Norte",
        "Verduras Eco",
    ];

    let mut count: u32 = 0;
    for dia in 1..=8i64 {
        let fecha = hoy - Duration::days(dia);
        for i in 0..4usize {
            let idx = (dia as usize - 1) * 4 + i;
            let base = Decimal::from(20 + (idx * 13) % 400);
            let iva = (base * Decimal::new(21, 2)).round_dp(2);
            let cat_id = if categorias.is_empty() {
                None
            } else {
                Some(categorias[idx % categorias.len()])
            };

            sqlx::query!(
                "INSERT INTO gastos (user_id, fecha, proveedor, categoria_id, tipo_documento, \
                 metodo_pago, numero_documento, recurrente, importe_base, importe_iva) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                user_id,
                fecha,
                proveedores[i],
                cat_id,
                tipos[idx % tipos.len()],
                metodos[idx % metodos.len()],
                format!("DOC-{:04}", count + 1),
                i == 0,
                base,
                iva
            )
            .execute(pool)
            .await?;
            count += 1;
        }
    }
    println!("  {count} gastos insertados.");
    Ok(())
}
