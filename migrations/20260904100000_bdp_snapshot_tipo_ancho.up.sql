/* [039A-1/Q1] Corrección aditiva: los snapshots parciales componen el `tipo`
 * como `parcial_{tipos.join("_")}` (p.ej. 'parcial_articulos_clientes_departamentos_empleados_salones',
 * 63 caracteres), desbordando el VARCHAR(50) original y rompiendo backup/parcial
 * con "valor demasiado largo" (500). Se ensancha sin tocar la migración ya
 * aplicada (inmutabilidad M18). Peor caso real con los 7 tipos: 73 caracteres. */

ALTER TABLE bdp_snapshots
    ALTER COLUMN tipo TYPE VARCHAR(100);