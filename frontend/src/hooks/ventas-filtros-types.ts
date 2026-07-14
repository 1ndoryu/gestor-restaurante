/* [147A-F5.9] Tipos extraídos de useListaVentas para cumplir límite de 120 líneas. */

export interface FiltrosVentas {
  pagina: number;
  desde: string;
  hasta: string;
  busqueda: string;
  turno: string[];
  canal: string[];
  metodoPago: string[];
  estadoHaddock: string[];
  estadoBdp: string[];
  sortBy: string;
  sortOrder: 'asc' | 'desc';
}

export const POR_PAGINA = 15;
