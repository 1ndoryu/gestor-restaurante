/* Fechas relativas de los datos de prueba: se calculan al cargar el módulo para que
 * los registros parezcan recientes en cualquier día. */
export const hoy = new Date().toISOString();
export const ayer = new Date(Date.now() - 86_400_000).toISOString();
export const haceUnaSemana = new Date(Date.now() - 7 * 86_400_000).toISOString();
