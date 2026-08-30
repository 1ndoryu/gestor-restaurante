/* [264A-4] Opciones de filtro de ListaVentas, extraídas a módulo auxiliar para
 * reducir el line-count efectivo del componente (protocolo limite-lineas).
 * Son constantes puras sin estado; reutilizables por otros listados. */

export const OPCIONES_TURNO = [
  { value: 'manana', label: 'Mañana' },
  { value: 'mediodia', label: 'Mediodía' },
  { value: 'noche', label: 'Noche' },
];

export const OPCIONES_CANAL = [
  { value: 'comedor', label: 'Comedor' },
  { value: 'barra', label: 'Barra' },
  { value: 'terraza', label: 'Terraza' },
  { value: 'delivery', label: 'Delivery' },
  { value: 'just_eat', label: 'Just Eat' },
  { value: 'eventos', label: 'Eventos' },
];

export const OPCIONES_METODO_PAGO = [
  { value: 'efectivo', label: 'Efectivo' },
  { value: 'tarjeta', label: 'Tarjeta' },
  { value: 'transferencia', label: 'Transferencia' },
];

export const OPCIONES_ESTADO_HADDOCK = [
  { value: 'synced', label: 'Sincronizada' },
  { value: 'error', label: 'Con error' },
  { value: 'pending', label: 'Pendiente' },
];

export const OPCIONES_ESTADO_BDP = [
  { value: 'synced', label: 'Sincronizada' },
  { value: 'accepted', label: 'Aceptada' },
  { value: 'invoiced', label: 'Facturada' },
  { value: 'error', label: 'Con error' },
  { value: 'pending', label: 'Pendiente' },
  { value: 'cancelled', label: 'Cancelada' },
];