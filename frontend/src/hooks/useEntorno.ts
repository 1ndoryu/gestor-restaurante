/* [31AA-1] Etiqueta del entorno de pruebas. El backend la lee de
 * ENTORNO_PRUEBA_ETIQUETA; en producción no existe y devuelve null. */

import { useQuery } from '@tanstack/react-query';
import { customInstance } from '@/api/axios-instance';

interface Entorno {
  etiqueta: string | null;
}

async function pedirEntorno(signal?: AbortSignal): Promise<Entorno> {
  const respuesta = await customInstance<{ data: Entorno }>('/api/entorno', { method: 'GET', signal });
  return respuesta.data;
}

export function useEntorno(): string | null {
  const { data } = useQuery({
    queryKey: ['entorno'],
    queryFn: ({ signal }) => pedirEntorno(signal),
    /* Solo cambia al reiniciar el backend: una consulta por carga basta. Si falla,
     * sin aviso (mejor callar que inventar un entorno). */
    staleTime: Infinity,
    retry: false,
  });
  return data?.etiqueta ?? null;
}
