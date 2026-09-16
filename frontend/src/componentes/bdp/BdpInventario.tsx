/* [198A-1/D6=A] Inventario físico: unidades esperadas vs contadas y diferencias.
 * [208A-2/C3] Persistencia local del conteo (decisión D3): "Guardar conteo"
 * persiste el recuento (fechado, auditable) y aplica la diferencia al stock
 * local con motivo 'conteo' (decisión D4). Si el modo efectivo es BDP, además
 * se encola el envío (UpdateMassiveInventory) para las líneas con código BDP;
 * en modo independiente no se envía nada y el mensaje lo dice con claridad
 * (ya no hay toast engañoso de "encolado"). Dos pestañas: "Inventario" (recuento
 * en curso) y "Conteos" (historial para retomar/recontar). El guardado pide
 * confirmación en un modal con el resumen antes de ajustar el stock. */

import { useMemo, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Warehouse, Save, History, RotateCcw, Loader2 } from 'lucide-react';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { useListarArticleMaps } from '@/api/generated/bdp-mapeos/bdp-mapeos';
import { useBdpArticleStock, useCrearConteoInventario, useListarConteosInventario } from '@/api/bdp';
import { useObtenerConfiguracion } from '@/api/generated/configuracion/configuracion';
import type { BdpArticleMap } from '@/api/generated/gestionRestauranteAPI.schemas';
import { useConteoInventario } from '@/hooks/useConteoInventario';

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

  const {
    contadas,
    observaciones,
    setObservaciones,
    retomando,
    conteoKey,
    setContada,
    retomarConteo,
    limpiar,
  } = useConteoInventario();

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
  const conDiferencia = contados.filter((f) => f.diferencia !== 0).length;
  const [confirmar, setConfirmar] = useState(false);
  const [tab, setTab] = useState('inventario');

  /* Paginación fija 50/página: el catálogo completo (557 filas) no cabe en
   * una sola vista. Las contadas viven en el hook (por código), así que
   * cambiar de página no pierde lo ya contado. */
  const [pagina, setPagina] = useState(1);
  const [busqueda, setBusqueda] = useState('');
  const PAGE_SIZE = 50;
  const busquedaNormalizada = busqueda.trim().toLowerCase();
  const filasFiltradas = busquedaNormalizada === ''
    ? filas
    : filas.filter((f) =>
      (f.m.articulo_glory_codigo ?? '').toLowerCase().includes(busquedaNormalizada)
      || (f.m.articulo_bdp_nombre ?? '').toLowerCase().includes(busquedaNormalizada),
    );
  const totalPaginas = Math.max(1, Math.ceil(filasFiltradas.length / PAGE_SIZE));
  const paginaSegura = Math.min(pagina, totalPaginas);
  const filasPagina = filasFiltradas.slice((paginaSegura - 1) * PAGE_SIZE, paginaSegura * PAGE_SIZE);

  /* Historial de conteos: paginación 10/página. */
  const [paginaConteos, setPaginaConteos] = useState(1);
  const CONTEOS_PAGE_SIZE = 10;
  const totalPaginasConteos = Math.max(1, Math.ceil((conteos?.length ?? 0) / CONTEOS_PAGE_SIZE));
  const paginaConteosSegura = Math.min(paginaConteos, totalPaginasConteos);
  const conteosPagina = (conteos ?? []).slice(
    (paginaConteosSegura - 1) * CONTEOS_PAGE_SIZE,
    paginaConteosSegura * CONTEOS_PAGE_SIZE,
  );

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
          setConfirmar(false);
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
          limpiar();
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
    const ok = await retomarConteo(conteoId);
    if (ok) {
      setTab('inventario');
      toast.success('Conteo cargado', {
        description: 'Recuenta las unidades y pulsa "Guardar conteo": se guardará como un conteo nuevo y ajustará el stock de nuevo.',
      });
    } else {
      toast.error('No se pudo cargar el conteo anterior');
    }
  }

  return (
    <div className="flex flex-col gap-4">
      <Tabs value={tab} onValueChange={setTab}>
        <TabsList>
          <TabsTrigger value="inventario">
            <Warehouse className="size-4 mr-1" />
            Inventario
          </TabsTrigger>
          <TabsTrigger value="conteos">
            <History className="size-4 mr-1" />
            Conteos{conteos && conteos.length > 0 ? ` (${conteos.length})` : ''}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="inventario" className="flex flex-col gap-4">
          <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
            <p className="text-sm text-muted-foreground">
              {contados.length} artículos contados · {filas.length} en catálogo
            </p>
            <Button onClick={() => setConfirmar(true)} disabled={guardarMutation.isPending || contados.length === 0}>
              {guardarMutation.isPending ? <Loader2 className="size-4 animate-spin mr-1" /> : <Save className="size-4 mr-1" />}
              Guardar conteo
            </Button>
          </div>

          <Input
            value={busqueda}
            onChange={(e) => { setBusqueda(e.target.value); setPagina(1); }}
            placeholder="Buscar por código o nombre…"
            maxLength={100}
            className="max-w-sm"
          />

          {isLoading ? (
            <p className="text-sm text-muted-foreground">Cargando…</p>
          ) : filasFiltradas.length === 0 ? (
            <div className="flex flex-col items-start gap-2 rounded-md border border-dashed p-4">
              <p className="text-sm text-muted-foreground">
                {busquedaNormalizada === ''
                  ? 'No hay artículos en el catálogo. Crea artículos desde Stock o Catálogo para poder inventariar.'
                  : `Sin resultados para «${busqueda.trim()}».`}
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
                  {filasPagina.map(({ m, esperada: e, diferencia }) => (
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
                          onChange={(ev) => setContada(m.articulo_glory_codigo, ev.target.value)}
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

          {filasFiltradas.length > PAGE_SIZE && (
            <div className="flex flex-col sm:flex-row items-center justify-between gap-3 text-sm">
              <span className="text-muted-foreground">
                Mostrando {filasPagina.length} de {filasFiltradas.length} artículos
                {busquedaNormalizada !== '' ? ` (filtrados de ${filas.length})` : ''}
              </span>
              <div className="flex items-center gap-2">
                <Button variant="outline" size="sm" onClick={() => setPagina((p) => Math.max(1, p - 1))} disabled={paginaSegura <= 1}>
                  Anterior
                </Button>
                <span className="text-muted-foreground">
                  Página {paginaSegura} de {totalPaginas}
                </span>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setPagina((p) => Math.min(totalPaginas, p + 1))}
                  disabled={paginaSegura >= totalPaginas}
                >
                  Siguiente
                </Button>
              </div>
            </div>
          )}
        </TabsContent>

        <TabsContent value="conteos" className="flex flex-col gap-2">
          {!conteos || conteos.length === 0 ? (
            <p className="text-xs text-muted-foreground">Todavía no hay conteos guardados.</p>
          ) : (
            <>
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
                  {conteosPagina.map((c) => (
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
            {(conteos?.length ?? 0) > CONTEOS_PAGE_SIZE && (
              <div className="flex flex-col sm:flex-row items-center justify-between gap-3 text-sm">
                <span className="text-muted-foreground">
                  Mostrando {conteosPagina.length} de {conteos?.length} conteos
                </span>
                <div className="flex items-center gap-2">
                  <Button variant="outline" size="sm" onClick={() => setPaginaConteos((p) => Math.max(1, p - 1))} disabled={paginaConteosSegura <= 1}>
                    Anterior
                  </Button>
                  <span className="text-muted-foreground">
                    Página {paginaConteosSegura} de {totalPaginasConteos}
                  </span>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setPaginaConteos((p) => Math.min(totalPaginasConteos, p + 1))}
                    disabled={paginaConteosSegura >= totalPaginasConteos}
                  >
                    Siguiente
                  </Button>
                </div>
              </div>
            )}
            </>
          )}
        </TabsContent>
      </Tabs>

      <Dialog open={confirmar} onOpenChange={setConfirmar}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Guardar conteo</DialogTitle>
            <DialogDescription>
              Se ajustará el stock local con las diferencias contadas. Esta acción queda registrada con motivo «conteo».
            </DialogDescription>
          </DialogHeader>
          <div className="flex flex-col gap-3 text-sm">
            <p>
              <span className="font-medium">{contados.length}</span> artículo(s) contados ·{' '}
              <span className="font-medium">{conDiferencia}</span> con diferencia
            </p>
            <div className="flex flex-col gap-1">
              <Label htmlFor="inventario-observaciones" className="text-xs">Observaciones (opcional)</Label>
              <Input
                id="inventario-observaciones"
                value={observaciones}
                onChange={(e) => setObservaciones(e.target.value)}
                placeholder="Ej: recuento semanal de almacén"
                maxLength={500}
              />
            </div>
            <p className="text-xs text-muted-foreground">
              {modoEfectivoBdp
                ? 'El conteo se guarda, ajusta el stock local y encola el envío al terminal para los artículos con código BDP.'
                : 'Modo independiente: el conteo se guarda y ajusta el stock local; no se envía a BDP.'}
            </p>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setConfirmar(false)} disabled={guardarMutation.isPending}>
              Cancelar
            </Button>
            <Button onClick={guardar} disabled={guardarMutation.isPending || contados.length === 0}>
              {guardarMutation.isPending ? <Loader2 className="size-4 animate-spin mr-1" /> : <Save className="size-4 mr-1" />}
              Confirmar y guardar
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

export default BdpInventario;
