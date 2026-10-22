/* [D33] Texto de la ayuda de Configuración, una entrada por TARJETA y no por
 * pestaña: la pestaña mezcla ajustes que no tienen nada que ver (una conexión,
 * un interruptor de seguridad y unas claves de marketing), así que una ayuda
 * conjunta obligaba a leer siete bloques para encontrar uno. Cada tarjeta lleva
 * su propio «Ayuda» en la cabecera y su texto vive aquí, en el mismo orden en
 * que aparecen los campos, para poder revisarlo sin abrir el formulario.
 * El texto está partido en tres bloques por pestañas para no pasar del límite de
 * líneas por archivo; este archivo solo une los tres. */

import type { BloqueAyuda } from './AyudaTarjeta';
import { AYUDA as AYUDA_NEGOCIO, type IdAyudaNegocio } from './ayuda-negocio';
import { AYUDA as AYUDA_CONEXIONES, type IdAyudaConexiones } from './ayuda-conexiones';
import { AYUDA as AYUDA_AVANZADO, type IdAyudaAvanzado } from './ayuda-avanzado';

/** Identificador de la tarjeta que lleva la ayuda. Un id por tarjeta con
 * formulario o con decisiones: si una tarjeta no está aquí, no tiene ayuda. */
export type IdAyuda = IdAyudaNegocio | IdAyudaConexiones | IdAyudaAvanzado;

export interface AyudaDeTarjeta {
  intro: string;
  bloques: readonly BloqueAyuda[];
}

export const AYUDA: Record<IdAyuda, AyudaDeTarjeta> = {
  ...AYUDA_NEGOCIO,
  ...AYUDA_CONEXIONES,
  ...AYUDA_AVANZADO,
};
