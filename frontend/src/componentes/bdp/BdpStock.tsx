/* [BDP-STOCK-03] Página de stock BDP.
 * Estructura coherente con ListaVentas/ListaGastos/ListaReservas.
 * Añadido modo demo para visualizar datos de prueba sin conexión a BDP.
 * [128A-1/F3] Stock local editable: merge de `stock_actual` (snapshot BDP,
 * solo lectura) con `bdp_article_stock` (stock local por almacén, fuente de
 * verdad editable). Botón "Ajustar" por fila → modal delta/motivo. */

import { useMemo, useState } from 'react';
import { Plus, RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useListarArticleMaps } from '@/api/generated/bdp-mapeos/bdp-mapeos';
import { useBdpStockFilters } from '@/hooks/useBdpStockFilters';
import { useBdpDemoMode } from '@/hooks/useBdpDemoMode';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { useBdpArticleStock, useAjustarBdpArticleStock, useTraspasarBdpArticleStock } from '@/api/bdp';
import type { TraspasarBdpArticleStockRequest } from '@/api/bdp/tipos';
import { formatDate, exportToCsv, type SortKey } from './bdp-stock-utils';
import { mockArticleMaps } from './bdp-mocks';
import { BdpStockActions } from './BdpStockActions';
import NuevoArticuloDialog from './NuevoArticuloDialog';
import { useObtenerConfiguracion } from '@/api/generated/configuracion/configuracion';
import type { BdpArticleMap } from '@/api/generated/gestionRestauranteAPI.schemas';
import { AjustarStockDialog } from './bdp-stock/AjustarStockDialog';
import { TablaStock } from './bdp-stock/TablaStock';
import { TraspasarStockDialog, type AlmacenBdp } from './bdp-stock/TraspasarStockDialog';
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
  const traspasarMutation = useTraspasarBdpArticleStock(queryClient);
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

  /* Un solo diálogo abierto a la vez: ajuste local o traspaso entre almacenes.
   * Así el estado del modal no se duplica ni puede quedar desincronizado. */
  const [dialogo, setDialogo] = useState<{ tipo: 'ajuste' | 'traspaso'; articulo: BdpArticleMap } | null>(null);
  const [nuevoOpen, setNuevoOpen] = useState(false);
  const articuloDialogo = dialogo?.articulo ?? null;
  function cerrarDialogo() {
    setDialogo(null);
  }

  /* Almacenes que ya conocemos por el stock local: BDP no los expone por API. */
  const almacenes = useMemo<AlmacenBdp[]>(() => {
    const vistos = new Map<number, string>();
    for (const fila of stockLocal) {
      const id = Number(fila.warehouse_id);
      if (!Number.isInteger(id) || vistos.has(id)) continue;
      vistos.set(id, fila.warehouse_name || `Almacén ${id}`);
    }
    return [...vistos].map(([id, nombre]) => ({ id, nombre })).sort((a, b) => a.id - b.id);
  }, [stockLocal]);

  function guardarAjuste(delta: string, motivo: string) {
    if (!dialogo) return;
    const articulo = dialogo.articulo;
    ajustarMutation.mutate(
      {
        articulo_glory_codigo: articulo.articulo_glory_codigo,
        delta,
        motivo,
        idempotency_key: `ajuste-${articulo.id}-${Date.now()}`,
      },
      {
        onSuccess: () => {
          toast.success(`Stock ajustado para ${articulo.articulo_glory_codigo}`);
          cerrarDialogo();
        },
        onError: () => toast.error('Error al ajustar el stock'),
      },
    );
  }

  function guardarTraspaso(req: TraspasarBdpArticleStockRequest) {
    traspasarMutation.mutate(req, {
      onSuccess: (resp) => {
        toast.success('Traspaso encolado', {
          description: `Se enviará a BDP en el próximo envío autorizado (${resp.entidad}).`,
        });
        cerrarDialogo();
      },
      onError: (err: unknown) => {
        const msg = (err as { response?: { data?: { message?: string } } })?.response?.data?.message;
        toast.error('No se pudo traspasar', { description: msg ?? 'Revisa los almacenes y las unidades.' });
      },
    });
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
        articulo={articuloDialogo}
        open={dialogo?.tipo === 'ajuste'}
        onOpenChange={(open) => { if (!open) cerrarDialogo(); }}
        onGuardar={guardarAjuste}
        pending={ajustarMutation.isPending}
      />

      <TraspasarStockDialog
        articulo={articuloDialogo}
        almacenes={almacenes}
        open={dialogo?.tipo === 'traspaso'}
        onOpenChange={(open) => { if (!open) cerrarDialogo(); }}
        onGuardar={guardarTraspaso}
        pending={traspasarMutation.isPending}
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
          <TablaStock
            filas={paginated}
            stockPorCodigo={stockPorCodigo}
            codigosStockLocal={codigosStockLocal}
            sortKey={sortKey}
            sortDir={sortDir}
            onOrdenar={handleSort}
            onAjustar={(articulo) => setDialogo({ tipo: 'ajuste', articulo })}
            onTraspasar={(articulo) => setDialogo({ tipo: 'traspaso', articulo })}
          />

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
