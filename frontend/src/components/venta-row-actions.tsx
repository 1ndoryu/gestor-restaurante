/* [064A-10] Acciones por fila de venta — extraídas de ListaVentas (300 line limit).
 * Botones: ver reserva, retry Haddock, editar, eliminar.
 * [147A-F5.4] Añadido botón retry BDP.
 * [223A-1] Tooltips con TooltipButton en vez de title HTML nativo.
 * [237A-3] Añadido botón "Consultar estado BDP" por venta individual.
 * [247A-9] Diálogo de pagos parciales BDP con historial, saldo e idempotencia.
 * [128A-1/F4] Anulación local de ventas (D4) con modalidad configurable.
 * [128A-1/F6] Pago parcial local (A8/M13) y factura local mínima (A7/D9):
 * botones visibles cuando no aplican los de BDP y la venta no está anulada
 * ni facturada. */

import { Button } from '@/components/ui/button';
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import { MoreHorizontal, Trash2, Pencil, Eye, RefreshCw, CreditCard, ReceiptText, Search, AlertTriangle, Ban, Coins } from 'lucide-react';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Switch } from '@/components/ui/switch';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import type { VentaConCliente } from '../api/generated';
import { useVentaRowActions } from '../hooks/useVentaRowActions';

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
  /* La lógica y el estado de las acciones viven en el hook custom para respetar
   * el límite de useState (protocolo usestate-excesivo). */
  const {
    bdp,
    totalVenta,
    accion,
    setAccion,
    tenderId,
    setTenderId,
    importe,
    setImporte,
    confirmacion,
    setConfirmacion,
    enviando,
    consultandoEstado,
    consultarEstado,
    pagos,
    cargandoPagos,
    anularAbierto,
    setAnularAbierto,
    motivo,
    setMotivo,
    propinaImporte,
    setPropinaImporte,
    propinaSumar,
    setPropinaSumar,
    hayAmbiguo,
    pendiente,
    pagado,
    cerrar,
    ejecutarBdp,
    ejecutarPropina,
    ejecutarLocal,
    puedePagar,
    puedePagoLocal,
    puedeFacturaLocal,
    motivoObligatorio,
    anulacionPendienteBdp,
    cerrarAnulacion,
    ejecutarAnulacion,
  } = useVentaRowActions({ venta: v, bdpSyncEnabled, onAnular, anulacionModalidad });

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
        {/* Un fallo de CreateOrder deja bdp_synced=false; el error es la señal de retry. */}
        {bdpSyncEnabled && !bdp.bdp_synced && bdp.bdp_sync_error && onRetryBdp && (
          <DropdownMenuItem onClick={() => onRetryBdp(v.id)} disabled={retryBdpPending}>
            <RefreshCw className={`size-4 text-blue-600 ${retryBdpPending ? 'animate-spin' : ''}`} />
            Reintentar envío a BDP
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
                  <table className="w-full text-[13px]">
                    <thead className="bg-muted/50">
                      <tr>
                        <th className="p-1.5 text-left font-medium">Fecha</th>
                        <th className="p-1.5 text-right font-medium">Importe</th>
                        <th className="p-1.5 text-left font-medium">Estado</th>
                      </tr>
                    </thead>
                    <tbody>
                      {pagos.pagos.map((p) => (
                        <tr key={p.id} className="border-t">
                          <td className="p-1.5 text-xs text-muted-foreground">{new Date(p.created_at).toLocaleDateString('es-ES')}</td>
                          <td className="p-1.5 text-right tabular-nums">{formatCurrency(p.amount)}</td>
                          <td className="p-1.5">
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
    <Dialog open={accion === 'pagoLocal'} onOpenChange={(open: boolean) => { if (!open) cerrar(); }}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Registrar pago local</DialogTitle>
          <DialogDescription>Permite cobrar el saldo total o parcial de la venta sin depender del BDP. Cada intento lleva una clave de idempotencia única.</DialogDescription>
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
                  <table className="w-full text-[13px]">
                    <thead className="bg-muted/50">
                      <tr>
                        <th className="p-1.5 text-left font-medium">Fecha</th>
                        <th className="p-1.5 text-right font-medium">Importe</th>
                        <th className="p-1.5 text-left font-medium">Estado</th>
                      </tr>
                    </thead>
                    <tbody>
                      {pagos.pagos.map((p) => (
                        <tr key={p.id} className="border-t">
                          <td className="p-1.5 text-xs text-muted-foreground">{new Date(p.created_at).toLocaleDateString('es-ES')}</td>
                          <td className="p-1.5 text-right tabular-nums">{formatCurrency(p.amount)}</td>
                          <td className="p-1.5">
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
              <div><Label htmlFor={`importe-local-${v.id}`}>Importe a cobrar</Label><Input id={`importe-local-${v.id}`} type="number" min="0.01" step="0.01" max={pendiente.toFixed(2)} value={importe} onChange={(e) => setImporte(e.target.value)} /></div>
              <div><Label htmlFor={`tender-local-${v.id}`}>Tender ID</Label><Input id={`tender-local-${v.id}`} type="number" min="1" value={tenderId} onChange={(e) => setTenderId(e.target.value)} /></div>
              <div><Label htmlFor={`confirmar-pago-local-${v.id}`}>Escribe PAGO LOCAL {v.id} {Number(importe || 0).toFixed(2)}</Label><Input id={`confirmar-pago-local-${v.id}`} value={confirmacion} onChange={(e) => setConfirmacion(e.target.value)} /></div>
            </div>
          </>
        )}
        <DialogFooter>
          <Button variant="outline" onClick={cerrar}>Cancelar</Button>
          <Button
            disabled={enviando || cargandoPagos || !tenderId || Number(importe) <= 0 || Number(importe) > pendiente + 0.001 || hayAmbiguo || confirmacion !== `PAGO LOCAL ${v.id} ${Number(importe || 0).toFixed(2)}`}
            onClick={ejecutarLocal}
          >
            {enviando ? 'Verificando…' : 'Registrar pago local'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
    <Dialog open={accion === 'facturaLocal'} onOpenChange={(open: boolean) => { if (!open) cerrar(); }}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader><DialogTitle>Facturar venta localmente</DialogTitle><DialogDescription>Genera la factura local (número F-año-000N) sin depender del BDP. Si hay pagos parciales, deben cubrir el total.</DialogDescription></DialogHeader>
        <div className="rounded border bg-muted/30 p-3 text-sm">
          <div className="flex justify-between"><span>Total</span><span className="font-semibold">{formatCurrency(pagos?.total ?? totalVenta)}</span></div>
          <div className="flex justify-between"><span>Pendiente</span><span className={`font-semibold ${pendiente > 0.01 ? 'text-amber-700' : 'text-emerald-700'}`}>{formatCurrency(pendiente)}</span></div>
          {bdp.factura_numero && (
            <div className="flex justify-between"><span>Nº factura</span><span className="font-semibold">{bdp.factura_numero}</span></div>
          )}
        </div>
        {pagos && pagos.pagos.length > 0 && pendiente > 0.01 && (
          <div className="flex items-start gap-2 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-900">
            <AlertTriangle className="mt-0.5 size-4 shrink-0" />
            <span>Queda saldo pendiente. Registra un pago por el importe restante antes de facturar.</span>
          </div>
        )}
        <div><Label htmlFor={`confirmar-factura-local-${v.id}`}>Escribe FACTURA LOCAL {v.id}</Label><Input id={`confirmar-factura-local-${v.id}`} value={confirmacion} onChange={(e) => setConfirmacion(e.target.value)} /></div>
        <DialogFooter><Button variant="outline" onClick={cerrar}>Cancelar</Button><Button disabled={enviando || (pagos !== null && pagos.pagos.length > 0 && pendiente > 0.01) || confirmacion !== `FACTURA LOCAL ${v.id}`} onClick={ejecutarLocal}>{enviando ? 'Verificando…' : 'Facturar localmente'}</Button></DialogFooter>
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
    {/* [198A-1/D8] Diálogo de propina (sumar/sustituir, D8). */}
    <Dialog open={accion === 'propina'} onOpenChange={(open: boolean) => { if (!open) cerrar(); }}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Añadir propina</DialogTitle>
          <DialogDescription>Guarda la propina localmente; si la comanda está en BDP, la empuja (sumar o sustituir).</DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-3">
          <div>
            <Label htmlFor={`propina-${v.id}`}>Importe</Label>
            <Input id={`propina-${v.id}`} type="number" min="0.01" step="0.01" value={propinaImporte} onChange={(e) => setPropinaImporte(e.target.value)} />
          </div>
          <div className="flex items-center gap-2">
            <Switch id={`propina-sumar-${v.id}`} checked={propinaSumar} onCheckedChange={setPropinaSumar} />
            <Label htmlFor={`propina-sumar-${v.id}`}>Sumar a la propina existente (si no, sustituye)</Label>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={cerrar}>Cancelar</Button>
          <Button disabled={enviando || Number(propinaImporte) <= 0} onClick={ejecutarPropina}>
            {enviando ? 'Guardando…' : 'Guardar propina'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
    </>
  );
}

export default VentaRowActions;
