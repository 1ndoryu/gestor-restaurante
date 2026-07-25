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

export interface CsvExportOptions {
  /** Si es true exporta todas las filas; si no, solo las filas filtradas/visibles. */
  allRows: boolean;
  /** Descripción de los filtros aplicados para incluir en el nombre del archivo. */
  filterLabel?: string;
}

function escapeCsvCell(cell: string | number | null | undefined): string {
  const value = cell == null ? '' : String(cell);
  if (/[",\n\r]/.test(value)) {
    return `"${value.replace(/"/g, '""')}"`;
  }
  return value;
}

function formatNumberCell(value?: string | null): string {
  if (!value || value === '0') return '0';
  const n = Number(value);
  if (Number.isNaN(n)) return '';
  return n.toFixed(2);
}

/** Exporta el stock a CSV. Incluye BOM para Excel, columnas extendidas,
 * fila de totales y nombre de archivo descriptivo. */
export function exportToCsv(allRows: BdpArticleMap[], filteredRows: BdpArticleMap[], options: CsvExportOptions = { allRows: false }) {
  const source = options.allRows ? allRows : filteredRows;
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
  const filterPart = options.filterLabel ? `-${options.filterLabel}` : '';

  const headers = [
    'Código Glory',
    'Código BDP',
    'Nombre BDP',
    'Descripción',
    'Familia',
    'Subfamilia',
    'Departamento',
    'Activo',
    'Precio',
    'Stock',
    'Código de barras',
    'Última sync',
  ];

  let totalStock = 0;
  const dataLines = source.map((m) => {
    const stockNum = Number(m.stock_actual ?? 0);
    if (!Number.isNaN(stockNum)) {
      totalStock += stockNum;
    }
    return [
      m.articulo_glory_codigo,
      m.articulo_bdp_codigo,
      m.articulo_bdp_nombre,
      m.descripcion,
      m.familia,
      m.subfamilia,
      m.departamento,
      m.activo ? 'Sí' : 'No',
      formatNumberCell(m.precio_tarifa1),
      formatNumberCell(m.stock_actual),
      m.barcode,
      m.ultima_sync_at ? formatDate(m.ultima_sync_at) : '',
    ];
  });

  const totalLine = ['', '', '', '', '', '', '', '', 'TOTAL', totalStock.toFixed(2), '', ''];

  const csv = [headers, ...dataLines, totalLine]
    .map((line) => line.map(escapeCsvCell).join(';'))
    .join('\r\n');

  // BOM para forzar decodificación UTF-8 en Excel.
  const blob = new Blob([`\uFEFF${csv}`], { type: 'text/csv;charset=utf-8;' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `bdp-stock${filterPart}-${options.allRows ? 'all' : 'filtered'}-${timestamp}.csv`;
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
}
