/* [198A-1/D6=A] Inventario físico: unidades esperadas vs contadas y diferencias.
 * [208A-2/C3] Persistencia local del conteo (decisión D3): "Guardar conteo"
 * persiste el recuento (fechado, auditable) y aplica la diferencia al stock
 * local con motivo 'conteo' (decisión D4). Si el modo efectivo es BDP, además
 * se encola el envío (UpdateMassiveInventory) para las líneas con código BDP;
 * en modo independiente no se envía nada y el mensaje lo dice con claridad
 * (ya no hay toast engañoso de "encolado"). Dos pestañas: "Inventario" (recuento
 * en curso) y "Conteos" (historial para retomar/recontar). El guardado pide
 * confirmación en un modal con el resumen antes de ajustar el stock. */

import { useMemo } from 'react';
import { Input } from '@/components/ui/input';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Warehouse, History } from 'lucide-react';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { useListarArticleMaps } from '@/api/generated/bdp-mapeos/bdp-mapeos';
import { useBdpArticleStock, useCrearConteoInventario, useListarConteosInventario } from '@/api/bdp';
import { useObtenerConfiguracion } from '@/api/generated/configuracion/configuracion';
import type { BdpArticleMap } from '@/api/generated/gestionRestauranteAPI.schemas';
import { useConteoInventario } from '@/hooks/useConteoInventario';
import { useVistaInventario } from './bdp-inventario/useVistaInventario';
import type { AlmacenBdp } from './bdp-stock/TraspasarStockDialog';
import { DialogoConfirmarConteo } from './bdp-inventario/DialogoConfirmarConteo';
import { PestanaConteos } from './bdp-inventario/PestanaConteos';
import { toNumber } from './bdp-inventario/formato';
import { TablaInventario, PaginacionInventario } from './bdp-inventario/TablaInventario';
import { CabeceraInventario } from './bdp-inventario/CabeceraInventario';

function BdpInventario() {
  const queryClient = useQueryClient();
  const { data, isLoading } = useListarArticleMaps();
  const stockLocalQuery = useBdpArticleStock();
  const guardarMutation = useCrearConteoInventario();
  /* [L7] El almacén vive en el hook del conteo porque «Retomar» lo cambia al
   * del conteo cargado; se lee antes del historial para pedirlo filtrado. */
  const {
    contadas,
    observaciones,
    setObservaciones,
    retomando,
    conteoKey,
    almacen,
    setAlmacen,
    setContada,
    retomarConteo,
    limpiar,
  } = useConteoInventario();
  const { data: conteos, refetch: refetchConteos } = useListarConteosInventario(almacen);
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

  /* [L7] Primero el stock del almacén elegido; si ese almacén aún no tiene
   * fila para el artículo, la del agregado «General» (0), que es de donde
   * salía el esperado antes de que el conteo tuviera almacén propio. */
  const stockPorCodigo = useMemo(() => {
    const delAlmacen = new Map<string, string>();
    const general = new Map<string, string>();
    for (const s of stockLocal) {
      if (s.warehouse_id === almacen) delAlmacen.set(s.articulo_glory_codigo, s.stock);
      if (s.warehouse_id === '0') general.set(s.articulo_glory_codigo, s.stock);
    }
    for (const [codigo, stock] of general) {
      if (!delAlmacen.has(codigo)) delAlmacen.set(codigo, stock);
    }
    return delAlmacen;
  }, [stockLocal, almacen]);

  /* Almacenes que ya conocemos por el stock local: BDP no los expone por API
   * (mismo criterio que el traspaso). */
  const almacenes = useMemo<AlmacenBdp[]>(() => {
    const vistos = new Map<number, string>();
    for (const fila of stockLocal) {
      const id = Number(fila.warehouse_id);
      if (!Number.isInteger(id) || vistos.has(id)) continue;
      vistos.set(id, fila.warehouse_name || `Almacén ${id}`);
    }
    return [...vistos].map(([id, nombre]) => ({ id, nombre })).sort((a, b) => a.id - b.id);
  }, [stockLocal]);

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
  const { confirmar, setConfirmar, tab, setTab, pagina, setPagina, busqueda, setBusqueda, paginaConteos, setPaginaConteos } = useVistaInventario();

  /* Paginación fija 50/página: el catálogo completo (557 filas) no cabe en
   * una sola vista. Las contadas viven en el hook (por código), así que
   * cambiar de página no pierde lo ya contado. */
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
        warehouse_id: almacen,
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
          {/* [L7] El conteo es de un almacén concreto: el esperado, el ajuste
              local y el envío a BDP salen de aquí. «General» (0) es el stock
              sin desglose, el que había antes de este selector. */}
          <CabeceraInventario
            contados={contados.length}
            totalCatalogo={filas.length}
            guardando={guardarMutation.isPending}
            almacen={almacen}
            almacenes={almacenes}
            onGuardar={() => setConfirmar(true)}
            onAlmacen={(valor) => { setAlmacen(valor.trim() === '' ? '0' : valor); setPagina(1); }}
          />

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
            <TablaInventario filas={filasPagina} contadas={contadas} onContada={setContada} />
          )}

          {filasFiltradas.length > PAGE_SIZE && (
            <PaginacionInventario
              mostrados={filasPagina.length}
              totalFiltrados={filasFiltradas.length}
              totalCatalogo={filas.length}
              filtrado={busquedaNormalizada !== ''}
              pagina={paginaSegura}
              totalPaginas={totalPaginas}
              onAnterior={() => setPagina((p) => Math.max(1, p - 1))}
              onSiguiente={() => setPagina((p) => Math.min(totalPaginas, p + 1))}
            />
          )}
        </TabsContent>

        <TabsContent value="conteos" className="flex flex-col gap-2">
          <PestanaConteos
            conteos={conteos}
            conteosPagina={conteosPagina}
            pagina={paginaConteosSegura}
            totalPaginas={totalPaginasConteos}
            tamanoPagina={CONTEOS_PAGE_SIZE}
            retomando={retomando}
            onRetomar={retomar}
            onAnterior={() => setPaginaConteos((p) => Math.max(1, p - 1))}
            onSiguiente={() => setPaginaConteos((p) => Math.min(totalPaginasConteos, p + 1))}
          />
        </TabsContent>
      </Tabs>

      <DialogoConfirmarConteo
        abierto={confirmar}
        onAbiertoChange={setConfirmar}
        numContados={contados.length}
        conDiferencia={conDiferencia}
        almacen={almacen}
        observaciones={observaciones}
        onObservacionesChange={setObservaciones}
        modoEfectivoBdp={modoEfectivoBdp}
        guardando={guardarMutation.isPending}
        onConfirmar={guardar}
      />
    </div>
  );
}

export default BdpInventario;
