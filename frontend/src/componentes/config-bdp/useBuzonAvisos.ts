/* [L9] Estado y llamadas del buzón de avisos del TPV (BDP → Glory).
 *
 * El buzón ya existía en el backend y no tenía interfaz: si el TPV mandaba un
 * aviso y su procesado fallaba, nadie podía verlo ni reintentarlo desde la web.
 * El estado vive aquí para que la tarjeta solo pinte. */

import { useCallback, useEffect, useState } from 'react';
import axios from '@/api/axios-instance';
import { toast } from 'sonner';

/** Una fila de `GET /api/bdp/avisos`. */
export interface Aviso {
  id: string;
  accion: string;
  origen: string | null;
  clave_idempotencia: string;
  payload: unknown;
  estado: string;
  intentos: number;
  ultimo_error: string | null;
  recibido_en: string;
  procesado_en: string | null;
}

/** Lo que devuelve `POST /api/bdp/avisos/procesar`. */
export interface ResumenLote {
  procesados: number;
  reintentos: number;
  revision: number;
}

/** Los estados que admite el buzón, con su etiqueta. El orden es el del ciclo:
 * llega, se procesa, y o queda bien o queda para una persona. */
export const ESTADOS_AVISO: { valor: string; etiqueta: string }[] = [
  { valor: '', etiqueta: 'Todos' },
  { valor: 'recibido', etiqueta: 'Sin procesar' },
  { valor: 'procesando', etiqueta: 'En curso' },
  { valor: 'procesado', etiqueta: 'Procesados' },
  { valor: 'requiere_revision', etiqueta: 'Requieren revisión' },
  { valor: 'rechazado', etiqueta: 'Rechazados' },
];

interface EstadoBuzon {
  avisos: Aviso[];
  cargando: boolean;
  procesando: boolean;
  /** Id del aviso que se está reabriendo, para no disparar dos a la vez. */
  reintentando: string | null;
  resumen: ResumenLote | null;
}

function mensajeDeError(error: unknown, porDefecto: string): string {
  return (
    (error as { response?: { data?: { message?: string } } })?.response?.data?.message ?? porDefecto
  );
}

export function useBuzonAvisos() {
  const [filtro, setFiltro] = useState('');
  const [buzon, setBuzon] = useState<EstadoBuzon>({
    avisos: [],
    cargando: true,
    procesando: false,
    reintentando: null,
    resumen: null,
  });

  const cargar = useCallback(async (estado: string) => {
    setBuzon((previo) => ({ ...previo, cargando: true }));
    try {
      const resp = await axios.get<Aviso[]>('/api/bdp/avisos', {
        /* Sin estado, el backend devuelve los cincuenta últimos de cualquier estado. */
        params: estado ? { estado, limite: 50 } : { limite: 50 },
      });
      setBuzon((previo) => ({ ...previo, avisos: resp.data }));
    } catch (error) {
      toast.error('No se pudo leer el buzón de avisos', {
        description: mensajeDeError(error, 'Revisa la conexión con el servidor.'),
      });
      setBuzon((previo) => ({ ...previo, avisos: [] }));
    } finally {
      setBuzon((previo) => ({ ...previo, cargando: false }));
    }
  }, []);

  useEffect(() => {
    void cargar(filtro);
  }, [cargar, filtro]);

  /** Procesa un lote: solo lee de BDP (`GetOrder`), no escribe en el TPV. */
  const procesar = useCallback(async () => {
    setBuzon((previo) => ({ ...previo, procesando: true, resumen: null }));
    try {
      const resp = await axios.post<ResumenLote>('/api/bdp/avisos/procesar', null, {
        params: { limite: 50 },
      });
      setBuzon((previo) => ({ ...previo, resumen: resp.data }));
      toast.success('Lote procesado', {
        description: `${resp.data.procesados} procesados, ${resp.data.reintentos} para reintentar, ${resp.data.revision} para revisar.`,
      });
      await cargar(filtro);
    } catch (error) {
      toast.error('No se pudo procesar el lote', {
        description: mensajeDeError(error, 'Revisa la configuración de BDP.'),
      });
    } finally {
      setBuzon((previo) => ({ ...previo, procesando: false }));
    }
  }, [cargar, filtro]);

  /** Devuelve un aviso a `recibido` para que vuelva a procesarse. */
  const reintentar = useCallback(
    async (id: string) => {
      setBuzon((previo) => ({ ...previo, reintentando: id }));
      try {
        await axios.post(`/api/bdp/avisos/${id}/reintentar`);
        toast.success('Aviso reabierto', { description: 'Volverá a procesarse en el próximo lote.' });
        await cargar(filtro);
      } catch (error) {
        toast.error('No se pudo reabrir el aviso', {
          description: mensajeDeError(error, 'Solo se reabren los que están en revisión.'),
        });
      } finally {
        setBuzon((previo) => ({ ...previo, reintentando: null }));
      }
    },
    [cargar, filtro],
  );

  return { filtro, setFiltro, ...buzon, cargar, procesar, reintentar };
}
