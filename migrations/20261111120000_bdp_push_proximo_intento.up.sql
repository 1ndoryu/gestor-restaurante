/* [M21] El tope de reintentos era un abandono: cinco barridos seguidos de 60 s
 * con el TPV apagado (o su Login caído, que también cuenta como transitorio)
 * dejaban la fila en 'error' para siempre y nadie se enteraba. Ahora cada fila
 * sabe cuándo puede volver a intentarse y el barrido la respeta. Una fila
 * existente sin marca (NULL) se puede intentar ya, así que no hay que rellenar
 * nada al migrar. */
ALTER TABLE bdp_push_pendientes
    ADD COLUMN IF NOT EXISTS proximo_intento_at TIMESTAMPTZ;

/* El barrido filtra «activa y no en espera»; sin el índice esa lectura paga un
 * escaneo por usuario en cada pasada de 60 s. */
CREATE INDEX IF NOT EXISTS idx_bdp_push_pendientes_espera
    ON bdp_push_pendientes (user_id, proximo_intento_at);
