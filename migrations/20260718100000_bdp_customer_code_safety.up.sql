/* Un mismo usuario no puede vincular dos clientes Glory al mismo cliente BDP.
 * Si hay datos históricos duplicados, la migración falla de forma visible para
 * exigir conciliación manual; nunca corrige identidades automáticamente. */
CREATE UNIQUE INDEX IF NOT EXISTS uq_clientes_user_bdp_customer_code
    ON clientes (user_id, bdp_customer_code)
    WHERE bdp_customer_code IS NOT NULL;
