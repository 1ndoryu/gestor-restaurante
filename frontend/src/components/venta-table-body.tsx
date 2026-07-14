/* [147A-F5.2] Cuerpo de la tabla de ventas extraído de ListaVentas (límite 300).
 * Renderiza filas con badges Haddock/BDP y acciones por fila. */

import { TableCell, TableRow } from '@/components/ui/table';
import HaddockSyncBadge from '@/components/haddock-sync-badge';
import BdpSyncBadge from '@/components/bdp-sync-badge';
import VentaRowActions from '@/components/venta-row-actions';
import type { VentaConCliente } from '../api/generated';
import type { VentaConClienteBdp } from '../api/bdp';

/* [283A-47] Mapa de etiquetas para turnos. */
const ETIQUETAS_TURNO: Record<string, string> = {
  manana: 'Mañana',
  mediodia: 'Mediodía',
  noche: 'Noche',
};

function formatearMoneda(valor: string): string {
  return new Intl.NumberFormat('es-ES', { style: 'currency', currency: 'EUR' }).format(parseFloat(valor));
}

interface VentaTableBodyProps {
  ventas: VentaConCliente[];
  haddockSyncEnabled: boolean;
  bdpSyncEnabled: boolean;
  onVerReserva: (id: string) => void;
  onEditar: (v: VentaConCliente) => void;
  onEliminar: (id: string) => void;
  onRetryHaddock: (id: string) => void;
  onRetryBdp?: (id: string) => void;
  eliminarPending: boolean;
  retryHaddockPending: boolean;
  retryBdpPending?: boolean;
}

function VentaTableBody({
  ventas,
  haddockSyncEnabled,
  bdpSyncEnabled,
  onVerReserva,
  onEditar,
  onEliminar,
  onRetryHaddock,
  onRetryBdp,
  eliminarPending,
  retryHaddockPending,
  retryBdpPending,
}: VentaTableBodyProps) {
  return (
    <>
      {ventas.map((v) => (
        <TableRow key={v.id}>
          <TableCell>{v.fecha}</TableCell>
          <TableCell>{v.nombre_cliente ?? '—'}</TableCell>
          <TableCell>{ETIQUETAS_TURNO[v.turno] ?? v.turno}</TableCell>
          <TableCell className="capitalize">{v.canal}</TableCell>
          <TableCell className="capitalize">{v.metodo_pago}</TableCell>
          <TableCell className="text-right">{formatearMoneda(v.importe_base)}</TableCell>
          <TableCell className="text-right">{formatearMoneda(v.importe_iva)}</TableCell>
          <TableCell className="text-right font-medium">{formatearMoneda((parseFloat(v.importe_base) + parseFloat(v.importe_iva)).toFixed(2))}</TableCell>
          {haddockSyncEnabled && (
            <TableCell className="text-center">
              <HaddockSyncBadge
                synced={v.haddock_synced}
                syncedAt={v.haddock_synced_at}
                syncError={v.haddock_sync_error}
              />
            </TableCell>
          )}
          {bdpSyncEnabled && (
            <TableCell className="text-center">
              <BdpSyncBadge
                synced={(v as unknown as VentaConClienteBdp).bdp_synced ?? false}
                orderStatus={(v as unknown as VentaConClienteBdp).bdp_order_status}
                syncError={(v as unknown as VentaConClienteBdp).bdp_sync_error}
                orderId={(v as unknown as VentaConClienteBdp).bdp_order_id}
              />
            </TableCell>
          )}
          <TableCell>
            <VentaRowActions
              venta={v}
              haddockSyncEnabled={haddockSyncEnabled}
              bdpSyncEnabled={bdpSyncEnabled}
              onVerReserva={onVerReserva}
              onEditar={onEditar}
              onEliminar={onEliminar}
              onRetrySync={onRetryHaddock}
              onRetryBdp={onRetryBdp}
              eliminarPending={eliminarPending}
              retryPending={retryHaddockPending}
              retryBdpPending={retryBdpPending}
            />
          </TableCell>
        </TableRow>
      ))}
    </>
  );
}

export default VentaTableBody;
