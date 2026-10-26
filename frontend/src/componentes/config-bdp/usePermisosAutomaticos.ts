/* [26AA-3] Interruptores por categoría del modo automático
 * (`/api/bdp/permisos-automaticos`). Nacen apagados: el servidor responde
 * siempre con las siete categorías, también las que no tienen fila. */

import { useCallback, useEffect, useState } from 'react';
import axios from '@/api/axios-instance';

export interface PermisoAutomatico {
  categoria: string;
  activo: boolean;
}

const URL_BASE = '/api/bdp/permisos-automaticos';

export function usePermisosAutomaticos() {
  const [permisos, setPermisos] = useState<PermisoAutomatico[]>([]);
  const [error, setError] = useState<string | null>(null);

  const cargar = useCallback(async () => {
    try {
      const resp = await axios.get<PermisoAutomatico[]>(URL_BASE);
      setPermisos(resp.data);
      setError(null);
    } catch {
      setError('No se pudieron leer los permisos automáticos');
    }
  }, []);

  useEffect(() => {
    void cargar();
  }, [cargar]);

  // Guardar la lista completa que devuelve el servidor evita mantener dos copias del estado.
  const cambiar = useCallback(async (categoria: string, activo: boolean) => {
    try {
      const resp = await axios.put<PermisoAutomatico[]>(`${URL_BASE}/${categoria}`, { activo });
      setPermisos(resp.data);
      setError(null);
    } catch (err: unknown) {
      setError(
        (err as { response?: { data?: { message?: string } } })?.response?.data?.message ??
          'No se pudo guardar el permiso',
      );
    }
  }, []);

  return { permisos, error, cambiar };
}
