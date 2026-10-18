/* [BDP-STOCK-03] Página de stock BDP.
 * Estructura coherente con ListaVentas/ListaGastos/ListaReservas.
 * Añadido modo demo para visualizar datos de prueba sin conexión a BDP.
 * [128A-1/F3] Stock local editable: merge de `stock_actual` (snapshot BDP,
 * solo lectura) con `bdp_article_stock` (stock local por almacén, fuente de
 * verdad editable). Botón "Ajustar" por fila → modal delta/motivo. */

import { useMemo, useState } from 'react';
import { Package, ChevronDown, ChevronUp, SlidersHorizontal, Plus, RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { useListarArticleMaps } from '@/api/generated/bdp-mapeos/bdp-mapeos';
import { useBdpStockFilters } from '@/hooks/useBdpStockFilters';
import { useBdpDemoMode } from '@/hooks/useBdpDemoMode';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { useBdpArticleStock, useAjustarBdpArticleStock } from '@/api/bdp';
import { formatPrice, formatStock, formatDate, exportToCsv, type SortKey } from './bdp-stock-utils';
import { mockArticleMaps } from './bdp-mocks';
import { BdpStockActions } from './BdpStockActions';
import NuevoArticuloDialog from './NuevoArticuloDialog';
import { useObtenerConfiguracion } from '@/api/generated/configuracion/configuracion';
import type { BdpArticleMap } from '@/api/generated/gestionRestauranteAPI.schemas';
import { AjustarStockDialog } from './bdp-stock/AjustarStockDialog';
import { FiltrosStock } from './bdp-stock/FiltrosStock';
import { PaginacionStock } from './bdp-stock/PaginacionStock';
import { TableSkeleton } from './bdp-stock/TableSkeleton';

function BdpStock() {
  const queryClient = useQueryClient();
  /* [159A-3] El toggle demo vive en Configuración → BDP; aquí solo se lee. */
  const { demoMode } = useBdpDemoMode();
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
  const { data, isLoading, error: listError, refetch } = useListarArticleMaps({
    query: { enabled: !demoMode },
  });
  const stockLocalQuery = useBdpArticleStock(!demoMode);
  const ajustarMutation = useAjustarBdpArticleStock(queryClient);
  const apiRows = data?.status === 200 ? data.data : [];
  const mapeos = demoMode ? mockArticleMaps : apiRows;
  const stockLocal = stockLocalQuery.data ?? [];

  /* [128A-1/F3] Merge: el stock local (bdp_article_stock) manda sobre el
   * snapshot BDP (stock_actual). El badge indica el origen del valor.
   * [H-S1] El stock EFECTIVO (local primero) es la única fuente para la
   * tabla, los filtros con/sin stock, el orden por stock y el export CSV:
   * antes el CSV/filtros usaban el snapshot BDP (0) aunque la UI mostrase
   * el stock local (p.ej. 43 local), exportando ceros engañosos. */
  const stockPorCodigo = useMemo(() => {
    const map = new Map<string, string>();
    for (const s of stockLocal) {
      map.set(s.articulo_glory_codigo, s.stock);
    }
    return map;
  }, [stockLocal]);

  const codigosStockLocal = useMemo(() => new Set(stockLocal.map((s) => s.articulo_glory_codigo)), [stockLocal]);

  const mapeosConStockEfectivo = useMemo(
    () =>
      mapeos.map((m) => ({
        ...m,
        stock_actual: stockPorCodigo.get(m.articulo_glory_codigo) ?? m.stock_actual,
      })),
    [mapeos, stockPorCodigo],
  );

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
  } = useBdpStockFilters(mapeosConStockEfectivo);

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
    exportToCsv(mapeosConStockEfectivo, sorted, {
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
        onExport={handleExport}
        onNuevo={() => setNuevoOpen(true)}
        nuevoDisabled={demoMode}
      />

      {/* [159A-4] El BDP no devuelve existencias (spike verificado: todo
        * artículo "NO ES DEL TIPO WEB", `ExportArticles` vacío). «—» significa
        * que el artículo aún no tiene stock local; se fija con «Ajustar» o con
        * un conteo en Inventario. */}
      <p className="text-sm text-muted-foreground">
        El BDP no devuelve existencias: «—» significa que el artículo aún no tiene stock local.
        Fíjalo con «Ajustar» o con un conteo en Inventario.
      </p>

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
        <FiltrosStock
          filtro={filtro}
          setFiltro={setFiltro}
          stockFilter={stockFilter}
          setStockFilter={setStockFilter}
          activeFilter={activeFilter}
          setActiveFilter={setActiveFilter}
          setPage={setPage}
        />
      </div>

      {hasError ? (
        <div className="flex flex-col items-start gap-3 rounded-md border border-destructive/50 p-4">
          <p className="text-sm text-destructive">
            Error al cargar el stock. Vuelve a intentarlo; si persiste, revisa la conexión con el servidor.
          </p>
          <Button variant="outline" size="sm" onClick={() => refetch()}>
            <RefreshCw className="size-3.5 mr-1.5" />
            Reintentar
          </Button>
        </div>
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
                  const origen = codigosStockLocal.has(m.articulo_glory_codigo) ? 'local' : 'bdp';
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

          <PaginacionStock
            mostrados={paginated.length}
            total={filteredCount}
            page={page}
            totalPages={totalPages}
            setPage={setPage}
            pageSize={pageSize}
            setPageSize={setPageSize}
          />
        </>
      )}
    </div>
  );
}

export default BdpStock;
