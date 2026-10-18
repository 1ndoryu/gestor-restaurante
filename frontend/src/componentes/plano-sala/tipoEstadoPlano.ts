/* Tipo del estado de usePlanoSala: los subcomponentes del plano lo usan para tipar
 * sus props sin repetir las firmas del hook. */
import type { usePlanoSala } from './usePlanoSala';

export type EstadoPlanoSala = ReturnType<typeof usePlanoSala>;
