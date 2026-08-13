/* [064A-10] Acciones por fila de venta — extraídas de ListaVentas (300 line limit).
 * Botones: ver reserva, retry Haddock, editar, eliminar.
 * [147A-F5.4] Añadido botón retry BDP.
 * [223A-1] Tooltips con TooltipButton en vez de title HTML nativo.
 * [237A-3] Añadido botón "Consultar estado BDP" por venta individual.
 * [247A-9] Diálogo de pagos parciales BDP con historial, saldo e idempotencia.
 * [128A-1/F4] Anulación local de ventas (D4) con modalidad configurable. */

import { TooltipButton } from '@/components/ui/tooltip-button';
import { Button } from '@/components/ui/button';
import { Trash2, Pencil, Eye, RefreshCw, CreditCard, ReceiptText, Search, AlertTriangle, Ban } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import instance from '@/api/axios-instance';
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
  onAnular?: (ventaId: string, motivo: string) => void;
  anulacionModalidad?: string;
  anularPending?: boolean;
  eliminarPending: boolean;
  retryPending: boolean;
  retryBdpPending?: boolean;
}

interface BdpPaymentHistoryItem {
  id: string;
  amount: string;
  tender_id: number;
  resultado: 'exito' | 'ambiguo' | 'error' | string;
  created_at: string;
}

interface BdpPaymentsResponse {
  venta_id: string;
  total: string;
  pagado: string;
  pendiente: string;
  pagos: BdpPaymentHistoryItem[];
}

