import { useState } from 'react';
import type { ExploreType } from './constantes';

/* Estado de la consulta BDP (tipo, filtro demo y código buscado). Vive en un hook
 * para que BdpExplorador no acumule useState (usestate-excesivo). */
export function useConsultaBdp() {
  const [tipo, setTipo] = useState<ExploreType>('all');
  const [busqueda, setBusqueda] = useState('');
  const [identificador, setIdentificador] = useState('');

  return { tipo, setTipo, busqueda, setBusqueda, identificador, setIdentificador };
}
