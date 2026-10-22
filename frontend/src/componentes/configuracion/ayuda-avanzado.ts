/* [D33] Ayuda de las tarjetas de Chatbot, Acceso y Avanzado: claves del chatbot,
 * permisos, terminal, respaldos, avisos, libro de stock y datos de prueba. */

import type { BloqueAyuda } from './AyudaTarjeta';

/** Tarjetas de este bloque; `ayuda.ts` une los tres para formar `IdAyuda`. */
export type IdAyudaAvanzado =
  | 'chatbot-claves'
  | 'permisos'
  | 'terminal'
  | 'datos-prueba';

interface AyudaDeTarjeta {
  intro: string;
  bloques: readonly BloqueAyuda[];
}

export const AYUDA: Record<IdAyudaAvanzado, AyudaDeTarjeta> = {
  'chatbot-claves': {
    intro: 'Cómo entra un chatbot externo a los datos del restaurante.',
    bloques: [
      {
        titulo: 'Crear una clave API',
        queHace: 'Genera una clave para que el chatbot consulte reservas, carta y disponibilidad.',
        detalle: 'La clave se ve una sola vez: cópiala al crearla. Si se pierde, se revoca y se crea otra; no se puede recuperar.',
      },
      {
        titulo: 'Revocar una clave',
        queHace: 'Anula una clave existente al instante.',
        detalle: 'El chatbot deja de funcionar en el siguiente mensaje. Revoca las claves que ya no uses: una clave olvidada es una puerta abierta.',
      },
    ],
  },
  permisos: {
    intro: 'Qué puede hacer cada rol en cada acción sensible.',
    bloques: [
      {
        titulo: 'Quién puede tocar cada cosa',
        queHace: 'Elige, por acción (catálogo, stock, albaranes, anulaciones, pagos y facturas), si puede hacerlo solo el admin, también los trabajadores o cualquiera.',
        detalle: 'El servidor aplica el permiso en cada operación: la pantalla solo lo declara. Un cambio afecta al siguiente intento, no a lo ya hecho.',
      },
    ],
  },
  terminal: {
    intro: 'La contraseña que protege el terminal del local.',
    bloques: [
      {
        titulo: 'Contraseña del terminal',
        queHace: 'Rota la contraseña del terminal del local pidiendo antes la actual.',
        detalle: 'Estar dentro de una sesión abierta no basta para cambiarla. Cada intento de acceso queda registrado, con el resultado y sin guardar nunca la contraseña.',
      },
    ],
  },
  'datos-prueba': {
    intro: 'Contenido ficticio para probar sin tocar los datos reales.',
    bloques: [
      {
        titulo: 'Datos de ejemplo y borrado',
        queHace: 'Muestra datos ficticios en las pantallas BDP y permite recargar o borrar el contenido de prueba de la cuenta.',
        detalle: 'El modo de ejemplo no escribe en el TPV. «Borrar todos los datos» elimina reservas, ventas, clientes, artículos y comandas, y conserva la cuenta: no se puede deshacer.',
      },
    ],
  },
};
