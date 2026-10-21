/* Hooks de React Query para los endpoints BDP. Las peticiones viven en los
 * módulos requests-*; aquí solo se componen con cachés e invalidaciones. */

import { useMutation, useQuery } from '@tanstack/react-query';
import type { QueryClient, UseMutationOptions } from '@tanstack/react-query';
import { useReintentarSyncBdp } from '../generated/ventas/ventas';
import type { ActualizarBdpMenuLocalRequest, ActualizarBdpPurchaseNoteRequest, AgregarPropinaRequest, AnularVentaResponse, BdpCatalogoTipo, BdpMenuLocalFilters, BdpPurchaseNoteFilters, BdpPurchaseNoteReconcileRequest, SumarPuntosRequest } from './tipos';
import { agregarPropinaVenta, anularVenta, fetchBdpPoll, fetchBdpStatus, fetchPuntosCliente, llamarCamarero, sumarPuntosCliente } from './requests-ventas';
import { ajustarBdpArticleStock, crearBdpClasificacion, fetchBdpArticleMaps, fetchBdpArticleStock, fetchBdpCatalogo, importarDepartamentosBdp, traspasarBdpArticleStock } from './requests-articulos';
import { crearConteoInventario, listarConteosInventario, registrarInventario } from './requests-inventario';
import { actualizarBdpPurchaseNote, crearBdpPurchaseNote, draftBdpPurchaseNote, eliminarBdpPurchaseNote, fetchBdpPurchaseNotes, reconcileBdpPurchaseNote, syncBdpPurchaseNotes } from './requests-albaranes';
import { actualizarBdpMenuLocal, crearBdpMenuLocal, eliminarBdpMenuLocal, fetchBdpMenuLocal, fetchBdpMenusLocales } from './requests-menus';
import { flushBdpPush, listarPushFilas, reintentarPushFila, sincronizarStockAlmacen } from './requests-sincronizacion';

/** Query hook: obtener estado BDP de una venta individual. */
export function useBdpStatus(ventaId: string | null, enabled = true) {
  return useQuery({
    queryKey: ['bdp-status', ventaId],
    queryFn: () => fetchBdpStatus(ventaId!),
    enabled: !!ventaId && enabled,
    staleTime: 30_000, /* 30s — polling manual, no automático */
  });
}

/** Mutation hook: disparar polling de todas las ventas BDP pendientes. */
export function useBdpPoll(queryClient?: QueryClient) {
  return useMutation({
    mutationFn: fetchBdpPoll,
    onSuccess: () => {
      queryClient?.invalidateQueries({ queryKey: ['listarVentas'] });
    },
  });
}

/** Query hook: obtener mapeos de artículos Glory → BDP. */
export function useBdpArticleMaps(enabled = true) {
  return useQuery({
    queryKey: ['bdp-article-maps'],
    queryFn: fetchBdpArticleMaps,
    enabled,
    staleTime: 5 * 60_000, /* 5 min — cambia poco */
  });
}

/** Query hook: stock local por almacén. */
export function useBdpArticleStock(enabled = true) {
  return useQuery({
    queryKey: ['bdp-article-stock'],
    queryFn: fetchBdpArticleStock,
    enabled,
    staleTime: 30_000,
  });
}

/** Mutation hook: ajuste manual de stock local.
 * Invalida stock local y catálogo (el merge de BdpStock usa ambos). */
export function useAjustarBdpArticleStock(queryClient?: QueryClient) {
  return useMutation({
    mutationFn: ajustarBdpArticleStock,
    onSuccess: () => {
      queryClient?.invalidateQueries({ queryKey: ['bdp-article-stock'] });
      queryClient?.invalidateQueries({ queryKey: ['/api/bdp/article-maps'] });
    },
  });
}

/** Mutation hook: traspasar stock entre almacenes de BDP. Invalida el stock
 * local porque el flush puede mover filas de ambos almacenes. */
export function useTraspasarBdpArticleStock(queryClient?: QueryClient) {
  return useMutation({
    mutationFn: traspasarBdpArticleStock,
    onSuccess: () => {
      queryClient?.invalidateQueries({ queryKey: ['bdp-article-stock'] });
      queryClient?.invalidateQueries({ queryKey: ['/api/bdp/article-maps'] });
    },
  });
}

