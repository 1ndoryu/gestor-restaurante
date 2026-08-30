/* [198A-1/D6=A] Inventario físico: unidades esperadas vs contadas y diferencias.
 * [208A-2/C3] Persistencia local del conteo (decisión D3): "Guardar conteo"
 * persiste el recuento (fechado, auditable) y aplica la diferencia al stock
 * local con motivo 'conteo' (decisión D4). Si el modo efectivo es BDP, además
 * se encola el envío (UpdateMassiveInventory) para las líneas con código BDP;
 * en modo independiente no se envía nada y el mensaje lo dice con claridad
 * (ya no hay toast engañoso de "encolado"). Sección "Conteos anteriores" para
 * retomar/recontar. */

import { useMemo, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { Warehouse, Save, History, RotateCcw, Loader2 } from 'lucide-react';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { useListarArticleMaps } from '@/api/generated/bdp-mapeos/bdp-mapeos';
import { useBdpArticleStock, useCrearConteoInventario, useListarConteosInventario, obtenerConteoInventario } from '@/api/bdp';
import { useObtenerConfiguracion } from '@/api/generated/configuracion/configuracion';
import type { BdpArticleMap } from '@/api/generated/gestionRestauranteAPI.schemas';

function toNumber(value?: string | null): number {
  const n = Number(value ?? 0);
  return Number.isFinite(n) ? n : 0;
}

function formatFecha(value: string | null | undefined): string {
  if (!value) return '—';
  const d = new Date(`${value}T00:00:00`);
  if (Number.isNaN(d.getTime())) return value;
  return d.toLocaleDateString('es-ES');
}

function BdpInventario() {
  const queryClient = useQueryClient();
  const { data, isLoading } = useListarArticleMaps();
  const stockLocalQuery = useBdpArticleStock();
  const guardarMutation = useCrearConteoInventario();
  const { data: conteos, refetch: refetchConteos } = useListarConteosInventario();
  const { data: configResponse } = useObtenerConfiguracion();

  const configData = configResponse?.status === 200 ? configResponse.data : undefined;
  const modoEfectivoBdp = !!configData && (
    configData.modo_operacion === 'bdp'
    || (configData.modo_operacion === 'auto'
      && configData.bdp_sync_enabled
      && (configData.bdp_base_url ?? '').trim() !== '')
  );

  const mapeos: BdpArticleMap[] = data?.status === 200 ? data.data : [];
  const stockLocal = stockLocalQuery.data ?? [];

  const stockPorCodigo = useMemo(() => {
    const map = new Map<string, string>();
    for (const s of stockLocal) map.set(s.articulo_glory_codigo, s.stock);
    return map;
  }, [stockLocal]);

  const [contadas, setContadas] = useState<Record<string, string>>({});
  const [observaciones, setObservaciones] = useState('');
  const [retomando, setRetomando] = useState<string | null>(null);
  /* [208A-2/C3] Clave de idempotencia por sesión de conteo: se genera al
   * montar y se regenera tras guardar con éxito; en un reintento tras un
   * fallo ambiguo se reenvía la misma clave, de modo que el backend no aplica
   * la diferencia dos veces (D4). */
  const [conteoKey, setConteoKey] = useState(() => crypto.randomUUID());

  const esperada = (m: BdpArticleMap): number => toNumber(stockPorCodigo.get(m.articulo_glory_codigo) ?? m.stock_actual);

  const filas = useMemo(
    () =>
      mapeos.map((m) => {
        const e = esperada(m);
        const c = toNumber(contadas[m.articulo_glory_codigo]);
        const contadaVacia = (contadas[m.articulo_glory_codigo] ?? '') === '';
        return { m, esperada: e, contada: c, diferencia: contadaVacia ? null : c - e };
      }),
    /* eslint-disable-next-line react-hooks/exhaustive-deps */
    [mapeos, contadas, stockPorCodigo],
  );

  const contados = filas.filter((f) => f.diferencia !== null);

  function guardar() {
    if (contados.length === 0) {
      toast.error('Introduce al menos una unidad contada');
      return;
    }
    guardarMutation.mutate(
      {
        observaciones: observaciones.trim() || undefined,
        idempotency_key: conteoKey,
        articulos: contados.map((f) => ({
          articulo_glory_codigo: f.m.articulo_glory_codigo,
          unidades_contadas: String(f.contada),
        })),
      },
      {
        onSuccess: (r) => {
          if (r.reutilizado) {
            toast.info('Conteo ya guardado', {
              description: 'Esta sesión de conteo ya se aplicó; no se vuelve a ajustar el stock.',
            });
            queryClient.invalidateQueries({ queryKey: ['/api/bdp/inventario/conteos'] });
            refetchConteos();
            return;
          }
          const base = `Conteo guardado: ${r.aplicadas} artículo(s) ajustado(s) en stock local`;
          if (modoEfectivoBdp) {
            toast.success(base, {
              description: r.encolados > 0
                ? `${r.encolados} encolado(s) para BDP${r.omitidos_sin_bdp ? ` · ${r.omitidos_sin_bdp} sin código BDP omitidos` : ''}`
                : r.omitidos_sin_bdp
                  ? `${r.omitidos_sin_bdp} sin código BDP (no se envían)`
                  : 'Sin líneas con código BDP que enviar',
            });
          } else {
            toast.success(base, {
              description: 'Modo independiente: el conteo se guarda localmente y no se envía a BDP.',
            });
          }
          setObservaciones('');
          setContadas({});
          setConteoKey(crypto.randomUUID());
          queryClient.invalidateQueries({ queryKey: ['/api/bdp/inventario/conteos'] });
          queryClient.invalidateQueries({ queryKey: ['/api/bdp/article-stock'] });
          refetchConteos();
        },
        onError: (err: unknown) => {
          const msg = (err as { response?: { data?: { message?: string } } })?.response?.data?.message;
          toast.error('No se pudo guardar el conteo', { description: msg });
        },
      },
    );
  }

  async function retomar(conteoId: string) {
    setRetomando(conteoId);
    try {
      const detalle = await obtenerConteoInventario(conteoId);
      const mapa: Record<string, string> = {};
      for (const l of detalle.lineas) mapa[l.articulo_glory_codigo] = String(l.contado);
      setContadas(mapa);
      toast.success('Conteo cargado', {
        description: 'Recuenta las unidades y pulsa "Guardar conteo": se guardará como un conteo nuevo y ajustará el stock de nuevo.',
      });
    } catch {
      toast.error('No se pudo cargar el conteo anterior');
    } finally {
      setRetomando(null);
    }
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-2 rounded-md border p-3">
        <p className="text-sm text-muted-foreground">
          <Warehouse className="size-3.5 inline mr-1" />
          {contados.length} artículos contados · {filas.length} en catálogo
        </p>
        <div className="flex flex-col gap-2 sm:flex-row sm:items-end">
          <div className="flex flex-col gap-1 flex-1">
            <Label htmlFor="inventario-observaciones" className="text-xs">Observaciones (opcional)</Label>
            <Input
              id="inventario-observaciones"
              value={observaciones}
              onChange={(e) => setObservaciones(e.target.value)}
              placeholder="Ej: recuento semanal de almacén"
              maxLength={500}
            />
          </div>
          <Button onClick={guardar} disabled={guardarMutation.isPending || contados.length === 0}>
            {guardarMutation.isPending ? <Loader2 className="size-4 animate-spin mr-1" /> : <Save className="size-4 mr-1" />}
            Guardar conteo
          </Button>
        </div>
        <p className="text-xs text-muted-foreground">
          {modoEfectivoBdp
            ? 'El conteo se guarda, ajusta el stock local (motivo "conteo") y encola el envío al terminal para los artículos con código BDP.'
            : 'Modo independiente: el conteo se guarda y ajusta el stock local (motivo "conteo"); no se envía a BDP.'}
        </p>
      </div>

      {isLoading ? (
        <p className="text-sm text-muted-foreground">Cargando…</p>
      ) : filas.length === 0 ? (
        <div className="flex flex-col items-start gap-2 rounded-md border border-dashed p-4">
          <p className="text-sm text-muted-foreground">
            No hay artículos en el catálogo. Crea artículos desde Stock o Catálogo para poder inventariar.
          </p>
        </div>
      ) : (
        <div className="rounded-md border overflow-x-auto">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Código</TableHead>
                <TableHead>Nombre</TableHead>
                <TableHead className="text-right">Esperadas</TableHead>
                <TableHead className="text-right">Contadas</TableHead>
                <TableHead className="text-right">Diferencia</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filas.map(({ m, esperada: e, diferencia }) => (
                <TableRow key={m.id}>
                  <TableCell className="font-mono text-xs">{m.articulo_glory_codigo || '—'}</TableCell>
                  <TableCell className="text-xs">{m.articulo_bdp_nombre || '—'}</TableCell>
                  <TableCell className="text-right tabular-nums">{e.toFixed(0)}</TableCell>
                  <TableCell className="text-right">
                    <Input
                      type="number"
                      step="any"
                      className="w-24 ml-auto text-right"
                      value={contadas[m.articulo_glory_codigo] ?? ''}
                      onChange={(ev) => setContadas((p) => ({ ...p, [m.articulo_glory_codigo]: ev.target.value }))}
                      placeholder="0"
                    />
                  </TableCell>
                  <TableCell className={`text-right tabular-nums ${diferencia !== null && diferencia !== 0 ? 'font-semibold text-amber-700' : 'text-muted-foreground'}`}>
                    {diferencia === null ? '—' : diferencia === 0 ? '0' : `${diferencia > 0 ? '+' : ''}${diferencia.toFixed(0)}`}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}

      <div className="flex flex-col gap-2">
        <p className="text-sm font-medium flex items-center gap-1.5">
          <History className="size-4" />
          Conteos anteriores
        </p>
        {!conteos || conteos.length === 0 ? (
          <p className="text-xs text-muted-foreground">Todavía no hay conteos guardados.</p>
        ) : (
          <div className="rounded-md border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Fecha</TableHead>
                  <TableHead>Observaciones</TableHead>
                  <TableHead className="text-right">Líneas</TableHead>
                  <TableHead>Estado</TableHead>
                  <TableHead className="w-32 text-center">Acciones</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {conteos.map((c) => (
                  <TableRow key={c.id}>
                    <TableCell className="text-xs tabular-nums">{formatFecha(c.fecha)}</TableCell>
                    <TableCell className="max-w-64 truncate text-xs" title={c.observaciones || undefined}>
                      {c.observaciones || '—'}
                    </TableCell>
                    <TableCell className="text-right text-xs tabular-nums">{c.total_lineas}</TableCell>
                    <TableCell>
                      <Badge variant="secondary" className="gap-1">
                        <Save className="size-3" />
                        {c.estado === 'aplicado' ? 'aplicado' : c.estado}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-center">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => retomar(c.id)}
                        disabled={retomando === c.id}
                      >
                        <RotateCcw className="size-3.5 mr-1" />
                        Retomar
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}
      </div>
    </div>
  );
}

export default BdpInventario;
