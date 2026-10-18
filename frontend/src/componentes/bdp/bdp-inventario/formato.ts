/* Utilidades de presentación del inventario: lectura numérica tolerante a vacíos y fecha local. */

export function toNumber(value?: string | null): number {
  const n = Number(value ?? 0);
  return Number.isFinite(n) ? n : 0;
}

export function formatFecha(value: string | null | undefined): string {
  if (!value) return '—';
  const d = new Date(`${value}T00:00:00`);
  if (Number.isNaN(d.getTime())) return value;
  return d.toLocaleDateString('es-ES');
}
