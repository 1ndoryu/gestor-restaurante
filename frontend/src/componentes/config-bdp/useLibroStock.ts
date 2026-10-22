/* [D26g] Estado y llamadas de la revisión del libro de stock.
 *
 * Cuando BDP pierde la respuesta de un ajuste, se relee `GetStock`: si el valor
 * no es ni S ni S+V, la operación queda ambigua y el guard de escritura bloquea
 * ese artículo hasta que una persona diga qué pasó. El backend ya no deja esa
 * revisión sin salida; esta pieza es su pantalla. El estado vive aquí para que la
 * tarjeta solo pinte. */

import { useCallback, useEffect, useState } from 'react';
import axios from '@/api/axios-instance';
import { toast } from 'sonner';

/** Una fila de `GET /api/bdp/stock/revisiones`. */
export interface RevisionStock {
  id_local: string;
  articulo_codigo: string;
  almacen: number;
  operacion: string;
  variacion: number;
  stock_previo: number | null;
  stock_esperado: number | null;
  stock_posterior: number | null;
  nota: string | null;
  documento_id: string | null;
  creado_en: string;
  /** Estado de la fila de la cola que originó la operación. */
  estado_cola: string | null;
}

/** Qué ha visto la persona en el BDP. */
export type DecisionStock = 'aplicado' | 'no_aplicado';

/** Lo que devuelve `POST /api/bdp/stock/revisiones/:id`. */
export interface ResolucionStock {
  id_local: string;
  aplicado: boolean;
  estado_cola: string;
}

interface EstadoLibro {
  filas: RevisionStock[];
  cargando: boolean;
  /** Id en curso, para no disparar dos decisiones a la vez. */
  resolviendo: string | null;
}

function mensajeDeError(error: unknown, porDefecto: string): string {
  return (
    (error as { response?: { data?: { message?: string } } })?.response?.data?.message ?? porDefecto
  );
}

export function useLibroStock() {
  const [libro, setLibro] = useState<EstadoLibro>({
    filas: [],
    cargando: true,
    resolviendo: null,
  });

  const cargar = useCallback(async () => {
    setLibro((previo) => ({ ...previo, cargando: true }));
    try {
      const resp = await axios.get<RevisionStock[]>('/api/bdp/stock/revisiones');
      setLibro((previo) => ({ ...previo, filas: resp.data }));
    } catch (error) {
      toast.error('No se pudo leer el libro de stock', {
        description: mensajeDeError(error, 'Revisa la conexión con el servidor.'),
      });
      setLibro((previo) => ({ ...previo, filas: [] }));
    } finally {
      setLibro((previo) => ({ ...previo, cargando: false }));
    }
  }, []);

  useEffect(() => {
    void cargar();
  }, [cargar]);

  /** Registra el dictamen. `aplicado` cierra la operación sin reenviarla;
   * `no_aplicado` la devuelve a la cola con los reintentos a cero. */
  const resolver = useCallback(
    async (idLocal: string, decision: DecisionStock, motivo: string) => {
      setLibro((previo) => ({ ...previo, resolviendo: idLocal }));
      try {
        const resp = await axios.post<{ ok: boolean; resolucion: ResolucionStock }>(
          `/api/bdp/stock/revisiones/${encodeURIComponent(idLocal)}`,
          { decision, motivo },
        );
        const resuelta = resp.data.resolucion;
        toast.success(decision === 'aplicado' ? 'Operación cerrada' : 'Operación reencolada', {
          description:
            decision === 'aplicado'
              ? 'El stock del BDP ya consta: no se reenvía nada.'
              : 'Se enviará en el próximo sincronizado con los reintentos a cero.',
        });
        /* Se quita de la lista sin recargar: la respuesta ya trae el estado. */
        setLibro((previo) => ({
          ...previo,
          filas: previo.filas.filter((fila) => fila.id_local !== resuelta.id_local),
        }));
      } catch (error) {
        toast.error('No se pudo registrar la revisión', {
          description: mensajeDeError(error, 'El motivo debe tener al menos 5 caracteres.'),
        });
      } finally {
        setLibro((previo) => ({ ...previo, resolviendo: null }));
      }
    },
    [],
  );

  return { ...libro, cargar, resolver };
}

/** [D-auto-push] Una auditoría de stock bloqueada: el envío falló antes de
 * abrir el libro, así que no hay relectura que la cierre y ese artículo y
 * almacén no admiten otro movimiento hasta que alguien la descarte. */
export interface AuditoriaBloqueada {
  id: string;
  operacion: string;
  resultado: string;
  error_mensaje: string | null;
  creada_en: string;
}

export function useAuditoriasBloqueadas() {
  const [estado, setEstado] = useState<{
    filas: AuditoriaBloqueada[];
    cargando: boolean;
    descartando: string | null;
  }>({ filas: [], cargando: true, descartando: null });

  const cargar = useCallback(async () => {
    setEstado((previo) => ({ ...previo, cargando: true }));
    try {
      const resp = await axios.get<AuditoriaBloqueada[]>('/api/bdp/stock/auditorias-bloqueadas');
      setEstado((previo) => ({ ...previo, filas: resp.data }));
    } catch (error) {
      toast.error('No se pudieron leer las auditorías bloqueadas', {
        description: mensajeDeError(error, 'Revisa la conexión con el servidor.'),
      });
      setEstado((previo) => ({ ...previo, filas: [] }));
    } finally {
      setEstado((previo) => ({ ...previo, cargando: false }));
    }
  }, []);

  useEffect(() => {
    void cargar();
  }, [cargar]);

  /** El motivo es obligatorio: sin él nadie sabrá por qué se destrabó. */
  const descartar = useCallback(async (id: string, motivo: string) => {
    setEstado((previo) => ({ ...previo, descartando: id }));
    try {
      await axios.post(`/api/bdp/stock/auditorias-bloqueadas/${id}`, { motivo });
      toast.success('Auditoría descartada', {
        description: 'Ese artículo y almacén vuelven a admitir movimientos.',
      });
      setEstado((previo) => ({
        ...previo,
        filas: previo.filas.filter((fila) => fila.id !== id),
      }));
    } catch (error) {
      toast.error('No se pudo descartar', {
        description: mensajeDeError(error, 'El motivo debe tener al menos 5 caracteres.'),
      });
    } finally {
      setEstado((previo) => ({ ...previo, descartando: null }));
    }
  }, []);

  return { ...estado, cargar, descartar };
}
