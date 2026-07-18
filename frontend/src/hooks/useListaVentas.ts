/* [283A-28] Hook para ListaVentas — orquesta sub-hooks y datos API.
 * [147A-F5.9] Refactorizado: filtros a useVentasFiltros, edición a useVentasEdicion.
 * Cumple límite de 120 líneas (Regla 8). */

import { useState } from 'react';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { useListarVentas, useEliminarVenta, useObtenerReserva, useReintentarSyncHaddock } from '../api/generated';
import { useObtenerConfiguracion } from '../api/generated/configuracion/configuracion';
import { useBdpPoll, useRetryBdpSync } from '../api/bdp';
import { useVentasFiltros } from './useVentasFiltros';
import { useVentasEdicion } from './useVentasEdicion';

function useListaVentas() {
  const { filtros, cambiarFiltro, toggleSort, cambiarFiltroColumna, porPagina } = useVentasFiltros();

  const { data, isLoading, refetch } = useListarVentas({
    page: filtros.pagina,
    per_page: porPagina,
    desde: filtros.desde || undefined,
    hasta: filtros.hasta || undefined,
    busqueda: filtros.busqueda || undefined,
    turno: filtros.turno.length > 0 ? filtros.turno.join(',') : undefined,
    canal: filtros.canal.length > 0 ? filtros.canal.join(',') : undefined,
    metodo_pago: filtros.metodoPago.length > 0 ? filtros.metodoPago.join(',') : undefined,
    estado_haddock: filtros.estadoHaddock.length > 0 ? filtros.estadoHaddock.join(',') : undefined,
    estado_bdp: filtros.estadoBdp.length > 0 ? filtros.estadoBdp.join(',') : undefined,
    sort_by: filtros.sortBy || undefined,
    sort_order: filtros.sortOrder || undefined,
  });

  const eliminarMutation = useEliminarVenta({
    mutation: {
      onSuccess: () => { refetch(); },
      onError: (err: unknown) => {
        const status = (err as { status?: number })?.status
          ?? (err as { response?: { status?: number } })?.response?.status;
        if (status === 409) {
          toast.error('Eliminación bloqueada', {
            description: 'La sincronización con Haddock está activa. Desactívela en Configuración para poder eliminar ventas.',
          });
        } else {
          toast.error('Error al eliminar la venta');
        }
      },
    },
  });

  /* [064A-8] Config: sync Haddock y BDP habilitados */
  const { data: configData } = useObtenerConfiguracion();
  const haddockSyncEnabled = configData?.status === 200
    ? configData.data.haddock_sync_enabled
    : false;
  const bdpSyncEnabled = configData?.status === 200
    ? Boolean((configData.data as unknown as Record<string, unknown>).bdp_sync_enabled ?? false)
    : false;

  /* Sub-hook de edición — depende de haddockSyncEnabled */
  const edicion = useVentasEdicion(haddockSyncEnabled, bdpSyncEnabled);

  const retryHaddockMutation = useReintentarSyncHaddock({
    mutation: {
      onSuccess: () => { toast.success('Sincronización Haddock completada'); refetch(); },
      onError: () => { toast.error('Error al reintentar sincronización Haddock'); refetch(); },
    },
  });

  const queryClient = useQueryClient();
  const bdpPollMutation = useBdpPoll(queryClient);
  const retryBdpMutation = useRetryBdpSync(queryClient);

  const ventas = data?.status === 200 ? data.data : null;

  /* [034A-5] Reserva asociada (solo cuando se abre el viewer) */
  const [reservaIdViewer, setReservaIdViewer] = useState<string | null>(null);
  const { data: reservaData, isLoading: reservaCargando } = useObtenerReserva(reservaIdViewer ?? '', {
    query: { enabled: !!reservaIdViewer },
  });
  const reservaDetalle = reservaData?.status === 200 ? reservaData.data : null;

  const [modalAbierto, setModalAbierto] = useState(false);
  const cerrarModalYRefrescar = () => { setModalAbierto(false); refetch(); };
  const cerrarEdicionYRefrescar = () => { edicion.setVentaEditando(null); refetch(); };

  return {
    filtros,
    cambiarFiltro,
    toggleSort,
    cambiarFiltroColumna,
    modalAbierto,
    setModalAbierto,
    ...edicion,
    porPagina,
    ventas,
    isLoading,
    eliminarMutation,
    haddockSyncEnabled,
    retryHaddockMutation,
    bdpSyncEnabled,
    bdpPollMutation,
    retryBdpMutation,
    cerrarModalYRefrescar,
    cerrarEdicionYRefrescar,
    reservaIdViewer,
    setReservaIdViewer,
    reservaDetalle,
    reservaCargando,
  };
}

export default useListaVentas;
