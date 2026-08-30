/* [208A-2/C4] Sección "Sincronización" (decisión D5): visibilidad de la cola
 * de push Glory → BDP. Lista las filas con estado, reintentos y último error,
 * permite reintentar individualmente (regla D2: el reintento tras bloqueo por
 * suscripción es solo manual) y disparar el flush global.
 * Visible en el menú; en modo independiente se muestra con motivo claro y las
 * acciones quedan deshabilitadas (R13.2: nada se ofrece ni se envía). */

import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { RefreshCw, Loader2, Send, ShieldAlert, Info } from 'lucide-react';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { useListarPushFilas, useReintentarPushFila, useFlushBdpPush } from '@/api/bdp';
import { useObtenerConfiguracion } from '@/api/generated/configuracion/configuracion';
import type { BdpPushFila } from '@/api/bdp';

const ETIQUETAS_DOMINIO: Record<string, string> = {
  articulo: 'Artículo',
  stock: 'Stock',
  departamento: 'Departamento',
  familia: 'Familia',
  venta: 'Venta',
  cliente_puntos: 'Puntos cliente',
  propina: 'Propina',
};

function badgeEstado(estado: string) {
  switch (estado) {
    case 'pendiente':
      return <Badge variant="outline">pendiente</Badge>;
    case 'pendiente_suscripcion':
      return <Badge variant="outline" className="border-amber-400 text-amber-700 bg-amber-50 dark:bg-amber-950/30 dark:text-amber-400">suscripción</Badge>;
    case 'error':
      return <Badge variant="destructive">error</Badge>;
    case 'sincronizado':
      return <Badge variant="secondary" className="text-emerald-700 dark:text-emerald-400">sincronizado</Badge>;
    case 'descartado':
      return <Badge variant="secondary">descartado</Badge>;
    default:
      return <Badge variant="outline">{estado}</Badge>;
  }
}

function formatFecha(value: string | null | undefined): string {
  if (!value) return '—';
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return value;
  return d.toLocaleString('es-ES', { day: '2-digit', month: '2-digit', year: '2-digit', hour: '2-digit', minute: '2-digit' });
}

