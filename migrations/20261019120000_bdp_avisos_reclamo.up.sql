/* [10AA-3/F2] Marca cuándo el worker reclamó un aviso. Sin ella, un aviso que
 * queda en `procesando` tras una caída del worker nunca vuelve a la cola: el
 * worker solo reclama `recibido`. Con la marca, un reclamo caducado se retoma. */
ALTER TABLE bdp_avisos_inbox
    ADD COLUMN IF NOT EXISTS reclamado_en TIMESTAMPTZ;
