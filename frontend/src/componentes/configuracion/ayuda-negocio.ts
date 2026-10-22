/* [D33] Ayuda de las tarjetas de Restaurante y Operación: identidad del
 * negocio, reglas de reserva y de venta, horarios e integraciones de venta. */

import type { BloqueAyuda } from './AyudaTarjeta';

/** Tarjetas de este bloque; `ayuda.ts` une los tres para formar `IdAyuda`. */
export type IdAyudaNegocio =
  | 'negocio-nombre'
  | 'negocio-reserva'
  | 'negocio-iva'
  | 'turnos'
  | 'venta-reserva'
  | 'anulacion'
  | 'haddock'
  | 'resenas';

interface AyudaDeTarjeta {
  intro: string;
  bloques: readonly BloqueAyuda[];
}

export const AYUDA: Record<IdAyudaNegocio, AyudaDeTarjeta> = {
  'negocio-nombre': {
    intro: 'El nombre con el que el negocio aparece en la app y en los documentos.',
    bloques: [
      {
        titulo: 'Nombre del restaurante',
        queHace: 'Es el nombre del establecimiento en la aplicación, en los documentos que se generan y en el enlace público de reservas.',
        detalle: 'Si se deja vacío, algunos documentos salen sin nombre. Cámbialo y vuelve a emitir el documento que ya hayas impreso.',
      },
    ],
  },
  'negocio-reserva': {
    intro: 'Qué se le pide al cliente cuando reserva.',
    bloques: [
      {
        titulo: 'Qué datos se piden al reservar',
        queHace: 'Marca qué campos son obligatorios al crear una reserva: nombre, apellidos, email y teléfono.',
        detalle: 'Fecha y hora no se pueden desactivar: una reserva sin ellas no existe. Si exiges demasiado, el cliente abandona el formulario; con nombre y teléfono suele bastar.',
      },
    ],
  },
  'negocio-iva': {
    intro: 'El IVA que se aplica cuando nadie indica otro.',
    bloques: [
      {
        titulo: 'IVA por defecto (%)',
        queHace: 'Es el IVA que llevan los artículos nuevos si nadie lo cambia en el alta.',
        detalle: 'En España, hostelería suele ser el 10 %. Un valor equivocado obliga a corregir el IVA artículo por artículo.',
      },
    ],
  },
  turnos: {
    intro: 'Con qué horarios se clasifican las reservas y las ventas.',
    bloques: [
      {
        titulo: 'Horarios de turnos',
        queHace: 'Define los rangos de desayuno, comida y cena con los que se clasifican las reservas y las ventas.',
        detalle: 'Los rangos no deben solaparse: una reserva cae en el primer turno que la contiene. Ajústalos a tu horario real, no al de la plantilla.',
      },
    ],
  },
  'venta-reserva': {
    intro: 'Si una reserva completada se convierte sola en venta.',
    bloques: [
      {
        titulo: 'Una venta al completar cada reserva',
        queHace: 'Crea la venta automáticamente cuando una reserva se marca como completada.',
        detalle: 'Si lo desactivas, cada reserva completada habrá que pasarla a venta a mano o no contará en los ingresos.',
      },
    ],
  },
  anulacion: {
    intro: 'Cómo se anula una venta ya emitida.',
    bloques: [
      {
        titulo: 'Cómo se anula una venta',
        queHace: '«Crédito completo» exige motivo y la resta del resumen diario; «Solo estado» solo marca la venta como anulada.',
        detalle: 'Si la anulación tiene que cuadrar con el cierre de caja, usa crédito completo. El cambio afecta a las anulaciones siguientes, no a las ya hechas.',
      },
    ],
  },
  haddock: {
    intro: 'El enlace con Haddock y su sincronización de ventas.',
    bloques: [
      {
        titulo: 'Enlace y token de Haddock',
        queHace: 'Añade un enlace a tu cuenta de Haddock y sincroniza las ventas por su POS API.',
        detalle: 'El token se obtiene en Haddock → Configuración → Integraciones → POS API → Conectar. Sin token, el enlace funciona pero no se sincroniza nada.',
      },
    ],
  },
  resenas: {
    intro: 'Qué se le ofrece al cliente que sale contento.',
    bloques: [
      {
        titulo: 'Pedir reseñas y enlazar WhatsApp',
        queHace: 'Decide a dónde va un cliente que puntúa 4-5 estrellas, el teléfono que se enlaza en WhatsApp y el botón de reserva de esos mensajes.',
        detalle: 'El enlace de Google Reviews se copia del botón «Pedir reseñas» de la ficha de Google del restaurante. Si está vacío, no se ofrece reseña.',
      },
    ],
  },
};