function BdpSincronizacion() {
  const queryClient = useQueryClient();
  const { data: filas, isLoading } = useListarPushFilas();
  const reintentarMutation = useReintentarPushFila();
  const flushMutation = useFlushBdpPush();
  const [reintentandoId, setReintentandoId] = useState<string | null>(null);
  const { data: configResponse } = useObtenerConfiguracion();

  const configData = configResponse?.status === 200 ? configResponse.data : undefined;
  const modoEfectivoBdp = !!configData && (
    configData.modo_operacion === 'bdp'
    || (configData.modo_operacion === 'auto'
      && configData.bdp_sync_enabled
      && (configData.bdp_base_url ?? '').trim() !== '')
  );

  const filasTotales = filas ?? [];

  function reinvalidar() {
    queryClient.invalidateQueries({ queryKey: ['/api/bdp/push/pendientes'] });
  }

  function flushGlobal() {
    flushMutation.mutate(undefined, {
      onSuccess: (r) => {
        if (r.sincronizados > 0) {
          toast.success('Sincronizado con BDP', { description: `${r.sincronizados} operación(es) enviada(s).` });
        } else if (r.pendientes_suscripcion > 0) {
          toast.warning('Pendiente de suscripción BDP', { description: `${r.pendientes_suscripcion} operación(es) requieren la suscripción WebLink.` });
        } else if (r.errores > 0) {
          toast.error('Errores al sincronizar', { description: `${r.errores} operación(es) fallaron.` });
        } else if (r.omitidos_standalone > 0) {
          toast.info('Modo independiente', { description: 'La cola no se envía mientras no haya BDP conectado.' });
        } else {
          toast.info('Sin operaciones pendientes');
        }
        reinvalidar();
      },
      onError: (err: unknown) => {
        const msg = (err as { response?: { data?: { message?: string } } })?.response?.data?.message;
        toast.error('No se pudo sincronizar', { description: msg });
      },
    });
  }

  function reintentar(fila: BdpPushFila) {
    setReintentandoId(fila.id);
    reintentarMutation.mutate(fila.id, {
      onSuccess: (r) => {
        if (r.sincronizados > 0) {
          toast.success('Operación sincronizada');
        } else if (r.pendientes_suscripcion > 0) {
          toast.warning('Pendiente de suscripción BDP', { description: 'La suscripción WebLink aún no está activa.' });
        } else if (r.omitidos_standalone > 0) {
          toast.info('Modo independiente', { description: 'La cola no se envía mientras no haya BDP conectado.' });
        } else {
          toast.error('La operación falló', { description: fila.ultimo_error ?? 'Error desconocido' });
        }
        reinvalidar();
      },
      onError: (err: unknown) => {
        const msg = (err as { response?: { data?: { message?: string } } })?.response?.data?.message;
        toast.error('No se pudo reintentar', { description: msg });
      },
      onSettled: () => setReintentandoId(null),
    });
  }

  const esTerminal = (estado: string) => estado === 'sincronizado' || estado === 'descartado';

  return (
    <div className="flex flex-col gap-4">
      {!modoEfectivoBdp && (
        <div className="flex items-start gap-2 rounded-md border border-dashed p-3 text-sm text-muted-foreground">
          <ShieldAlert className="mt-0.5 size-4 shrink-0" />
          <p>
            <strong>Requiere BDP conectado.</strong> En modo independiente los cambios locales (artículos, stock,
            departamentos, propinas, puntos, cancelaciones) se guardan localmente y quedan pendientes; se enviarán
            cuando la integración BDP esté activa y pulses "Sincronizar ahora" o el reintento individual.
          </p>
        </div>
      )}

      <div className="flex flex-col gap-2 rounded-md border p-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <p className="text-sm text-muted-foreground">
            <Send className="size-3.5 inline mr-1" />
            {filasTotales.length} filas en la cola de sincronización · {filasTotales.filter((f) => f.estado === 'sincronizado').length} sincronizadas
          </p>
          <Button onClick={flushGlobal} disabled={flushMutation.isPending || !modoEfectivoBdp} title={!modoEfectivoBdp ? 'Requiere BDP conectado' : undefined}>
            {flushMutation.isPending ? <Loader2 className="size-4 animate-spin mr-1" /> : <RefreshCw className="size-4 mr-1" />}
            Sincronizar ahora
          </Button>
        </div>
        <p className="text-xs text-muted-foreground">
          <Info className="size-3.5 inline mr-1" />
          El reintento individual es manual: una operación bloqueada por la suscripción WebLink solo se reintenta aquí o
          con "Sincronizar ahora" (no hay reintento automático para ese caso).
        </p>
      </div>

      {isLoading ? (
        <p className="text-sm text-muted-foreground">Cargando cola…</p>
      ) : filasTotales.length === 0 ? (
        <div className="rounded-md border border-dashed p-4">
          <p className="text-sm text-muted-foreground">
            No hay operaciones en la cola. Las ediciones locales (artículos, stock, departamentos, propinas, puntos,
            cancelaciones) aparecerán aquí cuando se encolen con la integración BDP activa.
          </p>
        </div>
      ) : (
        <div className="rounded-md border overflow-x-auto">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Dominio</TableHead>
                <TableHead>Operación</TableHead>
                <TableHead>Entidad</TableHead>
                <TableHead>Estado</TableHead>
                <TableHead className="text-right">Reintentos</TableHead>
                <TableHead>Último error</TableHead>
                <TableHead>Actualizado</TableHead>
                <TableHead className="w-28 text-center">Acciones</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filasTotales.map((fila) => (
                <TableRow key={fila.id}>
                  <TableCell className="text-xs">{ETIQUETAS_DOMINIO[fila.dominio] ?? fila.dominio}</TableCell>
                  <TableCell className="font-mono text-xs">{fila.operacion}</TableCell>
                  <TableCell className="font-mono text-xs">{fila.entidad_id}</TableCell>
                  <TableCell>{badgeEstado(fila.estado)}</TableCell>
                  <TableCell className="text-right text-xs tabular-nums">{fila.reintentos}</TableCell>
                  <TableCell className="max-w-56 truncate text-xs" title={fila.ultimo_error ?? undefined}>
                    {fila.ultimo_error || '—'}
                  </TableCell>
                  <TableCell className="text-xs tabular-nums">{formatFecha(fila.updated_at)}</TableCell>
                  <TableCell className="text-center">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => reintentar(fila)}
                      disabled={esTerminal(fila.estado) || reintentandoId === fila.id || !modoEfectivoBdp}
                      title={esTerminal(fila.estado) ? 'Ya sincronizada/descartada' : !modoEfectivoBdp ? 'Requiere BDP conectado' : 'Reintentar esta operación manualmente'}
                    >
                      {reintentandoId === fila.id ? <Loader2 className="size-3.5 animate-spin" /> : <RefreshCw className="size-3.5" />}
                      Reintentar
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}
    </div>
  );
}

export default BdpSincronizacion;
