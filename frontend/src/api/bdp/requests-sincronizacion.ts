/* Peticiones de sincronización con BDP: flush manual de la cola de push y sus pendientes. */

import { customInstance } from '../axios-instance';
import type { BdpPushFila, BdpPushFlushResumen } from './tipos';

/** Dispara el flush manual de la cola de push (POST /api/bdp/push/flush).
 *  D2: el reintento tras bloqueo por suscripción es SOLO manual; este botón
 *  procesa también las filas `pendiente_suscripcion`.
 *  [159A-2/F1] `dominios` filtra por sección (Exportar por sección); ausente o
 *  vacío = flush global (comportamiento histórico: Sincronización, cabecera). */
export async function flushBdpPush(dominios?: string[]): Promise<BdpPushFlushResumen> {
  const query = (dominios ?? [])
    .map((d) => `dominios=${encodeURIComponent(d)}`)
    .join('&');
  const resp = await customInstance(`/api/bdp/push/flush${query ? `?${query}` : ''}`, {
    method: 'POST',
  }) as { data: BdpPushFlushResumen };
  return resp.data;
}

export async function listarPushFilas(): Promise<BdpPushFila[]> {
  const resp = await customInstance('/api/bdp/push/pendientes', {
    method: 'GET',
  }) as { data: BdpPushFila[] };
  return resp.data;
}

export async function reintentarPushFila(id: string): Promise<BdpPushFlushResumen> {
  const resp = await customInstance(`/api/bdp/push/${id}/reintentar`, {
    method: 'POST',
  }) as { data: BdpPushFlushResumen };
  return resp.data;
}

