/* [L7] El conteo guardado deja de ser siempre del almacén "0" (General).
 * Antes, `crear_conteo` leía y escribía el stock local del almacén "0" y el
 * envío a BDP salía por `bdp_almacen_default`, así que contar el almacén 2
 * ajustaba el 1 en la web y mandaba otro distinto al terminal.
 * El default '0' mantiene el significado de los conteos ya guardados: no
 * tenían desglose, así que se leen como del almacén por defecto local. */
ALTER TABLE bdp_conteos_inventario
    ADD COLUMN IF NOT EXISTS warehouse_id TEXT NOT NULL DEFAULT '0';