function formatCurrency(value?: string | number | null): string {
  if (value == null) return '—';
  const n = Number(value);
  if (Number.isNaN(n)) return '—';
  return `${n.toFixed(2)} €`;
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
  const bdp = v as unknown as VentaConClienteBdp;
  const totalVenta = Number(v.importe_base) + Number(v.importe_iva);
  const total = totalVenta.toFixed(2);
  const queryClient = useQueryClient();
  const [accion, setAccion] = useState<'pago' | 'factura' | null>(null);
  const [tenderId, setTenderId] = useState('');
  const [importe, setImporte] = useState(total);
  const [confirmacion, setConfirmacion] = useState('');
  const [enviando, setEnviando] = useState(false);
  const [consultandoEstado, setConsultandoEstado] = useState(false);
  const [pagos, setPagos] = useState<BdpPaymentsResponse | null>(null);
  const [cargandoPagos, setCargandoPagos] = useState(false);
  /* [128A-1/F4] Anulación local */
  const [anularAbierto, setAnularAbierto] = useState(false);
  const [motivo, setMotivo] = useState('');

  const hayAmbiguo = useMemo(() => pagos?.pagos.some((p) => p.resultado === 'ambiguo') ?? false, [pagos]);
  const pendiente = useMemo(() => Number(pagos?.pendiente ?? totalVenta), [pagos, totalVenta]);
  const pagado = useMemo(() => Number(pagos?.pagado ?? 0), [pagos]);

  useEffect(() => {
    if (accion === 'pago') {
      setCargandoPagos(true);
      instance.get<BdpPaymentsResponse>(`/api/ventas/${v.id}/bdp-payments`)
        .then((r) => setPagos(r.data))
        .catch(() => toast.error('No se pudo cargar el historial de pagos BDP'))
        .finally(() => setCargandoPagos(false));
    }
  }, [accion, v.id]);

  useEffect(() => {
    if (accion === 'pago') {
      setImporte(pendiente.toFixed(2));
    }
  }, [accion, pendiente]);

  const cerrar = () => {
    setAccion(null);
    setConfirmacion('');
    setTenderId('');
    setImporte(total);
    setPagos(null);
  };

  const ejecutarBdp = async () => {
    if (!accion) return;
    const importeCanonico = Number(importe).toFixed(2);
    setEnviando(true);
    try {
      if (accion === 'pago') {
        const idempotencyKey = crypto.randomUUID();
        await instance.post(`/api/ventas/${v.id}/bdp-payment`, {
          amount: Number(importeCanonico),
          tender_id: Number(tenderId),
          confirmacion,
          idempotency_key: idempotencyKey,
          auto_arm: true,
        });
        toast.success('Pago registrado en BDP');
      } else {
        await instance.post(`/api/ventas/${v.id}/bdp-invoice`, {
          confirmacion,
          auto_arm: true,
        });
        toast.success('Factura confirmada por BDP');
      }
      cerrar();
      queryClient.invalidateQueries({ queryKey: ['listarVentas'] });
    } catch (error) {
      const message = (error as { response?: { data?: { message?: string } } }).response?.data?.message;
      toast.error('Operación BDP bloqueada', { description: message ?? 'Revisa preflight, armado y auditoría antes de reintentar.' });
    } finally {
      setEnviando(false);
    }
  };

  const puedePagar = !v.anulada && bdpSyncEnabled && bdp.bdp_synced && bdp.bdp_order_id && !bdp.bdp_invoiced && bdp.bdp_order_status !== 'cancelled' && bdp.bdp_order_status !== 'invoiced';
  const motivoObligatorio = anulacionModalidad === 'credito_completo';
  const anulacionPendienteBdp = v.anulada && bdp.bdp_synced && bdp.bdp_order_status !== 'cancelled' && bdp.bdp_order_status !== 'invoiced';

  const cerrarAnulacion = () => {
    setAnularAbierto(false);
    setMotivo('');
  };

  const ejecutarAnulacion = () => {
    if (!onAnular) return;
    onAnular(v.id, motivo.trim());
    cerrarAnulacion();
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
          tooltip="El envío a BDP es automático. Este botón solo aparece si la sincronización falló; úsalo para reintentarla."
          tooltipSide="left"
        >
          <RefreshCw className={`size-4 text-blue-600 ${retryBdpPending ? 'animate-spin' : ''}`} />
        </TooltipButton>
      )}
      {puedePagar && (
        <TooltipButton variant="ghost" size="icon" onClick={() => { setAccion('pago'); setConfirmacion(''); }} tooltip="Registrar pago en BDP" tooltipSide="left">
          <CreditCard className="size-4 text-emerald-700" />
        </TooltipButton>
      )}
      {puedePagar && (
        <TooltipButton variant="ghost" size="icon" onClick={() => { setAccion('factura'); setConfirmacion(''); }} tooltip="Facturar orden en BDP" tooltipSide="left">
          <ReceiptText className="size-4 text-violet-700" />
        </TooltipButton>
      )}
      {!v.anulada && onAnular && (
        <TooltipButton
          variant="ghost"
          size="icon"
          onClick={() => { setMotivo(''); setAnularAbierto(true); }}
          disabled={anularPending}
          tooltip="Anular venta"
          tooltipSide="left"
        >
          <Ban className="size-4 text-destructive" />
        </TooltipButton>
      )}
      <TooltipButton variant="ghost" size="icon" onClick={() => onEditar(v)} tooltip="Editar" tooltipSide="left">
        <Pencil className="size-4" />
      </TooltipButton>
      {!haddockSyncEnabled && !v.anulada && !bdp.bdp_synced && !bdp.bdp_order_id && (
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
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Registrar pago en BDP</DialogTitle>
          <DialogDescription>Permite pagar el saldo total o parcial de la comanda. Cada intento lleva una clave de idempotencia única.</DialogDescription>
        </DialogHeader>
        {cargandoPagos ? (
          <p className="text-sm text-muted-foreground">Cargando historial de pagos…</p>
        ) : (
          <>
            <div className="grid grid-cols-3 gap-2 text-sm">
              <div className="rounded border bg-muted/30 p-2 text-center">
                <div className="text-muted-foreground">Total</div>
                <div className="font-semibold">{formatCurrency(pagos?.total ?? totalVenta)}</div>
              </div>
              <div className="rounded border bg-muted/30 p-2 text-center">
                <div className="text-muted-foreground">Pagado</div>
                <div className="font-semibold text-emerald-700">{formatCurrency(pagado)}</div>
              </div>
              <div className="rounded border bg-muted/30 p-2 text-center">
                <div className="text-muted-foreground">Pendiente</div>
                <div className="font-semibold text-amber-700">{formatCurrency(pendiente)}</div>
              </div>
            </div>
            {hayAmbiguo && (
              <div className="flex items-start gap-2 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-900">
                <AlertTriangle className="mt-0.5 size-4 shrink-0" />
                <span>Existe un pago pendiente de confirmación. No se deben añadir más pagos hasta que se reconcilie.</span>
              </div>
            )}
            {pagos && pagos.pagos.length > 0 && (
              <>
                <Separator className="my-2" />
                <div className="max-h-40 overflow-auto rounded border">
                  <table className="w-full text-sm">
                    <thead className="bg-muted/50">
                      <tr>
                        <th className="p-2 text-left font-medium">Fecha</th>
                        <th className="p-2 text-right font-medium">Importe</th>
                        <th className="p-2 text-left font-medium">Estado</th>
                      </tr>
                    </thead>
                    <tbody>
                      {pagos.pagos.map((p) => (
                        <tr key={p.id} className="border-t">
                          <td className="p-2 text-xs text-muted-foreground">{new Date(p.created_at).toLocaleDateString('es-ES')}</td>
                          <td className="p-2 text-right tabular-nums">{formatCurrency(p.amount)}</td>
                          <td className="p-2">
                            {p.resultado === 'exito' && <Badge variant="default" className="bg-emerald-600">Éxito</Badge>}
                            {p.resultado === 'ambiguo' && <Badge variant="outline" className="border-amber-500 text-amber-700">Ambiguo</Badge>}
                            {p.resultado === 'error' && <Badge variant="destructive">Error</Badge>}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </>
            )}
            <div className="flex flex-col gap-3">
              <div><Label htmlFor={`importe-${v.id}`}>Importe a pagar</Label><Input id={`importe-${v.id}`} type="number" min="0.01" step="0.01" max={pendiente.toFixed(2)} value={importe} onChange={(e) => setImporte(e.target.value)} /></div>
              <div><Label htmlFor={`tender-${v.id}`}>Tender ID BDP</Label><Input id={`tender-${v.id}`} type="number" min="1" value={tenderId} onChange={(e) => setTenderId(e.target.value)} /></div>
              <div><Label htmlFor={`confirmar-pago-${v.id}`}>Escribe PAGAR {v.id} {Number(importe || 0).toFixed(2)}</Label><Input id={`confirmar-pago-${v.id}`} value={confirmacion} onChange={(e) => setConfirmacion(e.target.value)} /></div>
            </div>
          </>
        )}
        <DialogFooter>
          <Button variant="outline" onClick={cerrar}>Cancelar</Button>
          <Button
            disabled={enviando || cargandoPagos || !tenderId || Number(importe) <= 0 || Number(importe) > pendiente + 0.001 || hayAmbiguo || confirmacion !== `PAGAR ${v.id} ${Number(importe || 0).toFixed(2)}`}
            onClick={ejecutarBdp}
          >
            {enviando ? 'Verificando…' : 'Registrar pago'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
    <Dialog open={accion === 'factura'} onOpenChange={(open: boolean) => { if (!open) cerrar(); }}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader><DialogTitle>Facturar orden en BDP</DialogTitle><DialogDescription>Solo se facturará si no queda saldo pendiente.</DialogDescription></DialogHeader>
        <div className="rounded border bg-muted/30 p-3 text-sm">
          <div className="flex justify-between"><span>Total</span><span className="font-semibold">{formatCurrency(pagos?.total ?? totalVenta)}</span></div>
          <div className="flex justify-between"><span>Pendiente</span><span className={`font-semibold ${pendiente > 0.01 ? 'text-amber-700' : 'text-emerald-700'}`}>{formatCurrency(pendiente)}</span></div>
        </div>
        {pendiente > 0.01 && (
          <div className="flex items-start gap-2 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-900">
            <AlertTriangle className="mt-0.5 size-4 shrink-0" />
            <span>Queda saldo pendiente. Registra un pago por el importe restante antes de facturar.</span>
          </div>
        )}
        <div><Label htmlFor={`confirmar-factura-${v.id}`}>Escribe FACTURAR {v.id}</Label><Input id={`confirmar-factura-${v.id}`} value={confirmacion} onChange={(e) => setConfirmacion(e.target.value)} /></div>
        <DialogFooter><Button variant="outline" onClick={cerrar}>Cancelar</Button><Button disabled={enviando || pendiente > 0.01 || confirmacion !== `FACTURAR ${v.id}`} onClick={ejecutarBdp}>{enviando ? 'Verificando…' : 'Verificar y facturar'}</Button></DialogFooter>
      </DialogContent>
    </Dialog>
    <Dialog open={anularAbierto} onOpenChange={(open: boolean) => { if (!open) cerrarAnulacion(); }}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Anular venta</DialogTitle>
          <DialogDescription>
            Marca la venta como anulada. {motivoObligatorio ? 'El motivo es obligatorio y la venta se excluye del resumen diario.' : 'Solo se cambia el estado, sin exigir motivo.'}
          </DialogDescription>
        </DialogHeader>
        {motivoObligatorio && (
          <div>
            <Label htmlFor={`motivo-anulacion-${v.id}`}>Motivo de anulación</Label>
            <Textarea
              id={`motivo-anulacion-${v.id}`}
              value={motivo}
              onChange={(e) => setMotivo(e.target.value)}
              placeholder="Ej.: comanda incorrecta, cliente no recogió el pedido…"
            />
          </div>
        )}
        {anulacionPendienteBdp && (
          <div className="flex items-start gap-2 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-900 dark:border-amber-800 dark:bg-amber-950/30 dark:text-amber-200">
            <AlertTriangle className="mt-0.5 size-4 shrink-0" />
            <span>Pendiente de anular en BDP: la venta sigue abierta allí y podría volver a sincronizarse. La anulación local la excluye del resumen hasta gestionarla en BDP.</span>
          </div>
        )}
        <div><Label htmlFor={`confirmar-anular-${v.id}`}>Escribe ANULAR {v.id}</Label><Input id={`confirmar-anular-${v.id}`} value={confirmacion} onChange={(e) => setConfirmacion(e.target.value)} /></div>
        <DialogFooter>
          <Button variant="outline" onClick={cerrarAnulacion}>Cancelar</Button>
          <Button
            variant="destructive"
            disabled={anularPending || confirmacion !== `ANULAR ${v.id}` || (motivoObligatorio && motivo.trim().length === 0)}
            onClick={ejecutarAnulacion}
          >
            {anularPending ? 'Anulando…' : 'Anular venta'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
    </>
  );
}

export default VentaRowActions;
