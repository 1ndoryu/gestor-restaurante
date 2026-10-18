/* Diálogos de las acciones de una venta: pago BDP y local, facturas, anulación y propina.
 * Vive aparte de venta-row-actions.tsx para mantener cada archivo bajo el límite de líneas. */

import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Switch } from '@/components/ui/switch';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { AlertTriangle } from 'lucide-react';
import type { VentaConCliente } from '../api/generated';
import type { useVentaRowActions } from '../hooks/useVentaRowActions';

type Acciones = ReturnType<typeof useVentaRowActions>;
type ListaPagos = NonNullable<Acciones['pagos']>['pagos'];
type Importe = Parameters<typeof formatCurrency>[0];

interface VentaRowDialogosProps {
  v: VentaConCliente;
  acciones: Acciones;
  anularPending: boolean;
}

function formatCurrency(value?: string | number | null): string {
  if (value == null) return '—';
  const n = Number(value);
  if (Number.isNaN(n)) return '—';
  return `${n.toFixed(2)} €`;
}

/* Tres tarjetas de importe (total, pagado, pendiente) y el aviso de pago ambiguo. */
function ResumenPagos({ total, pagado, pendiente, hayAmbiguo }: { total: Importe; pagado: Importe; pendiente: Importe; hayAmbiguo: boolean }) {
  return (
    <>
      <div className="grid grid-cols-3 gap-2 text-sm">
        <div className="rounded border bg-muted/30 p-2 text-center">
          <div className="text-muted-foreground">Total</div>
          <div className="font-semibold">{formatCurrency(total)}</div>
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
    </>
  );
}

/* Historial de pagos ya registrados (fecha, importe y resultado). */
function TablaPagos({ lista }: { lista: ListaPagos }) {
  return (
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
            {lista.map((p) => (
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
  );
}

export function VentaRowDialogos({ v, acciones, anularPending }: VentaRowDialogosProps) {
  const {
    bdp, totalVenta, accion, tenderId, setTenderId, importe, setImporte,
    confirmacion, setConfirmacion, enviando, pagos, cargandoPagos,
    anularAbierto, motivo, setMotivo, propinaImporte, setPropinaImporte,
    propinaSumar, setPropinaSumar, hayAmbiguo, pendiente, pagado, cerrar,
    ejecutarBdp, ejecutarPropina, ejecutarLocal, motivoObligatorio,
    anulacionPendienteBdp, cerrarAnulacion, ejecutarAnulacion,
  } = acciones;

  return (
    <>
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
            <ResumenPagos total={pagos?.total ?? totalVenta} pagado={pagado} pendiente={pendiente} hayAmbiguo={hayAmbiguo} />
            {pagos && pagos.pagos.length > 0 && <TablaPagos lista={pagos.pagos} />}
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
            <ResumenPagos total={pagos?.total ?? totalVenta} pagado={pagado} pendiente={pendiente} hayAmbiguo={hayAmbiguo} />
            {pagos && pagos.pagos.length > 0 && <TablaPagos lista={pagos.pagos} />}
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
