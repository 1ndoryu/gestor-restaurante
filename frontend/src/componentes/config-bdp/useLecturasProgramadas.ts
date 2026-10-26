/* [26AA-2] Cadencia y estado de la lectura programada de formas de pago y
 * empleados (`/api/bdp/lecturas-programadas`). A diferencia de los otros
 * maestros, esta cadencia se guarda al cambiarla: vive en su propia tabla, no en
 * la configuración general. */

import { useCallback, useEffect, useState } from 'react';
import axios from '@/api/axios-instance';

export interface LecturaProgramada {
  maestro: 'formas_pago' | 'empleados';
  segundos: number;
  ultima_lectura: string | null;
  ultima_ok: string | null;
  ultimo_error: string | null;
  resumen: string | null;
  activos: number;
  inactivos: number;
}

const URL_BASE = '/api/bdp/lecturas-programadas';

export function useLecturasProgramadas() {
  const [lecturas, setLecturas] = useState<LecturaProgramada[]>([]);
  const [error, setError] = useState<string | null>(null);

  const cargar = useCallback(async () => {
    try {
      const resp = await axios.get<LecturaProgramada[]>(URL_BASE);
      setLecturas(resp.data);
      setError(null);
    } catch {
      setError('No se pudo leer el estado de la lectura programada');
    }
  }, []);

  useEffect(() => {
    void cargar();
  }, [cargar]);

  const cambiar = useCallback(async (maestro: LecturaProgramada['maestro'], segundos: number) => {
    try {
      const resp = await axios.put<LecturaProgramada>(`${URL_BASE}/${maestro}`, { segundos });
      setLecturas((previas) => previas.map((l) => (l.maestro === maestro ? resp.data : l)));
      setError(null);
    } catch (err: unknown) {
      setError(
        (err as { response?: { data?: { message?: string } } })?.response?.data?.message ??
          'No se pudo guardar la cadencia',
      );
    }
  }, []);

  return { lecturas, error, cambiar, recargar: cargar };
}
