/* [HOOK-BDP-STOCK] Filtros, ordenación y paginación para la vista de stock BDP.
 * Mantiene el estado local de filtros y deriva la lista paginada de forma eficiente. */

import { useMemo, useState } from 'react';
import type { BdpArticleMap } from '@/api/generated/gestionRestauranteAPI.schemas';
import type { SortDir, SortKey } from '@/componentes/bdp/bdp-stock-utils';

export function useBdpStockFilters(rows: BdpArticleMap[]) {
  const [filtro, setFiltro] = useState('');
  const [stockFilter, setStockFilter] = useState<'all' | 'with' | 'without'>('all');
  const [activeFilter, setActiveFilter] = useState<'all' | 'active' | 'inactive'>('all');
  const [sortKey, setSortKey] = useState<SortKey>('articulo_glory_codigo');
  const [sortDir, setSortDir] = useState<SortDir>('asc');
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState<10 | 25 | 50>(25);

  const filtered = useMemo(() => {
    const q = filtro.trim().toLowerCase();
    return rows.filter((m) => {
      const matchesText =
        !q ||
        m.articulo_glory_codigo?.toLowerCase().includes(q) ||
        m.articulo_bdp_codigo?.toLowerCase().includes(q) ||
        m.articulo_bdp_nombre?.toLowerCase().includes(q);

      let matchesStock = true;
      if (stockFilter === 'with') matchesStock = !!m.stock_actual && m.stock_actual !== '0';
      if (stockFilter === 'without') matchesStock = !m.stock_actual || m.stock_actual === '0';

      let matchesActive = true;
      if (activeFilter === 'active') matchesActive = m.activo !== false;
      if (activeFilter === 'inactive') matchesActive = m.activo === false;

      return matchesText && matchesStock && matchesActive;
    });
  }, [rows, filtro, stockFilter, activeFilter]);

  const sorted = useMemo(() => {
    return [...filtered].sort((a, b) => {
      const aVal = a[sortKey];
      const bVal = b[sortKey];
      const aStr = aVal === null || aVal === undefined ? '' : String(aVal);
      const bStr = bVal === null || bVal === undefined ? '' : String(bVal);
      const comparison = aStr.localeCompare(bStr, undefined, { numeric: true, sensitivity: 'base' });
      return sortDir === 'asc' ? comparison : -comparison;
    });
  }, [filtered, sortKey, sortDir]);

  const totalPages = Math.max(1, Math.ceil(sorted.length / pageSize));
  const safePage = Math.min(page, totalPages);
  const paginated = useMemo(() => {
    const start = (safePage - 1) * pageSize;
    return sorted.slice(start, start + pageSize);
  }, [sorted, safePage, pageSize]);

  return {
    filtro,
    setFiltro,
    stockFilter,
    setStockFilter,
    activeFilter,
    setActiveFilter,
    sortKey,
    setSortKey,
    sortDir,
    setSortDir,
    page: safePage,
    setPage,
    pageSize,
    setPageSize,
    totalPages,
    paginated,
    sorted,
    filteredCount: filtered.length,
  };
}
