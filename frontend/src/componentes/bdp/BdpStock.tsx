/* [BDP-STOCK-02] Página individual de stock BDP — solo lectura.
 * Vista unificada, segura y defensiva del stock proveniente del catálogo BDP.
 * No expone ninguna operación de escritura sobre el inventario. */

import { useMemo } from 'react';
import {
  Search,
  RefreshCw,
  Loader2,
  Package,
  ArrowLeft,
  Download,
  AlertCircle,
  ChevronDown,
  ChevronUp,
} from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { useListarArticleMaps, useSyncCatalog } from '@/api/generated/bdp-mapeos/bdp-mapeos';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Skeleton } from '@/components/ui/skeleton';
import { useBdpStockFilters } from '@/hooks/useBdpStockFilters';
import { formatPrice, formatStock, formatDate, exportToCsv, PAGE_SIZES, type SortKey } from './bdp-stock-utils';

function TableSkeleton() {
  return (
    <div className="space-y-2">
      {Array.from({ length: 5 }).map((_, i) => (
        <Skeleton key={i} className="h-10 w-full" />
      ))}
    </div>
  );
}

function ReadOnlyBanner() {
  return (
    <div className="rounded-lg border bg-muted/50 p-4">
      <div className="flex items-start gap-3">
        <AlertCircle className="size-5 shrink-0 text-muted-foreground" />
        <div>
          <h3 className="text-sm font-medium">Solo lectura</h3>
          <p className="text-sm text-muted-foreground">
            Esta página muestra el stock disponible en BDP pero no permite modificarlo. Para ajustar
            inventario, usa el TPV/BDP. La exportación a CSV es informativa.
          </p>
        </div>
      </div>
    </div>
  );
}

function BdpStock() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { data, isLoading, error: listError } = useListarArticleMaps();
  const syncCatalogMutation = useSyncCatalog({
    mutation: {
      onSuccess: (resp) => {
        const d = resp as unknown as { creados?: number; actualizados?: number };
        toast.success(`Sync completado: ${d.creados ?? 0} nuevos, ${d.actualizados ?? 0} actualizados`);
        queryClient.invalidateQueries({ queryKey: ['/api/bdp/article-maps'] });
      },
      onError: () => toast.error('Error al sincronizar catálogo BDP'),
    },
  });

  const mapeos = data?.status === 200 ? data.data : [];
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
    exportToCsv(sorted);
  }

  return (
    <div className="space-y-4 p-4 md:p-6">
      <div className="flex items-center gap-2">
        <Button variant="ghost" size="icon" onClick={() => navigate('/configuracion')}>
          <ArrowLeft className="size-4" />
        </Button>
        <h1 className="text-xl font-semibold">Stock BDP</h1>
      </div>

      <ReadOnlyBanner />

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <Package className="size-4" />
            Stock de artículos BDP
          </CardTitle>
          <CardDescription>
            Vista solo lectura del stock actual en BDP. Última sincronización:{' '}
            {lastSync ? formatDate(lastSync) : '—'}.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-col gap-3 lg:flex-row lg:items-end lg:justify-between">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-end flex-wrap">
              <div className="relative w-full sm:w-64">
                <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
                <Input
                  placeholder="Código o nombre..."
                  value={filtro}
                  onChange={(e) => {
                    setFiltro(e.target.value);
                    setPage(1);
                  }}
                  className="pl-9"
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
                  <SelectItem value="all">Todos</SelectItem>
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
                  <SelectItem value="all">Todos</SelectItem>
                  <SelectItem value="active">Activos</SelectItem>
                  <SelectItem value="inactive">Inactivos</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="flex items-center gap-2">
              <TooltipButton
                variant="outline"
                onClick={() => syncCatalogMutation.mutate()}
                disabled={syncCatalogMutation.isPending}
                tooltip="Importa/actualiza artículos y stock desde BDP a Glory. No modifica BDP."
              >
                {syncCatalogMutation.isPending ? (
                  <Loader2 className="size-3.5 animate-spin" />
                ) : (
                  <RefreshCw className="size-3.5" />
                )}
                Sync catálogo
              </TooltipButton>
              <Button
                variant="outline"
                onClick={handleExport}
                disabled={paginated.length === 0}
                title="Exportar resultados filtrados a CSV"
              >
                <Download className="size-3.5 mr-1" />
                CSV
              </Button>
            </div>
          </div>

          {listError ? (
            <p className="text-sm text-destructive">
              Error al cargar el stock. Revisa que la sesión esté activa y vuelve a intentarlo.
            </p>
          ) : isLoading ? (
            <TableSkeleton />
          ) : filteredCount === 0 ? (
            <p className="text-sm text-muted-foreground">
              No hay artículos que coincidan con los filtros. Sincroniza el catálogo desde BDP.
            </p>
          ) : (
            <div className="rounded-md border overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="cursor-pointer" onClick={() => handleSort('articulo_glory_codigo')}>
                      <span className="flex items-center gap-1">Código Glory <SortIcon column="articulo_glory_codigo" /></span>
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
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {paginated.map((m) => {
                    const stock = formatStock(m.stock_actual);
                    return (
                      <TableRow key={m.id}>
                        <TableCell className="font-mono text-xs">{m.articulo_glory_codigo || '—'}</TableCell>
                        <TableCell className="font-mono text-xs">{m.articulo_bdp_codigo || '—'}</TableCell>
                        <TableCell className="text-xs">{m.articulo_bdp_nombre || '—'}</TableCell>
                        <TableCell className="text-xs tabular-nums">{formatPrice(m.precio_tarifa1)}</TableCell>
                        <TableCell>
                          {stock.hasStock ? (
                            <Badge variant="secondary" className="gap-1">
                              <Package className="size-3" />
                              {stock.text}
                            </Badge>
                          ) : (
                            <span className="text-xs text-muted-foreground">—</span>
                          )}
                        </TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </div>
          )}

          {!isLoading && !listError && filteredCount > 0 && (
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
          )}
        </CardContent>
      </Card>
    </div>
  );
}

export default BdpStock;
