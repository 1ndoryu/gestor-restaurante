/* [BDP-STOCK-03] Página de stock BDP.
 * Estructura coherente con ListaVentas/ListaGastos/ListaReservas.
 * Añadido modo demo para visualizar datos de prueba sin conexión a BDP.
 * [128A-1/F3] Stock local editable: merge de `stock_actual` (snapshot BDP,
 * solo lectura) con `bdp_article_stock` (stock local por almacén, fuente de
 * verdad editable). Botón "Ajustar" por fila → modal delta/motivo. */

import { useMemo, useState } from 'react';
import {
  Search,
  Package,
  ChevronDown,
  ChevronUp,
  SlidersHorizontal,
  Plus,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogClose } from '@/components/ui/dialog';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { useListarArticleMaps } from '@/api/generated/bdp-mapeos/bdp-mapeos';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Skeleton } from '@/components/ui/skeleton';
import { useBdpStockFilters } from '@/hooks/useBdpStockFilters';
import { useBdpDemoMode } from '@/hooks/useBdpDemoMode';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { useBdpArticleStock, useAjustarBdpArticleStock } from '@/api/bdp';
import { formatPrice, formatStock, formatDate, exportToCsv, PAGE_SIZES, type SortKey } from './bdp-stock-utils';
import { mockArticleMaps } from './bdp-mocks';
import { BdpStockActions } from './BdpStockActions';
import NuevoArticuloDialog from './NuevoArticuloDialog';
import { useObtenerConfiguracion } from '@/api/generated/configuracion/configuracion';
import type { BdpArticleMap } from '@/api/generated/gestionRestauranteAPI.schemas';

/* Skeletons estáticos: filas sin identidad propia; claves de un array estable
 * (no el índice del map) para reconciliación consistente. */
const SKELETON_IDS = [0, 1, 2, 3, 4];

function TableSkeleton() {
  return (
    <div className="space-y-2">
      {SKELETON_IDS.map((id) => (
        <Skeleton key={id} className="h-10 w-full" />
      ))}
    </div>
  );
}

