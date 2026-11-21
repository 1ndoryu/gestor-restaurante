/* [26AA-3] Interruptores por categoría del modo automático
 * (`/api/bdp/permisos-automaticos`). Nacen apagados: el servidor responde
 * siempre con todas las categorías, también las que no tienen fila.
 * [21AB-5] Añade la vuelta rápida a solo lectura y la actividad reciente. */

import { useCallback, useEffect, useState } from 'react';
import axios from '@/api/axios-instance';

export interface PermisoAutomatico {
  categoria: string;
  activo: boolean;
}

export interface CambioPermiso {
  categoria: string;
  activo: boolean;
  cambiado_en: string;
}

const URL_BASE = '/api/bdp/permisos-automaticos';

export function usePermisosAutomaticos() {
  const [permisos, setPermisos] = useState<PermisoAutomatico[]>([]);
  const [actividad, setActividad] = useState<CambioPermiso[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [apagando, setApagando] = useState(false);

  const cargarActividad = useCallback(async () => {
    try {
      const resp = await axios.get<CambioPermiso[]>(`${URL_BASE}/actividad`);
      setActividad(resp.data);
    } catch {
      // La actividad es informativa: si falla no se rompe la pantalla de permisos.
      setActividad([]);
    }
  }, []);

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
    void cargarActividad();
  }, [cargar, cargarActividad]);

  // Guardar la lista completa que devuelve el servidor evita mantener dos copias del estado.
  const cambiar = useCallback(
    async (categoria: string, activo: boolean) => {
      try {
        const resp = await axios.put<PermisoAutomatico[]>(`${URL_BASE}/${categoria}`, { activo });
        setPermisos(resp.data);
        setError(null);
        void cargarActividad();
      } catch (err: unknown) {
        setError(
          (err as { response?: { data?: { message?: string } } })?.response?.data?.message ??
            'No se pudo guardar el permiso',
        );
      }
    },
    [cargarActividad],
  );

  /* [21AB-5] El botón de emergencia: apaga todas las etapas de golpe. */
  const volverASoloLectura = useCallback(async () => {
    setApagando(true);
    try {
      const resp = await axios.post<PermisoAutomatico[]>(`${URL_BASE}/solo-lectura`);
      setPermisos(resp.data);
      setError(null);
      void cargarActividad();
    } catch (err: unknown) {
      setError(
        (err as { response?: { data?: { message?: string } } })?.response?.data?.message ??
          'No se pudo volver a solo lectura',
      );
    } finally {
      setApagando(false);
    }
  }, [cargarActividad]);

  return { permisos, actividad, error, apagando, cambiar, volverASoloLectura };
}
