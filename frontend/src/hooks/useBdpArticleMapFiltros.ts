import { useState } from 'react';

export type CampoOrden = 'codigo' | 'descripcion' | 'precio';
export type Orden = { campo: CampoOrden | null; dir: 'asc' | 'desc' };

/* [P2.4] Búsqueda, orden y página de la tabla de mapeos. Todo en cliente, sin tocar BDP.
 * El orden va en un solo estado (campo + dirección) para respetar el máximo de useState. */
export function useBdpArticleMapFiltros() {
  const [busqueda, setBusqueda] = useState('');
  const [orden, setOrden] = useState<Orden>({ campo: null, dir: 'asc' });
  const [pagina, setPagina] = useState(0);

  // Un campo nuevo empieza en ascendente; el mismo campo alterna la dirección.
  function alternarOrden(campo: CampoOrden) {
    setOrden((o) => (o.campo !== campo
      ? { campo, dir: 'asc' }
      : { campo, dir: o.dir === 'asc' ? 'desc' : 'asc' }));
    setPagina(0);
  }

  function cambiarBusqueda(v: string) {
    setBusqueda(v);
    setPagina(0);
  }

  return { busqueda, cambiarBusqueda, orden, alternarOrden, pagina, setPagina };
}
