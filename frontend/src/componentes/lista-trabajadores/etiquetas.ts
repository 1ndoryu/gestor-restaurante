/* [169A-3] Etiquetas legibles de las 24 secciones (grano por entrada de
 * menú). Las marcadas "(dueño)" además exigen rol propietario en backend:
 * concederlas a un trabajador no les da acceso (defensa en profundidad).
 * El mapa va congelado: es una constante compartida, no debe mutarse. */

export const ETIQUETAS_SECCION: Readonly<Record<string, string>> = Object.freeze({
  dashboard: "Panel principal",
  ventas: "Ventas",
  gastos: "Gastos",
  compras: "Compras (BDP)",
  stock: "Stock (BDP)",
  inventario: "Inventario (BDP)",
  catalogo: "Catálogo (BDP)",
  menus_packs: "Menús y Packs",
  reservas: "Reservas",
  calendario: "Calendario",
  clientes: "Clientes",
  canales: "Canales de reserva",
  no_shows: "No-Shows",
  plano_sala: "Plano de Sala",
  campanas: "Campañas WhatsApp",
  plantillas_wa: "Plantillas WhatsApp",
  recordatorios: "Recordatorios automáticos",
  historial: "Historial (BDP)",
  sincronizacion: "Sincronización (dueño)",
  trabajadores: "Trabajadores (dueño)",
  resenas: "Reseñas",
  inactividad: "Reactivación clientes",
  configuracion: "Configuración (dueño)",
  notificaciones: "Notificaciones",
});

export function etiquetaSeccion(seccion: string): string {
  return ETIQUETAS_SECCION[seccion] ?? seccion.replace(/_/g, " ")
}
