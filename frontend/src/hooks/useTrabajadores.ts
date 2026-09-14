/* [134A-2] Hook para gestión de trabajadores con permisos por sección.
 * CRUD completo + toggle de permisos individuales.
 * Los permisos se obtienen de listarSecciones (backend define las secciones disponibles). */

import { useState } from 'react';
import { toast } from 'sonner';
import {
  useListar,
  useCrear,
  useActualizar,
  useEliminar,
  useListarSecciones,
} from '../api/generated/trabajadores/trabajadores';
import type {
  TrabajadorResponse,
  CrearTrabajadorRequest,
  ActualizarTrabajadorRequest,
} from '../api/generated/gestionRestauranteAPI.schemas';

function extraerError(err: unknown): string {
  const e = err as { response?: { data?: { message?: string } } };
  return e?.response?.data?.message || 'Error inesperado';
}

export function useTrabajadores() {
  const [modalCrear, setModalCrear] = useState(false);
  const [trabajadorEditar, setTrabajadorEditar] = useState<TrabajadorResponse | null>(null);

  /* [149A-3/F2] isError/error se exponen para que la página pueda decir
   * "sin permiso" en vez de quedarse en "Cargando..." o pintar 0 trabajadores. */
  const { data, isLoading, isError, error, refetch } = useListar({
    query: { queryKey: ['trabajadores'] },
  });

  const { data: seccionesData, isError: seccionesError } = useListarSecciones({
    query: { queryKey: ['trabajadores-secciones'] },
  });

  const crearMut = useCrear({
    mutation: {
      onSuccess: () => {
        toast.success('Trabajador creado');
        refetch();
        setModalCrear(false);
      },
      onError: (err: unknown) => toast.error(extraerError(err)),
    },
  });

  const actualizarMut = useActualizar({
    mutation: {
      onSuccess: () => {
        toast.success('Trabajador actualizado');
        refetch();
        setTrabajadorEditar(null);
      },
      onError: (err: unknown) => toast.error(extraerError(err)),
    },
  });

  const eliminarMut = useEliminar({
    mutation: {
      onSuccess: () => {
        toast.success('Trabajador eliminado');
        refetch();
      },
      onError: (err: unknown) => toast.error(extraerError(err)),
    },
  });

  const trabajadores: TrabajadorResponse[] = data?.data ?? [];
  const secciones: string[] = seccionesData?.data ?? [];

  return {
    trabajadores,
    secciones,
    isLoading,
    isError,
    error,
    seccionesError,
    modalCrear,
    setModalCrear,
    trabajadorEditar,
    setTrabajadorEditar,
    crearTrabajador: (req: CrearTrabajadorRequest) => crearMut.mutate({ data: req }),
    actualizarTrabajador: (id: string, req: ActualizarTrabajadorRequest) =>
      actualizarMut.mutate({ id, data: req }),
    eliminarTrabajador: (id: string) => eliminarMut.mutate({ id }),
    creando: crearMut.isPending,
    actualizando: actualizarMut.isPending,
    refetch,
  };
}
