/* [D-auto] Enviar la cola en automático sin releer el estado deja la comanda
 * enviada y congelada en la web. Quien ya eligió automático está pidiendo justo
 * lo contrario, así que su sondeo de estados se enciende; en manual no se toca
 * nada, para no cambiar un restaurante que no lo pidió. */
UPDATE configuracion_restaurante
   SET bdp_poll_enabled = TRUE,
       updated_at = NOW()
 WHERE push_modalidad = 'automatico'
   AND bdp_poll_enabled = FALSE;
