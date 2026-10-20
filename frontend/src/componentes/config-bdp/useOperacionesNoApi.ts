/* [D32] Estado y llamadas de las operaciones que BDP no expone por API. Vive en un
 * hook para que la tarjeta se quede en pintar: cinco `useState` y cuatro llamadas
 * dentro del componente disparaban el hallazgo `usestate-excesivo`. */

import { useCallback, useEffect, useState } from 'react';
import axios from '@/api/axios-instance';
import { toast } from 'sonner';

/** Una fila de `GET /api/bdp/operaciones-no-api`. */
export interface OperacionNoApi {
  codigo: string;
  operacion: string;
  responsable: string;
  procedimiento: string;
  /** true si ni la operación ni el responsable global tienen a nadie asignado. */
  responsable_pendiente: boolean;
  mensaje: string;
}

/** Borrador editable de una fila; vacío significa «usar el valor por defecto». */
export interface Borrador {
  responsable: string;
  procedimiento: string;
}

/** Marca del guardado del responsable global, que no es una operación con código. */
const CLAVE_GLOBAL = '__global__';

function mensajeDeError(error: unknown, porDefecto: string): string {
  return (
    (error as { response?: { data?: { message?: string } } })?.response?.data?.message ?? porDefecto
  );
}

export function useOperacionesNoApi() {
  const [operaciones, setOperaciones] = useState<OperacionNoApi[]>([]);
  const [borradores, setBorradores] = useState<Record<string, Borrador>>({});
  const [global, setGlobal] = useState('');
  const [cargando, setCargando] = useState(true);
  const [guardando, setGuardando] = useState<string | null>(null);

  const cargar = useCallback(async () => {
    try {
      /* La configuración trae el responsable global; la lista, el de cada operación. */
      const [config, listado] = await Promise.all([
        axios.get<{ bdp_responsable_no_api: string }>('/api/configuracion'),
        axios.get<OperacionNoApi[]>('/api/bdp/operaciones-no-api'),
      ]);
      setGlobal(config.data.bdp_responsable_no_api ?? '');
      setOperaciones(listado.data);
      setBorradores(
        Object.fromEntries(
          listado.data.map((fila) => [
            fila.codigo,
            { responsable: fila.responsable, procedimiento: fila.procedimiento },
          ]),
        ),
      );
    } catch (error) {
      toast.error('No se pudieron leer las operaciones no API', {
        description: mensajeDeError(error, 'Error de red'),
      });
    } finally {
      setCargando(false);
    }
  }, []);

  useEffect(() => {
    void cargar();
  }, [cargar]);

  function editar(codigo: string, campo: keyof Borrador, valor: string) {
    setBorradores((actual) => ({
      ...actual,
      [codigo]: { ...actual[codigo], [campo]: valor },
    }));
  }

  /** Descarta el borrador de una fila y vuelve a los valores guardados. */
  function deshacer(fila: OperacionNoApi) {
    setBorradores((actual) => ({
      ...actual,
      [fila.codigo]: { responsable: fila.responsable, procedimiento: fila.procedimiento },
    }));
  }

  /** Guarda una operación. Devuelve la fila ya resuelta, que es lo que pinta la lista. */
  async function guardar(codigo: string) {
    setGuardando(codigo);
    try {
      const respuesta = await axios.put<OperacionNoApi>(
        `/api/bdp/operaciones-no-api/${codigo}`,
        borradores[codigo] ?? { responsable: '', procedimiento: '' },
      );
      setOperaciones((actual) =>
        actual.map((fila) => (fila.codigo === codigo ? respuesta.data : fila)),
      );
      toast.success('Operación guardada');
    } catch (error) {
      toast.error('No se pudo guardar', { description: mensajeDeError(error, 'Error de red') });
    } finally {
      setGuardando(null);
    }
  }

  /* El responsable global se guarda por su cuenta (PATCH de configuración): es un
   * respaldo para todas las operaciones, no parte del formulario de conexión. */
  async function guardarGlobal() {
    setGuardando(CLAVE_GLOBAL);
    try {
      await axios.patch('/api/configuracion', { bdp_responsable_no_api: global });
      await cargar();
      toast.success('Responsable por defecto guardado');
    } catch (error) {
      toast.error('No se pudo guardar el responsable', {
        description: mensajeDeError(error, 'Error de red'),
      });
    } finally {
      setGuardando(null);
    }
  }

  const pendientes = operaciones.filter((fila) => fila.responsable_pendiente).length;

  return {
    operaciones,
    borradores,
    global,
    setGlobal,
    cargando,
    guardando,
    guardarGlobalSeleccionado: guardando === CLAVE_GLOBAL,
    pendientes,
    editar,
    deshacer,
    guardar,
    guardarGlobal,
  };
}
