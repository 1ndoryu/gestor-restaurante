/* Peticiones BDP de inventario: registro de conteos y su consulta. */

import { customInstance } from '../axios-instance';
import type { BdpConteoInventario, BdpConteoInventarioLinea, ConteoInventarioCreado, InventarioLineaRequest, RegistrarInventarioRequest, RegistrarInventarioResponse } from './tipos';

export async function registrarInventario(
  req: RegistrarInventarioRequest,
): Promise<RegistrarInventarioResponse> {
  const resp = await customInstance('/api/bdp/inventario', {
    method: 'POST',
    body: JSON.stringify(req),
  }) as { data: RegistrarInventarioResponse };
  return resp.data;
}

/** [L7] `warehouse_id` filtra el historial: sin él, todos los almacenes. */
export async function listarConteosInventario(
  warehouseId?: string,
): Promise<BdpConteoInventario[]> {
  const consulta = warehouseId ? `?warehouse_id=${encodeURIComponent(warehouseId)}` : '';
  const resp = await customInstance(`/api/bdp/inventario/conteos${consulta}`, {
    method: 'GET',
  }) as { data: BdpConteoInventario[] };
  return resp.data;
}

export async function crearConteoInventario(req: {
  observaciones?: string;
  idempotency_key?: string;
  articulos: InventarioLineaRequest[];
  /** [L7] Almacén del conteo; ausente es el agregado local «General» (0). */
  warehouse_id?: string;
}): Promise<ConteoInventarioCreado> {
  const resp = await customInstance('/api/bdp/inventario/conteos', {
    method: 'POST',
    body: JSON.stringify(req),
  }) as { data: ConteoInventarioCreado };
  return resp.data;
}

export async function obtenerConteoInventario(id: string): Promise<{
  conteo: BdpConteoInventario;
  lineas: BdpConteoInventarioLinea[];
}> {
  const resp = await customInstance(`/api/bdp/inventario/conteos/${id}`, {
    method: 'GET',
  }) as { data: { conteo: BdpConteoInventario; lineas: BdpConteoInventarioLinea[] } };
  return resp.data;
}

