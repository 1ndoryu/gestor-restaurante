/* [D24-p] Listas reales de terminales, empleados y perfiles de artículos del
 * BDP (`GET /api/bdp/maestros`).
 *
 * La tarjeta de conexión pide esos tres números a mano y a la vez advierte que
 * no se deben inventar. No se consultan al abrir la pantalla: es una llamada
 * remota y solo se hace al pulsar, como las formas de pago. */

import { useCallback, useState } from 'react';
import axios from '@/api/axios-instance';

export interface MaestroBdp {
  id: number;
  nombre: string;
  detalle?: string;
}

export interface MaestrosBdp {
  terminales: MaestroBdp[];
  empleados: MaestroBdp[];
  perfiles: MaestroBdp[];
  /** Un fallo por categoría: una lista vacía no significa «BDP no tiene». */
  errores: string[];
}

export function useMaestrosBdp() {
  const [maestros, setMaestros] = useState<MaestrosBdp | null>(null);
  const [cargando, setCargando] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const consultar = useCallback(async () => {
    setCargando(true);
    setError(null);
    try {
      const resp = await axios.get<MaestrosBdp>('/api/bdp/maestros');
      setMaestros(resp.data);
    } catch (err: unknown) {
      const msg =
        (err as { response?: { data?: { message?: string } } })?.response?.data?.message ??
        'No se pudieron leer los maestros del BDP';
      setError(msg);
    } finally {
      setCargando(false);
    }
  }, []);

  return { maestros, cargando, error, consultar };
}
