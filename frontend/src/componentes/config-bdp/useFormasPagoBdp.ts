/* [D24-m] Lectura bajo demanda de las formas de pago reales del BDP
 * (`GET /api/bdp/tenders` → `Tenders/GetList`). No se consulta al abrir la
 * pantalla: es una llamada remota, y el mapeo se revisa cuando hace falta. */

import { useCallback, useState } from 'react';
import axios from '@/api/axios-instance';

export interface FormaPagoBdp {
  id: number;
  nombre: string;
  tipo: number | null;
}

export function useFormasPagoBdp() {
  const [formas, setFormas] = useState<FormaPagoBdp[] | null>(null);
  const [cargando, setCargando] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const consultar = useCallback(async () => {
    setCargando(true);
    setError(null);
    try {
      const resp = await axios.get<FormaPagoBdp[]>('/api/bdp/tenders');
      setFormas(resp.data);
    } catch (err: unknown) {
      const msg =
        (err as { response?: { data?: { message?: string } } })?.response?.data?.message ??
        'No se pudo leer las formas de pago del BDP';
      setError(msg);
    } finally {
      setCargando(false);
    }
  }, []);

  return { formas, cargando, error, consultar };
}
