/* [064A-10] Acciones por fila de venta — extraídas de ListaVentas (300 line limit).
 * Botones: ver reserva, retry Haddock, editar, eliminar.
 * [147A-F5.4] Añadido botón retry BDP.
 * [223A-1] Tooltips con TooltipButton en vez de title HTML nativo.
 * [237A-3] Añadido botón "Consultar estado BDP" por venta individual.
 * [247A-9] Diálogo de pagos parciales BDP con historial, saldo e idempotencia.
 * [128A-1/F4] Anulación local de ventas (D4) con modalidad configurable.
 * [128A-1/F6] Pago parcial local (A8/M13) y factura local mínima (A7/D9):
 * botones visibles cuando no aplican los de BDP y la venta no está anulada
 * ni facturada.
 * Los diálogos viven en venta-row-dialogos.tsx. */

import { Button } from '@/components/ui/button';
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import { MoreHorizontal, Trash2, Pencil, Eye, RefreshCw, CreditCard, ReceiptText, Search, Ban, Coins } from 'lucide-react';
import type { VentaConCliente } from '../api/generated';
import { useVentaRowActions } from '../hooks/useVentaRowActions';
import { VentaRowDialogos } from './venta-row-dialogos';

interface VentaRowActionsProps {
  venta: VentaConCliente;
  haddockSyncEnabled: boolean;
  bdpSyncEnabled?: boolean;
  onVerReserva: (id: string) => void;
  onEditar: (v: VentaConCliente) => void;
  onEliminar: (id: string) => void;
  onRetrySync: (id: string) => void;
  onRetryBdp?: (id: string) => void;
  onAnular?: (ventaId: string, motivo: string) => void;
  anulacionModalidad?: string;
  anularPending?: boolean;
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
  onAnular,
  anulacionModalidad = 'credito_completo',
  anularPending = false,
  eliminarPending,
  retryPending,
  retryBdpPending,
}: VentaRowActionsProps) {
  /* La lógica y el estado de las acciones viven en el hook custom para respetar
   * el límite de useState (protocolo usestate-excesivo). */
  const acciones = useVentaRowActions({ venta: v, bdpSyncEnabled, onAnular, anulacionModalidad });
  const {
    bdp,
    consultandoEstado,
    consultarEstado,
    setAccion,
    setConfirmacion,
    setPropinaImporte,
    setPropinaSumar,
    setMotivo,
    setAnularAbierto,
    puedePagar,
    puedePagoLocal,
    puedeFacturaLocal,
  } = acciones;

  return (
    <>
    {/* [VENTAS-UI] Acciones agrupadas en menú contextual de 3 puntos para
     * no saturar la fila con iconos sueltos; los diálogos se mantienen. */}
    <div className="flex items-center justify-center">
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="icon" aria-label="Acciones de la venta" className="bg-muted/40 hover:bg-muted">
          <MoreHorizontal className="size-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-60">
        {v.reserva_id && (
          <DropdownMenuItem onClick={() => onVerReserva(v.reserva_id!)}>
            <Eye className="size-4" />
            Ver reserva
          </DropdownMenuItem>
        )}
        {haddockSyncEnabled && !v.haddock_synced && v.haddock_sync_error && (
          <DropdownMenuItem onClick={() => onRetrySync(v.id)} disabled={retryPending}>
            <RefreshCw className={`size-4 text-amber-600 ${retryPending ? 'animate-spin' : ''}`} />
            Reintentar sincronización Haddock
          </DropdownMenuItem>
        )}
        {/* [237A-3] Consultar estado BDP individual */}
        {bdpSyncEnabled && bdp.bdp_synced && bdp.bdp_order_id && (
          <DropdownMenuItem
            disabled={consultandoEstado}
            onClick={consultarEstado}
          >
            <Search className={`size-4 text-blue-600 ${consultandoEstado ? 'animate-pulse' : ''}`} />
            Consultar estado BDP
          </DropdownMenuItem>
        )}
        {/* Un fallo de CreateOrder deja bdp_synced=false; el error es la señal de retry.
         * [159A-2/F2] Vocabulario Exportar: reintenta el envío local→BDP. */}
        {bdpSyncEnabled && !bdp.bdp_synced && bdp.bdp_sync_error && onRetryBdp && (
          <DropdownMenuItem onClick={() => onRetryBdp(v.id)} disabled={retryBdpPending}>
            <RefreshCw className={`size-4 text-blue-600 ${retryBdpPending ? 'animate-spin' : ''}`} />
            Exportar a BDP
          </DropdownMenuItem>
        )}
        {puedePagar && (
          <DropdownMenuItem onClick={() => { setAccion('pago'); setConfirmacion(''); }}>
            <CreditCard className="size-4 text-emerald-700" />
            Registrar pago en BDP
          </DropdownMenuItem>
        )}
        {puedePagar && (
          <DropdownMenuItem onClick={() => { setAccion('factura'); setConfirmacion(''); }}>
            <ReceiptText className="size-4 text-violet-700" />
            Facturar orden en BDP
          </DropdownMenuItem>
        )}
        {puedePagoLocal && (
          <DropdownMenuItem onClick={() => { setAccion('pagoLocal'); setConfirmacion(''); }}>
            <CreditCard className="size-4 text-emerald-700" />
            Registrar pago local
          </DropdownMenuItem>
        )}
        {puedeFacturaLocal && (
          <DropdownMenuItem onClick={() => { setAccion('facturaLocal'); setConfirmacion(''); }}>
            <ReceiptText className="size-4 text-violet-700" />
            Facturar localmente
          </DropdownMenuItem>
        )}
        {/* [198A-1/D8] Propina por venta: local siempre disponible. */}
        {!v.anulada && (
          <DropdownMenuItem onClick={() => { setAccion('propina'); setPropinaImporte(''); setPropinaSumar(true); }}>
            <Coins className="size-4 text-amber-600" />
            Añadir propina
          </DropdownMenuItem>
        )}
        {!v.anulada && onAnular && (
          <DropdownMenuItem onClick={() => { setMotivo(''); setAnularAbierto(true); }} disabled={anularPending}>
            <Ban className="size-4 text-destructive" />
            Anular venta
          </DropdownMenuItem>
        )}
        <DropdownMenuSeparator />
        <DropdownMenuItem onClick={() => onEditar(v)}>
          <Pencil className="size-4" />
          Editar
        </DropdownMenuItem>
        {!haddockSyncEnabled && !v.anulada && !bdp.bdp_synced && !bdp.bdp_order_id && (
          <DropdownMenuItem onClick={() => onEliminar(v.id)} disabled={eliminarPending}>
            <Trash2 className="size-4 text-destructive" />
            Eliminar venta
          </DropdownMenuItem>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
    </div>
    <VentaRowDialogos v={v} acciones={acciones} anularPending={anularPending} />
    </>
  );
}

export default VentaRowActions;
