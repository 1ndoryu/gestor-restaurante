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

export async function listarConteosInventario(): Promise<BdpConteoInventario[]> {
  const resp = await customInstance('/api/bdp/inventario/conteos', {
    method: 'GET',
  }) as { data: BdpConteoInventario[] };
  return resp.data;
}

export async function crearConteoInventario(req: {
  observaciones?: string;
  idempotency_key?: string;
  articulos: InventarioLineaRequest[];
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

