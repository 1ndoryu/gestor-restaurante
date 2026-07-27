/* [S7-H1/287A-4] Prevenir que dos ventas locales mapeen al mismo BDP OrderId.
 * Índice parcial UNIQUE: solo aplica cuando bdp_order_id IS NOT NULL.
 * El advisory lock distribuido ya mitiga la carrera, pero esto es la
 * garantía dura a nivel de base de datos. El preflight aborta antes de crear
 * el índice si existen duplicados; nunca elimina ni fusiona ventas. */
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM ventas
        WHERE bdp_order_id IS NOT NULL
        GROUP BY user_id, bdp_order_id
        HAVING COUNT(*) > 1
    ) THEN
        RAISE EXCEPTION 'No se puede aplicar la unicidad BDP: existen ventas duplicadas por user_id y bdp_order_id. Revisar y conciliar manualmente.';
    END IF;
END
$$;

CREATE UNIQUE INDEX IF NOT EXISTS idx_ventas_bdp_order_id_unique
    ON ventas(user_id, bdp_order_id)
    WHERE bdp_order_id IS NOT NULL;
