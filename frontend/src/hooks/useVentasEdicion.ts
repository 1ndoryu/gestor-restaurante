/* [147A-F5.9] Sub-hook extraído de useListaVentas — flujo de edición con confirmación.
 * Cumple límite de 120 líneas (Regla 8). */

import { useState } from 'react';
import { toast } from 'sonner';
import type { VentaConCliente } from '../api/generated';

export function useVentasEdicion(haddockSyncEnabled: boolean, bdpSyncEnabled: boolean) {
  const [ventaEditando, setVentaEditando] = useState<VentaConCliente | null>(null);
  /* [064A-9] Venta pendiente de confirmación antes de editar (cuando ya está sincronizada) */
  const [ventaPendienteEdicion, setVentaPendienteEdicion] = useState<VentaConCliente | null>(null);

  /* [064A-9] Inicia edición de venta. Si está sincronizada con Haddock,
   * muestra diálogo de confirmación antes de abrir el formulario. */
  const iniciarEdicion = (venta: VentaConCliente) => {
    if (bdpSyncEnabled && venta.bdp_synced) {
      toast.error('Edición bloqueada por seguridad BDP', {
        description: 'La comanda ya existe en BDP y WebLink no ofrece una actualización idempotente confirmada.',
      });
      return;
    }
    if (haddockSyncEnabled && venta.haddock_synced) {
      setVentaPendienteEdicion(venta);
    } else {
      setVentaEditando(venta);
    }
  };

  const confirmarEdicion = () => {
    if (ventaPendienteEdicion) {
      setVentaEditando(ventaPendienteEdicion);
      setVentaPendienteEdicion(null);
    }
  };

  const cancelarEdicion = () => {
    setVentaPendienteEdicion(null);
  };

  return {
    ventaEditando,
    setVentaEditando,
    ventaPendienteEdicion,
    iniciarEdicion,
    confirmarEdicion,
    cancelarEdicion,
  };
}