/* [128A-1/F3] Modal de ajuste manual de stock local. */
function AjustarStockDialog({
  articulo,
  open,
  onOpenChange,
  onGuardar,
  pending,
}: {
  articulo: BdpArticleMap | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onGuardar: (delta: string, motivo: string) => void;
  pending: boolean;
}) {
  const [delta, setDelta] = useState('');
  const [motivo, setMotivo] = useState('');
  const [ultimoArticuloKey, setUltimoArticuloKey] = useState<string | null>(null);

  const deltaNum = Number(delta);
  const deltaValido = delta.trim() !== '' && Number.isFinite(deltaNum) && deltaNum !== 0;

  /* [128A-1/F3] Reset del formulario cuando cambia el artículo del modal. */
  const articuloKey = articulo?.id ?? 'none';
  if (ultimoArticuloKey !== articuloKey) {
    setDelta('');
    setMotivo('');
    setUltimoArticuloKey(articuloKey);
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Ajustar stock de {articulo?.articulo_glory_codigo ?? ''}</DialogTitle>
          <DialogDescription>
            Entrada (delta positivo) o salida (delta negativo) de stock local.
            El stock BDP (snapshot) no se modifica.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-2">
          <div className="grid gap-2">
            <Label htmlFor="ajuste-delta">Cantidad (delta)</Label>
            <Input
              id="ajuste-delta"
              type="number"
              step="any"
              placeholder="Ej: 5 o -3"
              value={delta}
              onChange={(e) => setDelta(e.target.value)}
            />
            {!deltaValido && delta.trim() !== '' && (
              <p className="text-xs text-destructive">El delta debe ser un número distinto de cero.</p>
            )}
          </div>
          <div className="grid gap-2">
            <Label htmlFor="ajuste-motivo">Motivo</Label>
            <Textarea
              id="ajuste-motivo"
              placeholder="Ej: entrada de mercancía, merma, conteo..."
              value={motivo}
              onChange={(e) => setMotivo(e.target.value)}
              maxLength={255}
            />
          </div>
        </div>
        <DialogFooter>
          <DialogClose asChild>
            <Button variant="outline">Cancelar</Button>
          </DialogClose>
          <Button
            onClick={() => onGuardar(delta, motivo)}
            disabled={pending || !deltaValido || motivo.trim() === ''}
          >
            Guardar
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function BdpStock() {
  const queryClient = useQueryClient();
  const { demoMode, setDemoMode } = useBdpDemoMode();
  /* [208A-2/C2] Modo efectivo (misma lógica que el backend/site-header):
   * en standalone no se ofrecen acciones BDP (H7). */
  const { data: configResponse } = useObtenerConfiguracion();
  const configData = configResponse?.status === 200 ? configResponse.data : undefined;
  const modoEfectivoBdp = !!configData && (
    configData.modo_operacion === 'bdp'
    || (configData.modo_operacion === 'auto'
      && configData.bdp_sync_enabled
      && (configData.bdp_base_url ?? '').trim() !== '')
  );
  const { data, isLoading, error: listError } = useListarArticleMaps({
    query: { enabled: !demoMode },
  });
  const stockLocalQuery = useBdpArticleStock(!demoMode);
  const ajustarMutation = useAjustarBdpArticleStock(queryClient);
  const apiRows = data?.status === 200 ? data.data : [];
  const mapeos = demoMode ? mockArticleMaps : apiRows;
  const stockLocal = stockLocalQuery.data ?? [];

  /* [128A-1/F3] Merge: el stock local (bdp_article_stock) manda sobre el
   * snapshot BDP (stock_actual). El badge indica el origen del valor. */
  const stockPorCodigo = useMemo(() => {
    const map = new Map<string, string>();
    for (const s of stockLocal) {
      map.set(s.articulo_glory_codigo, s.stock);
    }
    return map;
  }, [stockLocal]);

  const [ajustarArticulo, setAjustarArticulo] = useState<BdpArticleMap | null>(null);
  const [ajusteOpen, setAjusteOpen] = useState(false);
  const [nuevoOpen, setNuevoOpen] = useState(false);

  function guardarAjuste(delta: string, motivo: string) {
    if (!ajustarArticulo) return;
    ajustarMutation.mutate(
      {
        articulo_glory_codigo: ajustarArticulo.articulo_glory_codigo,
        delta,
        motivo,
        idempotency_key: `ajuste-${ajustarArticulo.id}-${Date.now()}`,
      },
      {
        onSuccess: () => {
          toast.success(`Stock ajustado para ${ajustarArticulo.articulo_glory_codigo}`);
          setAjusteOpen(false);
          setAjustarArticulo(null);
        },
        onError: () => toast.error('Error al ajustar el stock'),
      },
    );
  }

  const lastSync = useMemo(() => {
    if (!mapeos.length) return null;
    const dates = mapeos.map((m) => m.ultima_sync_at).filter(Boolean) as string[];
    if (!dates.length) return null;
    dates.sort((a, b) => new Date(b).getTime() - new Date(a).getTime());
    return dates[0];
  }, [mapeos]);

  const {
    filtro,
    setFiltro,
    stockFilter,
    setStockFilter,
    activeFilter,
    setActiveFilter,
    sortKey,
    setSortKey,
    sortDir,
    setSortDir,
    page,
    setPage,
    pageSize,
    setPageSize,
    totalPages,
    paginated,
    sorted,
    filteredCount,
  } = useBdpStockFilters(mapeos);

  function handleSort(key: SortKey) {
    if (sortKey === key) {
      setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'));
    } else {
      setSortKey(key);
      setSortDir('asc');
    }
    setPage(1);
  }

  function SortIcon({ column }: { column: SortKey }) {
    if (sortKey !== column) return <span className="inline-block w-3" />;
    return sortDir === 'asc' ? <ChevronUp className="size-3" /> : <ChevronDown className="size-3" />;
  }

  function handleExport() {
    exportToCsv(mapeos, sorted, {
      allRows: false,
      filterLabel: stockFilter !== 'all' ? stockFilter : undefined,
    });
  }

  const isLoadingEffective = !demoMode && isLoading;
  const hasError = !demoMode && !!listError;

  return (
    <div className="flex flex-col gap-4">
      <BdpStockActions
        summary={`${filteredCount} artículos · Última sync: ${lastSync ? formatDate(lastSync) : '—'}`}
        demoMode={demoMode}
        bdpMode={modoEfectivoBdp}
        exportDisabled={paginated.length === 0}
        onToggleDemo={setDemoMode}
        onExport={handleExport}
      />

      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="text-sm text-muted-foreground">
          {modoEfectivoBdp
            ? 'Stock local editable; con BDP conectado, el ajuste también se encola al terminal.'
            : 'Modo independiente: el stock se gestiona localmente y no se envía a BDP.'}
        </p>
        <Button variant="default" size="sm" onClick={() => setNuevoOpen(true)} disabled={demoMode}>
          <Plus className="size-3.5 mr-1" />
          Nuevo artículo
        </Button>
      </div>

      <NuevoArticuloDialog open={nuevoOpen} onOpenChange={setNuevoOpen} />

      <AjustarStockDialog
        articulo={ajustarArticulo}
        open={ajusteOpen}
        onOpenChange={(open) => {
          setAjusteOpen(open);
          if (!open) setAjustarArticulo(null);
        }}
        onGuardar={guardarAjuste}
        pending={ajustarMutation.isPending}
      />

      <div className="flex flex-col gap-3 lg:flex-row lg:items-end lg:justify-between">
        <div className="flex flex-wrap gap-3 items-center">
          <div className="relative w-full sm:w-64">
            <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
            <Input
              placeholder="Código o nombre..."
              value={filtro}
              onChange={(e) => {
                setFiltro(e.target.value);
                setPage(1);
              }}
              className="pl-9 max-w-xs"
            />
          </div>
          <Select
            value={stockFilter}
            onValueChange={(v) => {
              setStockFilter(v as typeof stockFilter);
              setPage(1);
            }}
          >
            <SelectTrigger className="w-full sm:w-44">
              <SelectValue placeholder="Stock" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">Cualquier stock</SelectItem>
              <SelectItem value="with">Con stock</SelectItem>
              <SelectItem value="without">Sin stock</SelectItem>
            </SelectContent>
          </Select>
          <Select
            value={activeFilter}
            onValueChange={(v) => {
              setActiveFilter(v as typeof activeFilter);
              setPage(1);
            }}
          >
            <SelectTrigger className="w-full sm:w-44">
              <SelectValue placeholder="Estado" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">Cualquier estado</SelectItem>
              <SelectItem value="active">Activos</SelectItem>
              <SelectItem value="inactive">Inactivos</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>

      {hasError ? (
        <p className="text-sm text-destructive">
          Error al cargar el stock. Revisa que la sesión esté activa y vuelve a intentarlo.
        </p>
      ) : isLoadingEffective ? (
        <TableSkeleton />
      ) : filteredCount === 0 ? (
        mapeos.length === 0 ? (
          <div className="flex flex-col items-start gap-3 rounded-md border border-dashed p-4">
            <p className="text-sm text-muted-foreground">
              No hay artículos todavía. Crea el primero con "Nuevo artículo" — funciona en modo
              independiente, sin BDP.
            </p>
            <Button variant="default" size="sm" onClick={() => setNuevoOpen(true)} disabled={demoMode}>
              <Plus className="size-3.5 mr-1" />
              Nuevo artículo
            </Button>
          </div>
        ) : (
          <p className="text-sm text-muted-foreground">
            No hay artículos que coincidan con los filtros.
          </p>
        )
      ) : (
        <>
          <div className="rounded-md border overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="cursor-pointer" onClick={() => handleSort('articulo_glory_codigo')}>
                    <span className="flex items-center gap-1">Código Aplicación Web <SortIcon column="articulo_glory_codigo" /></span>
                  </TableHead>
                  <TableHead className="cursor-pointer" onClick={() => handleSort('articulo_bdp_codigo')}>
                    <span className="flex items-center gap-1">Código BDP <SortIcon column="articulo_bdp_codigo" /></span>
                  </TableHead>
                  <TableHead className="cursor-pointer" onClick={() => handleSort('articulo_bdp_nombre')}>
                    <span className="flex items-center gap-1">Nombre BDP <SortIcon column="articulo_bdp_nombre" /></span>
                  </TableHead>
                  <TableHead className="cursor-pointer" onClick={() => handleSort('precio_tarifa1')}>
                    <span className="flex items-center gap-1">Precio <SortIcon column="precio_tarifa1" /></span>
                  </TableHead>
                  <TableHead className="cursor-pointer" onClick={() => handleSort('stock_actual')}>
                    <span className="flex items-center gap-1">Stock <SortIcon column="stock_actual" /></span>
                  </TableHead>
                  <TableHead className="text-center">Acciones</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {paginated.map((m) => {
                  const stockLocalVal = stockPorCodigo.get(m.articulo_glory_codigo);
                  const origen = stockLocalVal !== undefined ? 'local' : 'bdp';
                  const stock = formatStock(stockLocalVal ?? m.stock_actual);
                  return (
                    <TableRow key={m.id}>
                      <TableCell className="font-mono text-xs">{m.articulo_glory_codigo || '—'}</TableCell>
                      <TableCell className="font-mono text-xs">{m.articulo_bdp_codigo || '—'}</TableCell>
                      <TableCell className="text-xs">{m.articulo_bdp_nombre || '—'}</TableCell>
                      <TableCell className="text-xs tabular-nums">{formatPrice(m.precio_tarifa1)}</TableCell>
                      <TableCell>
                        {stock.hasStock ? (
                          <span className="inline-flex items-center gap-1">
                            <Badge variant={origen === 'local' ? 'secondary' : 'outline'} className="gap-1">
                              <Package className="size-3" />
                              {stock.text}
                            </Badge>
                            <Badge variant={origen === 'local' ? 'secondary' : 'outline'}>
                              {origen}
                            </Badge>
                          </span>
                        ) : (
                          <span className="text-xs text-muted-foreground">—</span>
                        )}
                      </TableCell>
                      <TableCell>
                        <div className="flex justify-center">
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => {
                              setAjustarArticulo(m);
                              setAjusteOpen(true);
                            }}
                          >
                            <SlidersHorizontal className="size-3" />
                            Ajustar
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          </div>

          <div className="flex flex-col sm:flex-row items-center justify-between gap-3 text-sm">
            <span className="text-muted-foreground">
              Mostrando {paginated.length} de {filteredCount} artículos
            </span>
            <div className="flex items-center gap-2">
              <Button variant="outline" size="sm" onClick={() => setPage((p) => Math.max(1, p - 1))} disabled={page <= 1}>
                Anterior
              </Button>
              <span className="text-muted-foreground">
                Página {page} de {totalPages}
              </span>
              <Button
                variant="outline"
                size="sm"
                onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
                disabled={page >= totalPages}
              >
                Siguiente
              </Button>
            </div>
            <Select
              value={String(pageSize)}
              onValueChange={(v) => {
                setPageSize(Number(v) as 10 | 25 | 50);
                setPage(1);
              }}
            >
              <SelectTrigger className="w-20">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {PAGE_SIZES.map((s) => (
                  <SelectItem key={s} value={String(s)}>
                    {s}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </>
      )}
    </div>
  );
}

export default BdpStock;
