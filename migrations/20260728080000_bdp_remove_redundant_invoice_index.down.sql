CREATE UNIQUE INDEX IF NOT EXISTS idx_ventas_bdp_invoiced_order_unique
    ON ventas(user_id, bdp_order_id)
    WHERE bdp_invoiced = TRUE AND bdp_order_id IS NOT NULL;