/** Query hook: obtener albaranes de compra BDP. */
export function useBdpPurchaseNotes(filters: BdpPurchaseNoteFilters = {}, enabled = true) {
  return useQuery({
    queryKey: ['bdp-purchase-notes', filters],
    queryFn: () => fetchBdpPurchaseNotes(filters),
    enabled,
    staleTime: 5 * 60_000,
  });
}

/** Mutation hook: sincronizar albaranes de compra BDP. */
export function useSyncBdpPurchaseNotes(queryClient?: QueryClient) {
  return useMutation({
    mutationFn: syncBdpPurchaseNotes,
    onSuccess: () => {
      queryClient?.invalidateQueries({ queryKey: ['bdp-purchase-notes'] });
    },
  });
}

/** Mutation hook: crear albarán de compra local (F5). */
export function useCrearBdpPurchaseNote(queryClient?: QueryClient) {
  return useMutation({
    mutationFn: crearBdpPurchaseNote,
    onSuccess: () => {
      queryClient?.invalidateQueries({ queryKey: ['bdp-purchase-notes'] });
    },
  });
}

/** Mutation hook: actualizar albarán de compra local (F5). */
export function useActualizarBdpPurchaseNote(queryClient?: QueryClient) {
  return useMutation({
    mutationFn: ({ id, req }: { id: string; req: ActualizarBdpPurchaseNoteRequest }) =>
      actualizarBdpPurchaseNote(id, req),
    onSuccess: () => {
      queryClient?.invalidateQueries({ queryKey: ['bdp-purchase-notes'] });
    },
  });
}

/** Mutation hook: eliminar albarán de compra local (F5). */
export function useEliminarBdpPurchaseNote(queryClient?: QueryClient) {
  return useMutation({
    mutationFn: eliminarBdpPurchaseNote,
    onSuccess: () => {
      queryClient?.invalidateQueries({ queryKey: ['bdp-purchase-notes'] });
    },
  });
}

/** Mutation hook: marcar albarán como borrador. */
export function useDraftBdpPurchaseNote(queryClient?: QueryClient) {
  return useMutation({
    mutationFn: draftBdpPurchaseNote,
    onSuccess: () => {
      queryClient?.invalidateQueries({ queryKey: ['bdp-purchase-notes'] });
    },
  });
}

/** Mutation hook: conciliar albarán. */
export function useReconcileBdpPurchaseNote(queryClient?: QueryClient) {
  return useMutation({
    mutationFn: ({ id, req }: { id: string; req: BdpPurchaseNoteReconcileRequest }) =>
      reconcileBdpPurchaseNote(id, req),
    onSuccess: () => {
      queryClient?.invalidateQueries({ queryKey: ['bdp-purchase-notes'] });
    },
  });
}

/** Mutation hook: reintentar sincronización BDP individual de una venta.
 *  Usa el hook generado por Orval (useReintentarSyncBdp) y añade invalidación de queries. */
export function useRetryBdpSync(queryClient?: QueryClient) {
  const generated = useReintentarSyncBdp();
  return {
    ...generated,
    mutateAsync: async (ventaId: string) => {
      const result = await generated.mutateAsync({ id: ventaId, data: {} });
      queryClient?.invalidateQueries({ queryKey: ['listarVentas'] });
      return result;
    },
  };
}

/** [D26b] Sync del stock real por almacén (`GetListStock`). Invalida el stock
 *  mapeado para que la tabla lo relea. */
export function useSyncBdpStockAlmacen(queryClient?: QueryClient) {
  return useMutation({
    mutationFn: (almacen?: number) => sincronizarStockAlmacen(almacen),
    onSuccess: () => queryClient?.invalidateQueries({ queryKey: ['bdp-article-stock'] }),
  });
}

/** Query hook: listar menús/packs locales. */
export function useBdpMenusLocales(filters: BdpMenuLocalFilters = {}, enabled = true) {
  return useQuery({
    queryKey: ['bdp-menus-locales', filters],
    queryFn: () => fetchBdpMenusLocales(filters),
    enabled,
    staleTime: 30_000,
  });
}

/** Query hook: detalle de un menú/pack local. */
export function useBdpMenuLocal(id: string | null, enabled = true) {
  return useQuery({
    queryKey: ['bdp-menus-locales', id],
    queryFn: () => fetchBdpMenuLocal(id!),
    enabled: !!id && enabled,
    staleTime: 30_000,
  });
}

/** Mutation hook: crear menú/pack local (F7). */
export function useCrearBdpMenuLocal(queryClient?: QueryClient) {
  return useMutation({
    mutationFn: crearBdpMenuLocal,
    onSuccess: () => {
      queryClient?.invalidateQueries({ queryKey: ['bdp-menus-locales'] });
    },
  });
}

