/* [147A-F5.9] Sub-hook extraído de useListaVentas — filtros y ordenamiento.
 * Cumple límite de 120 líneas (Regla 8). */

import { useState } from 'react';
import type { FiltrosVentas } from './ventas-filtros-types';
import { POR_PAGINA } from './ventas-filtros-types';

export function useVentasFiltros() {
  const [filtros, setFiltros] = useState<FiltrosVentas>({
    pagina: 1,
    desde: '',
    hasta: '',
    busqueda: '',
    turno: [],
    canal: [],
    metodoPago: [],
    estadoHaddock: [],
    estadoBdp: [],
    sortBy: '',
    sortOrder: 'desc',
  });

  const cambiarFiltro = <K extends keyof FiltrosVentas>(
    campo: K,
    valor: FiltrosVentas[K],
  ) => {
    setFiltros(prev => ({
      ...prev,
      [campo]: valor,
      pagina: campo === 'pagina' ? valor as number : 1,
    }));
  };

  /* [044A-8] Alterna ordenamiento por columna — click en la misma columna invierte dirección */
  const toggleSort = (columna: string) => {
    setFiltros(prev => ({
      ...prev,
      sortBy: columna,
      sortOrder: prev.sortBy === columna && prev.sortOrder === 'asc' ? 'desc' : 'asc',
      pagina: 1,
    }));
  };

  /* [064A-3] Actualizar filtro de columna (array de valores seleccionados) */
  const cambiarFiltroColumna = (
    campo: 'turno' | 'canal' | 'metodoPago' | 'estadoHaddock' | 'estadoBdp',
    valores: string[],
  ) => {
    setFiltros(prev => ({ ...prev, [campo]: valores, pagina: 1 }));
  };

  return {
    filtros,
    cambiarFiltro,
    toggleSort,
    cambiarFiltroColumna,
    porPagina: POR_PAGINA,
  };
}
