/* [10AA-3/L5 F2] Revierte la columna de la foto de pedido de F2. */
ALTER TABLE bdp_avisos_inbox
    DROP COLUMN IF EXISTS pedido_bdp;
