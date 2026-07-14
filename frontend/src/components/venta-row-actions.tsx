/* [064A-10] Acciones por fila de venta — extraídas de ListaVentas (300 line limit).
 * Botones: ver reserva, retry Haddock, editar, eliminar.
 * [147A-F5.4] Añadido botón retry BDP. */

import { Button } from '@/components/ui/button';
import { Trash2, Pencil, Eye, RefreshCw } from 'lucide-react';
import type { VentaConCliente } from '../api/generated';
import type { VentaConClienteBdp } from '../api/bdp';

interface VentaRowActionsProps {
  venta: VentaConCliente;
  haddockSyncEnabled: boolean;
  bdpSyncEnabled?: boolean;
  onVerReserva: (id: string) => void;
  onEditar: (v: VentaConCliente) => void;
  onEliminar: (id: string) => void;
  onRetrySync: (id: string) => void;
  onRetryBdp?: (id: string) => void;
  eliminarPending: boolean;
  retryPending: boolean;
  retryBdpPending?: boolean;
}

function VentaRowActions({
  venta: v,
  haddockSyncEnabled,
  bdpSyncEnabled,
  onVerReserva,
  onEditar,
  onEliminar,
  onRetrySync,
  onRetryBdp,
  eliminarPending,
  retryPending,
  retryBdpPending,
}: VentaRowActionsProps) {
  return (
    <div className="flex gap-1">
      {v.reserva_id && (
        <Button variant="ghost" size="icon" onClick={() => onVerReserva(v.reserva_id!)} title="Ver reserva">
          <Eye className="size-4" />
        </Button>
      )}
      {haddockSyncEnabled && !v.haddock_synced && v.haddock_sync_error && (
        <Button
          variant="ghost"
          size="icon"
          onClick={() => onRetrySync(v.id)}
          disabled={retryPending}
          title="Reintentar sincronización Haddock"
        >
          <RefreshCw className={`size-4 text-amber-600 ${retryPending ? 'animate-spin' : ''}`} />
        </Button>
      )}
      {/* [147A-F5.4] Retry BDP — solo si la venta fue enviada a BDP y tiene error */}
      {bdpSyncEnabled && (v as unknown as VentaConClienteBdp).bdp_synced && (v as unknown as VentaConClienteBdp).bdp_sync_error && onRetryBdp && (
        <Button
          variant="ghost"
          size="icon"
          onClick={() => onRetryBdp(v.id)}
          disabled={retryBdpPending}
          title="Reintentar sincronización BDP"
        >
          <RefreshCw className={`size-4 text-blue-600 ${retryBdpPending ? 'animate-spin' : ''}`} />
        </Button>
      )}
      <Button variant="ghost" size="icon" onClick={() => onEditar(v)} title="Editar">
        <Pencil className="size-4" />
      </Button>
      {!haddockSyncEnabled && (
        <Button
          variant="ghost"
          size="icon"
          onClick={() => onEliminar(v.id)}
          disabled={eliminarPending}
          title="Eliminar"
        >
          <Trash2 className="size-4 text-destructive" />
        </Button>
      )}
    </div>
  );
}

export default VentaRowActions;
