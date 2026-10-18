/* [10AA-3/L5 F2] El worker de avisos completa cada aviso con el pedido real que
 * devuelve BDP (`Orders/Get`). Esa foto se guarda aparte del payload del TPV,
 * que es lo que el TPV dijo, y nunca se sobrescribe: si el pedido cambia en BDP,
 * la siguiente lectura sobrescribe esta columna, pero el aviso original queda. */
ALTER TABLE bdp_avisos_inbox
    ADD COLUMN IF NOT EXISTS pedido_bdp JSONB;
