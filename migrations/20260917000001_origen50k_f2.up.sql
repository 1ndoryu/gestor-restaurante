/* [179A-1/F2b] Compuestos del combo default en clientes y reservas.
 * Ventas/gastos ya quedaron cubiertos en D2
 * (`idx_ventas_user_fecha_origen50k`, `idx_gastos_user_fecha_origen50k`).
 * - clientes default: ORDER BY apellidos DESC, nombre ASC
 *   (`src/repositories/cliente.rs:175-186`).
 * - reservas default: ORDER BY fecha DESC, hora ASC + predicado
 *   `estado != 'cancelada'` (`src/repositories/reserva.rs:137-155`).
 * Sin CONCURRENTLY: sqlx migra en transacción y lo prohíbe; tablas de
 * decenas de miles de filas, bloqueo breve. */
CREATE INDEX IF NOT EXISTS idx_clientes_user_apellidos_f2
    ON clientes (user_id, apellidos DESC, nombre ASC);
CREATE INDEX IF NOT EXISTS idx_reservas_user_fecha_f2
    ON reservas (user_id, fecha DESC, hora ASC);
