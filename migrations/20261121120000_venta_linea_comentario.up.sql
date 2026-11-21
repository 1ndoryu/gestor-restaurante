/* [21AB-3c] Comentario de línea de venta (punto 3 de la lista de Guillermo:
 * «cambiar comentarios»). BDP lo admite por línea (`Comments[]`, probado el
 * 18/11 y el 20/11: doc `ticket-lineas-comentarios-2026-11-20.md`), pero la
 * web no tenía dónde guardarlo, así que la línea salía siempre con
 * `Comments: []`.
 *
 * `DEFAULT ''` para que las filas existentes y los INSERT antiguos queden
 * válidos: vacío significa «sin comentario» y no viaja al TPV. */

ALTER TABLE venta_lineas
    ADD COLUMN IF NOT EXISTS comentario TEXT NOT NULL DEFAULT '';
