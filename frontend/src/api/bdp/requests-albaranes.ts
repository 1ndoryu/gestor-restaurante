/* Peticiones BDP de albaranes de compra (importados y locales) y su conciliación. */

import { customInstance } from '../axios-instance';
import type { ActualizarBdpPurchaseNoteRequest, BdpPurchaseNote, BdpPurchaseNoteFilters, BdpPurchaseNoteReconcileRequest, BdpPurchaseNoteReconcileResult, BdpPurchaseNoteSyncRequest, BdpPurchaseNoteSyncResult, CrearBdpPurchaseNoteRequest } from './tipos';

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

/** Crear un albarán de compra local (F5). */
export async function crearBdpPurchaseNote(
  req: CrearBdpPurchaseNoteRequest,
): Promise<BdpPurchaseNote> {
  const resp = await customInstance('/api/bdp/purchase-notes', {
    method: 'POST',
    body: JSON.stringify(req),
  }) as { data: BdpPurchaseNote };
  return resp.data;
}

/** Actualizar un albarán de compra local (F5). */
export async function actualizarBdpPurchaseNote(
  id: string,
  req: ActualizarBdpPurchaseNoteRequest,
): Promise<BdpPurchaseNote> {
  const resp = await customInstance(`/api/bdp/purchase-notes/${id}`, {
    method: 'PUT',
    body: JSON.stringify(req),
  }) as { data: BdpPurchaseNote };
  return resp.data;
}

/** Eliminar un albarán de compra local (F5). */
export async function eliminarBdpPurchaseNote(id: string): Promise<{ mensaje: string }> {
  const resp = await customInstance(`/api/bdp/purchase-notes/${id}`, {
    method: 'DELETE',
  }) as { data: { mensaje: string } };
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

