/* [169A-5/D2.1] Índices compuestos para abaratar el origen lector.
 * D1 midió en staging (~17k filas ventas/gastos): `ventas_listar` 14,2 ms
 * (Index Scan Backward + filtro user_id, 16,7k filas tocadas para 20) y los
 * SUM del dashboard 13–15 ms con Seq Scan (los índices simples
 * `idx_ventas_user_id` / `idx_ventas_fecha` no sirven al planificador para
 * este patrón). Estos compuestos cubren filtro + orden + agregados
 * (index-only scan en los SUM vía INCLUDE). Sin CONCURRENTLY: sqlx migra en
 * transacción y CONCURRENTLY está prohibido ahí; tablas pequeñas, bloqueo breve. */
CREATE INDEX IF NOT EXISTS idx_ventas_user_fecha_origen50k
    ON ventas (user_id, fecha DESC, created_at DESC)
    INCLUDE (importe_base, anulada);
CREATE INDEX IF NOT EXISTS idx_gastos_user_fecha_origen50k
    ON gastos (user_id, fecha DESC)
    INCLUDE (importe_base);
