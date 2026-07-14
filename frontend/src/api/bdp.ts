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

/* ── Tipos ──────────────────────────────────────────────────────────── */

/** Extensión manual de VentaConCliente con campos BDP.
 *  Cuando Orval codegen se regenere, estos campos vendrán del schema generado. */
export interface VentaConClienteBdp {
  bdp_synced?: boolean;
  bdp_order_id?: string | null;
  bdp_sync_error?: string | null;
  bdp_order_status?: string | null;
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

export async function fetchBdpStatus(ventaId: string): Promise<BdpOrderStatusResponse> {
  return customInstance<BdpOrderStatusResponse>(`/api/ventas/${ventaId}/bdp-status`, {
    method: 'GET',
  });
}

export async function fetchBdpPoll(): Promise<BdpPollResponse> {
  return customInstance<BdpPollResponse>('/api/ventas/bdp-poll', {
    method: 'POST',
  });
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
  return customInstance<BdpArticleMapItem[]>('/api/bdp/article-maps', {
    method: 'GET',
  });
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
