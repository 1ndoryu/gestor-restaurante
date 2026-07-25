/* [BDP-STOCK-UTILS] Utilidades de formateo y exportación para la página de stock BDP.
 * Toda operación es solo lectura; no hay mutaciones de inventario. */

import type { BdpArticleMap } from '@/api/generated/gestionRestauranteAPI.schemas';

export type SortKey = 'articulo_glory_codigo' | 'articulo_bdp_codigo' | 'articulo_bdp_nombre' | 'precio_tarifa1' | 'stock_actual';
export type SortDir = 'asc' | 'desc';

export const PAGE_SIZES = [10, 25, 50] as const;

export function formatPrice(value?: string | null): string {
  if (!value || value === '0') return '—';
  const n = Number(value);
  if (Number.isNaN(n)) return '—';
  return `${n.toFixed(2)} €`;
}

export function formatStock(value?: string | null): { text: string; hasStock: boolean } {
  if (!value || value === '0') return { text: '—', hasStock: false };
  const n = Number(value);
  if (Number.isNaN(n)) return { text: '—', hasStock: false };
  return { text: n.toFixed(0), hasStock: n > 0 };
}

export function formatDate(iso?: string | null): string {
  if (!iso) return '—';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '—';
  return d.toLocaleString('es-ES', {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

export function exportToCsv(rows: BdpArticleMap[]) {
  const headers = ['Código Glory', 'Código BDP', 'Nombre BDP', 'Precio', 'Stock', 'Última sync'];
  const lines = rows.map((m) => [
    m.articulo_glory_codigo,
    m.articulo_bdp_codigo,
    m.articulo_bdp_nombre,
    m.precio_tarifa1,
    m.stock_actual,
    m.ultima_sync_at ?? '',
  ]);
  const csv = [headers, ...lines].map((line) => line.map((cell) => `"${String(cell).replace(/"/g, '""')}"`).join(',')).join('\n');
  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `stock-bdp-${new Date().toISOString().slice(0, 10)}.csv`;
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
}
