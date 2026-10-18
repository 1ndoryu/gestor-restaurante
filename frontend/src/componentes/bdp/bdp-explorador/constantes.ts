/* Tipos, etiquetas y utilidades del explorador BDP (menús, fastfood y packs).
 * Sin React: lo comparten el componente principal y sus paneles. */
import type { ErrorResponse } from '@/api/generated/gestionRestauranteAPI.schemas';

export type ExploreType = 'all' | 'menu' | 'fastfood' | 'pack';

export const TIPOS: { value: ExploreType; label: string }[] = [
  { value: 'all', label: 'Todos' },
  { value: 'menu', label: 'Menú' },
  { value: 'fastfood', label: 'Fastfood' },
  { value: 'pack', label: 'Pack' },
];

export function tipoLabel(tipo: string) {
  return TIPOS.find((t) => t.value === tipo)?.label ?? tipo;
}

export interface BdpDefLine {
  article_code?: string | number;
  article_name?: string;
  quantity?: number;
}

export interface BdpDefinition {
  code?: string | number;
  name?: string;
  description?: string;
  lines?: BdpDefLine[];
}

export function extraerMensajeError(err: unknown): string {
  if (typeof err === 'string') return err;
  if (err instanceof Error) return err.message;
  const er = err as ErrorResponse | undefined;
  if (er?.message) return er.message;
  return 'Error al consultar. Verifica que el código existe en BDP y que la integración está activa.';
}
