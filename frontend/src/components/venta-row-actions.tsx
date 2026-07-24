/* [064A-10] Acciones por fila de venta — extraídas de ListaVentas (300 line limit).
 * Botones: ver reserva, retry Haddock, editar, eliminar.
 * [147A-F5.4] Añadido botón retry BDP.
 * [223A-1] Tooltips con TooltipButton en vez de title HTML nativo.
 * [237A-3] Añadido botón "Consultar estado BDP" por venta individual. */

import { TooltipButton } from '@/components/ui/tooltip-button';
import { Button } from '@/components/ui/button';
import { Trash2, Pencil, Eye, RefreshCw, CreditCard, ReceiptText, Search } from 'lucide-react';
import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { customInstance } from '@/api/axios-instance';
import { toast } from 'sonner';
import type { VentaConCliente } from '../api/generated';
import type { VentaConClienteBdp } from '../api/bdp';
import { fetchBdpStatus } from '../api/bdp';

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
  const bdp = v as unknown as VentaConClienteBdp;
  const total = (Number(v.importe_base) + Number(v.importe_iva)).toFixed(2);
  const queryClient = useQueryClient();
  const [accion, setAccion] = useState<'pago' | 'factura' | null>(null);
  const [tenderId, setTenderId] = useState('');
  const [importe, setImporte] = useState(total);
  const [confirmacion, setConfirmacion] = useState('');
  const [enviando, setEnviando] = useState(false);
  const [consultandoEstado, setConsultandoEstado] = useState(false);
  const puedeLiquidar = Boolean(bdpSyncEnabled && bdp.bdp_synced && bdp.bdp_order_id && !bdp.bdp_invoiced && bdp.bdp_order_status !== 'cancelled' && bdp.bdp_order_status !== 'invoiced');

  const cerrar = () => {
    setAccion(null);
    setConfirmacion('');
    setTenderId('');
    setImporte(total);
  };

  const ejecutarBdp = async () => {
    if (!accion) return;
    const importeCanonico = Number(importe).toFixed(2);
    setEnviando(true);
    try {
      if (accion === 'pago') {
        await customInstance(`/api/ventas/${v.id}/bdp-payment`, {
          method: 'POST',
          body: JSON.stringify({
            amount: importeCanonico,
            tender_id: Number(tenderId),
            confirmacion: confirmacion,
          }),
        });
        toast.success('Pago completo confirmado por BDP');
      } else {
        await customInstance(`/api/ventas/${v.id}/bdp-invoice`, {
          method: 'POST',
          body: JSON.stringify({ confirmacion }),
        });
        toast.success('Factura confirmada por BDP');
      }
      cerrar();
    } catch (error) {
      const message = (error as { response?: { data?: { message?: string } } }).response?.data?.message;
      toast.error('Operación BDP bloqueada', { description: message ?? 'Revisa preflight, armado y auditoría antes de reintentar.' });
    } finally {
      setEnviando(false);
    }
  };

  return (
    <>
    <div className="flex gap-1">
      {v.reserva_id && (
        <TooltipButton variant="ghost" size="icon" onClick={() => onVerReserva(v.reserva_id!)} tooltip="Ver reserva" tooltipSide="left">
          <Eye className="size-4" />
        </TooltipButton>
      )}
      {haddockSyncEnabled && !v.haddock_synced && v.haddock_sync_error && (
        <TooltipButton
          variant="ghost"
          size="icon"
          onClick={() => onRetrySync(v.id)}
          disabled={retryPending}
          tooltip="Reintentar sincronización Haddock"
          tooltipSide="left"
        >
          <RefreshCw className={`size-4 text-amber-600 ${retryPending ? 'animate-spin' : ''}`} />
        </TooltipButton>
      )}
      {/* [237A-3] Botón consultar estado BDP individual */}
      {bdpSyncEnabled && bdp.bdp_synced && bdp.bdp_order_id && (
        <TooltipButton
          variant="ghost"
          size="icon"
          onClick={async () => {
            setConsultandoEstado(true);
            try {
              const status = await fetchBdpStatus(v.id);
              toast.info(`Estado BDP: ${status.bdp_order_status ?? 'desconocido'}`, {
                description: `Orden: ${status.bdp_order_id ?? '—'} · Sync: ${status.bdp_synced ? 'sí' : 'no'}${status.bdp_sync_error ? ` · Error: ${status.bdp_sync_error}` : ''}`,
              });
              queryClient.invalidateQueries({ queryKey: ['listarVentas'] });
            } catch {
              toast.error('No se pudo consultar el estado BDP');
            } finally {
              setConsultandoEstado(false);
            }
          }}
          disabled={consultandoEstado}
          tooltip="Consultar estado actual de esta comanda en BDP"
          tooltipSide="left"
        >
          <Search className={`size-4 text-blue-600 ${consultandoEstado ? 'animate-pulse' : ''}`} />
        </TooltipButton>
      )}
      {/* Un fallo de CreateOrder deja bdp_synced=false; el error es la señal de retry. */}
      {bdpSyncEnabled && !(v as unknown as VentaConClienteBdp).bdp_synced && (v as unknown as VentaConClienteBdp).bdp_sync_error && onRetryBdp && (
        <TooltipButton
          variant="ghost"
          size="icon"
          onClick={() => onRetryBdp(v.id)}
          disabled={retryBdpPending}
          tooltip="Reintentar sincronización BDP"
          tooltipSide="left"
        >
          <RefreshCw className={`size-4 text-blue-600 ${retryBdpPending ? 'animate-spin' : ''}`} />
        </TooltipButton>
      )}
      {puedeLiquidar && (
        <TooltipButton variant="ghost" size="icon" onClick={() => { setAccion('pago'); setImporte(total); setConfirmacion(''); }} tooltip="Registrar pago completo en BDP" tooltipSide="left">
          <CreditCard className="size-4 text-emerald-700" />
        </TooltipButton>
      )}
      {puedeLiquidar && (
        <TooltipButton variant="ghost" size="icon" onClick={() => { setAccion('factura'); setConfirmacion(''); }} tooltip="Facturar orden pagada en BDP" tooltipSide="left">
          <ReceiptText className="size-4 text-violet-700" />
        </TooltipButton>
      )}
      <TooltipButton variant="ghost" size="icon" onClick={() => onEditar(v)} tooltip="Editar" tooltipSide="left">
        <Pencil className="size-4" />
      </TooltipButton>
      {!haddockSyncEnabled && (
        <TooltipButton
          variant="ghost"
          size="icon"
          onClick={() => onEliminar(v.id)}
          disabled={eliminarPending}
          tooltip="Eliminar venta"
          tooltipSide="left"
        >
          <Trash2 className="size-4 text-destructive" />
        </TooltipButton>
      )}
    </div>
    <Dialog open={accion === 'pago'} onOpenChange={(open: boolean) => { if (!open) cerrar(); }}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Registrar pago completo en BDP</DialogTitle>
          <DialogDescription>El servidor releerá la orden y solo aceptará exactamente todo el saldo pendiente. Los pagos parciales están bloqueados para evitar reutilizar una intención.</DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-3">
          <div><Label htmlFor={`importe-${v.id}`}>Importe esperado</Label><Input id={`importe-${v.id}`} type="number" min="0.01" step="0.01" value={importe} onChange={(e) => setImporte(e.target.value)} /></div>
          <div><Label htmlFor={`tender-${v.id}`}>Tender ID BDP</Label><Input id={`tender-${v.id}`} type="number" min="1" value={tenderId} onChange={(e) => setTenderId(e.target.value)} /></div>
          <div><Label htmlFor={`confirmar-pago-${v.id}`}>Escribe PAGAR {v.id} {Number(importe || 0).toFixed(2)}</Label><Input id={`confirmar-pago-${v.id}`} value={confirmacion} onChange={(e) => setConfirmacion(e.target.value)} /></div>
        </div>
        <DialogFooter><Button variant="outline" onClick={cerrar}>Cancelar</Button><Button disabled={enviando || !tenderId || Number(importe) <= 0 || confirmacion !== `PAGAR ${v.id} ${Number(importe || 0).toFixed(2)}`} onClick={ejecutarBdp}>{enviando ? 'Verificando…' : 'Verificar y pagar'}</Button></DialogFooter>
      </DialogContent>
    </Dialog>
    <Dialog open={accion === 'factura'} onOpenChange={(open: boolean) => { if (!open) cerrar(); }}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader><DialogTitle>Facturar orden en BDP</DialogTitle><DialogDescription>Solo se facturará si BDP confirma que la orden no está cancelada y que no queda saldo pendiente.</DialogDescription></DialogHeader>
        <div><Label htmlFor={`confirmar-factura-${v.id}`}>Escribe FACTURAR {v.id}</Label><Input id={`confirmar-factura-${v.id}`} value={confirmacion} onChange={(e) => setConfirmacion(e.target.value)} /></div>
        <DialogFooter><Button variant="outline" onClick={cerrar}>Cancelar</Button><Button disabled={enviando || confirmacion !== `FACTURAR ${v.id}`} onClick={ejecutarBdp}>{enviando ? 'Verificando…' : 'Verificar y facturar'}</Button></DialogFooter>
      </DialogContent>
    </Dialog>
    </>
  );
}

export default VentaRowActions;
