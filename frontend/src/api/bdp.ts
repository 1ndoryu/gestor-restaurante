/* [147A-F5] Tipos manuales y hooks para endpoints BDP WebLink REST API.
 * Orval codegen no disponible (backend no compila localmente por módulos glory-rs faltantes).
 * Cuando se regenere el codegen, estos tipos pueden reemplazarse por los generados.
 *
 * Endpoints:
 *   GET  /api/ventas/:id/bdp-status  → BdpOrderStatusResponse
 *   POST /api/ventas/bdp-poll         → BdpPollResponse
 *   POST /api/ventas/:id/bdp-sync     → ya existe en generated (useReintentarSyncBdp)
 */

import { useMutation, useQuery } from '@tanstack/react-query';
import type { QueryClient } from '@tanstack/react-query';
import { customInstance } from './axios-instance';
import { useReintentarSyncBdp } from './generated/ventas/ventas';

/* ── Tipos ──────────────────────────────────────────────────────────── */

/** Extensión manual de VentaConCliente con campos BDP.
 *  Cuando Orval codegen se regenere, estos campos vendrán del schema generado. */
export interface VentaConClienteBdp {
  bdp_synced?: boolean;
  bdp_order_id?: string | null;
  bdp_sync_error?: string | null;
  bdp_order_status?: string | null;
  bdp_invoiced?: boolean;
}

export interface BdpOrderStatusResponse {
  venta_id: string;
  bdp_order_id: string | null;
  bdp_order_status: string | null;
  bdp_synced: boolean;
  bdp_sync_error: string | null;
}

export interface BdpPollResponse {
  updated: number;
}

/** Estado BDP mapeado para UI — hereda de VentaConCliente (generated). */
export type BdpStatus = 'pending' | 'accepted' | 'cancelled' | 'invoiced' | 'error' | 'none';

export function mapBdpStatus(
  bdpSynced?: boolean,
  bdpOrderStatus?: string | null,
  bdpSyncError?: string | null,
): BdpStatus {
  if (!bdpSynced) return 'none';
  if (bdpSyncError) return 'error';
  if (!bdpOrderStatus) return 'pending';
  switch (bdpOrderStatus) {
    case 'pending': return 'pending';
    case 'accepted': return 'accepted';
    case 'cancelled': return 'cancelled';
    case 'invoiced': return 'invoiced';
    default: return 'pending';
  }
}

/* ── API functions ──────────────────────────────────────────────────── */

/* [BKP-008c] customInstance returns { data, status, headers } (Orval pattern).
 * Manual fetchers must extract .data to return the raw payload to consumers. */

export async function fetchBdpStatus(ventaId: string): Promise<BdpOrderStatusResponse> {
  const resp = await customInstance(`/api/ventas/${ventaId}/bdp-status`, {
    method: 'GET',
  }) as { data: BdpOrderStatusResponse };
  return resp.data;
}

export async function fetchBdpPoll(): Promise<BdpPollResponse> {
  const resp = await customInstance('/api/ventas/bdp-poll', {
    method: 'POST',
  }) as { data: BdpPollResponse };
  return resp.data;
}

/** Estructura de un mapeo artículo Glory → BDP (GET /api/bdp/article-maps). */
export interface BdpArticleMapItem {
  id: string;
  user_id: string;
  articulo_glory_codigo: string;
  articulo_bdp_codigo: string;
  articulo_bdp_nombre: string;
}

/** Listar todos los mapeos de artículos Glory → BDP. */
export async function fetchBdpArticleMaps(): Promise<BdpArticleMapItem[]> {
  const resp = await customInstance('/api/bdp/article-maps', {
    method: 'GET',
  }) as { data: BdpArticleMapItem[] };
  return resp.data;
}

/* [128A-1/F3] Stock local por almacén (fuente de verdad: bdp_article_stock).
 * `stock_actual` de BdpArticleMap es el snapshot BDP de solo lectura; el
 * ajuste manual escribe aquí y nunca pisa `stock_actual`. */
export interface BdpArticleStockItem {
  id: string;
  user_id: string;
  articulo_glory_codigo: string;
  warehouse_id: string;
  warehouse_name: string;
  stock: string;
  ultima_sync_at?: string | null;
  created_at: string;
  updated_at: string;
}

/** Request para ajustar stock local (entrada/salida). */
export interface AjustarBdpArticleStockRequest {
  articulo_glory_codigo: string;
  delta: string;
  motivo: string;
  warehouse_id?: string;
  idempotency_key?: string;
}