/** Mutation hook: actualizar menú/pack local (F7). */
export function useActualizarBdpMenuLocal(queryClient?: QueryClient) {
  return useMutation({
    mutationFn: ({ id, req }: { id: string; req: ActualizarBdpMenuLocalRequest }) =>
      actualizarBdpMenuLocal(id, req),
    onSuccess: () => {
      queryClient?.invalidateQueries({ queryKey: ['bdp-menus-locales'] });
    },
  });
}

/** Mutation hook: eliminar menú/pack local (F7). */
export function useEliminarBdpMenuLocal(queryClient?: QueryClient) {
  return useMutation({
    mutationFn: eliminarBdpMenuLocal,
    onSuccess: () => {
      queryClient?.invalidateQueries({ queryKey: ['bdp-menus-locales'] });
    },
  });
}

/** Mutation hook: anulación local de una venta (D4, C1 idempotencia). */
export function useAnularVenta(queryClient?: QueryClient, options?: UseMutationOptions<AnularVentaResponse, unknown, { ventaId: string; req: { motivo?: string; idempotency_key?: string } }>) {
  return useMutation<AnularVentaResponse, unknown, { ventaId: string; req: { motivo?: string; idempotency_key?: string } }>({
    ...options,
    mutationFn: ({ ventaId, req }) => anularVenta(ventaId, req),
    onSuccess: (data, variables, onMutateResult, context) => {
      options?.onSuccess?.(data, variables, onMutateResult, context);
      queryClient?.invalidateQueries({ queryKey: ['listarVentas'] });
    },
    onError: (error, variables, onMutateResult, context) => {
      options?.onError?.(error, variables, onMutateResult, context);
    },
  });
}

export function useAgregarPropinaVenta(queryClient?: QueryClient) {
  return useMutation({
    mutationFn: ({ ventaId, req }: { ventaId: string; req: AgregarPropinaRequest }) =>
      agregarPropinaVenta(ventaId, req),
    onSuccess: () => {
      queryClient?.invalidateQueries({ queryKey: ['listarVentas'] });
    },
  });
}

export function usePuntosCliente(clienteId: string | null, enabled = true) {
  return useQuery({
    queryKey: ['cliente-puntos', clienteId],
    queryFn: () => fetchPuntosCliente(clienteId!),
    enabled: !!clienteId && enabled,
    staleTime: 30_000,
  });
}

export function useSumarPuntosCliente(queryClient?: QueryClient) {
  return useMutation({
    mutationFn: ({ clienteId, req }: { clienteId: string; req: SumarPuntosRequest }) =>
      sumarPuntosCliente(clienteId, req),
    onSuccess: (_data, variables) => {
      queryClient?.invalidateQueries({ queryKey: ['cliente-puntos', variables.clienteId] });
    },
  });
}

export function useBdpCatalogo(tipo: BdpCatalogoTipo, enabled = true) {
  return useQuery({
    queryKey: ['bdp-catalogo', tipo],
    queryFn: () => fetchBdpCatalogo(tipo),
    enabled,
    staleTime: 60_000,
  });
}

export function useCrearBdpClasificacion(queryClient?: QueryClient) {
  return useMutation({
    mutationFn: crearBdpClasificacion,
    onSuccess: (_data, variables) => {
      queryClient?.invalidateQueries({ queryKey: ['bdp-catalogo', variables.tipo] });
    },
  });
}

export function useImportarDepartamentosBdp() {
  return useMutation({ mutationFn: importarDepartamentosBdp });
}

export function useLlamarCamarero() {
  return useMutation({ mutationFn: llamarCamarero });
}

export function useRegistrarInventario() {
  return useMutation({ mutationFn: registrarInventario });
}

export function useListarConteosInventario() {
  return useQuery({ queryKey: ['/api/bdp/inventario/conteos'], queryFn: listarConteosInventario });
}

export function useCrearConteoInventario() {
  return useMutation({ mutationFn: crearConteoInventario });
}

export function useFlushBdpPush() {
  return useMutation({ mutationFn: flushBdpPush });
}

export function useListarPushFilas() {
  return useQuery({ queryKey: ['/api/bdp/push/pendientes'], queryFn: listarPushFilas });
}

export function useReintentarPushFila() {
  return useMutation({ mutationFn: reintentarPushFila });
}
