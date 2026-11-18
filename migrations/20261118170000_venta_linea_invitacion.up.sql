/* [18AB-4] Invitación por línea de venta (Guillermo, 18/11/2026): la web
 * tiene que impedir marcar como invitación un artículo con
 * `NotAllowedAsInvitation` activo, aunque BDP lo acepte.
 *
 * `invitacion` marca la línea; el precio no sirve como señal (una cortesía
 * deliberada a precio 0 no es una invitación y el descuento del 100 % tampoco).
 * `bdp_article_map.invitacion_prohibida` guarda el flag de la ficha del
 * artículo: NULL = BDP aún no lo ha informado, así que no se bloquea. */

ALTER TABLE venta_lineas
    ADD COLUMN IF NOT EXISTS invitacion BOOLEAN NOT NULL DEFAULT false;

ALTER TABLE bdp_article_map
    ADD COLUMN IF NOT EXISTS invitacion_prohibida BOOLEAN NULL;