/** Listar el stock local por almacén (GET /api/bdp/article-stock). */
export async function fetchBdpArticleStock(): Promise<BdpArticleStockItem[]> {
  const resp = await customInstance('/api/bdp/article-stock', {
    method: 'GET',
  }) as { data: BdpArticleStockItem[] };
  return resp.data;
}

/** Ajustar el stock local de un artículo (POST /api/bdp/article-stock/ajustar). */
export async function ajustarBdpArticleStock(
  req: AjustarBdpArticleStockRequest,
): Promise<BdpArticleStockItem> {
  const resp = await customInstance('/api/bdp/article-stock/ajustar', {
    method: 'POST',
    body: JSON.stringify(req),
  }) as { data: BdpArticleStockItem };
  return resp.data;
}

/* ── Hooks ──────────────────────────────────────────────────────────── */

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

/** Albarán de compra BDP importado. */
export interface BdpPurchaseNote {
  id: string;
  user_id: string;
  serie: string;
  numero: string;
  fecha: string | null;
  codigo_proveedor: string | null;
  nombre_proveedor: string | null;
  total: string | null;
  estado: 'pendiente' | 'borrador' | 'conciliado';
  gasto_id: string | null;
  datos_bdp: Record<string, unknown>;
  ultima_sync_at: string | null;
  created_at: string;
  updated_at: string;
}

/** Parámetros para listar albaranes. */
export interface BdpPurchaseNoteFilters {
  proveedor?: string;
  fecha_desde?: string;
  fecha_hasta?: string;
}

/** Request para sincronizar albaranes desde BDP. */
export interface BdpPurchaseNoteSyncRequest {
  export_profile_code?: number;
  fecha_desde?: string;
  fecha_hasta?: string;
  proveedor_desde?: number;
  proveedor_hasta?: number;
}

/** Resumen de sincronización. */
export interface BdpPurchaseNoteSyncResult {
  procesados: number;
  total_bdp: number;
}

/** Listar albaranes de compra BDP. */
export async function fetchBdpPurchaseNotes(filters: BdpPurchaseNoteFilters = {}): Promise<BdpPurchaseNote[]> {
  const params = new URLSearchParams();
  if (filters.proveedor) params.set('proveedor', filters.proveedor);
  if (filters.fecha_desde) params.set('fecha_desde', filters.fecha_desde);
  if (filters.fecha_hasta) params.set('fecha_hasta', filters.fecha_hasta);
  /* [287A-7] No añadir un separador vacío: la colección sin filtros usa su URL
   * canónica y los parámetros solo aparecen cuando realmente existen. */
  const query = params.toString();
  const url = query ? `/api/bdp/purchase-notes?${query}` : '/api/bdp/purchase-notes';
  const resp = await customInstance(url, { method: 'GET' }) as { data: BdpPurchaseNote[] };
  return resp.data;
}

/** Sincronizar albaranes de compra desde BDP. */
export async function syncBdpPurchaseNotes(req: BdpPurchaseNoteSyncRequest): Promise<BdpPurchaseNoteSyncResult> {
  const resp = await customInstance('/api/bdp/purchase-notes/sync', {
    method: 'POST',
    body: JSON.stringify(req),
  }) as { data: BdpPurchaseNoteSyncResult };
  return resp.data;
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

/** Marcar un albarán como borrador (Fase 2). */
export async function draftBdpPurchaseNote(id: string): Promise<BdpPurchaseNote> {
  const resp = await customInstance(`/api/bdp/purchase-notes/${id}/draft`, {
    method: 'POST',
    body: JSON.stringify({}),
  }) as { data: BdpPurchaseNote };
  return resp.data;
}

/** Conciliar un albarán con un gasto existente o nuevo (Fase 3). */
export async function reconcileBdpPurchaseNote(
  id: string,
  req: BdpPurchaseNoteReconcileRequest,
): Promise<BdpPurchaseNoteReconcileResult> {
  const resp = await customInstance(`/api/bdp/purchase-notes/${id}/reconcile`, {
    method: 'POST',
    body: JSON.stringify(req),
  }) as { data: BdpPurchaseNoteReconcileResult };
  return resp.data;
}

/** Request para conciliar un albarán. */
export interface BdpPurchaseNoteReconcileRequest {
  gasto_existente_id?: string;
  categoria_id?: string;
}

/** Resultado de la conciliación. */
export interface BdpPurchaseNoteReconcileResult {
  albaran_id: string;
  gasto_id: string;
  accion: string;
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
