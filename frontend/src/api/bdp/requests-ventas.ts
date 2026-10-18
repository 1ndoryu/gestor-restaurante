/* Peticiones BDP de ventas: estado, poll, anulación, propinas, puntos y camarero. */

import { customInstance } from '../axios-instance';
import type { AgregarPropinaRequest, AnularVentaResponse, BdpOrderStatusResponse, BdpPollResponse, PuntoCliente, PuntosClienteResponse, SumarPuntosRequest } from './tipos';

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

/* [128A-1/F4] Anular localmente una venta (D4). El motivo es obligatorio en
 * modalidad credito_completo. Idempotency key protege el doble click (C1). */
export async function anularVenta(
  ventaId: string,
  req: { motivo?: string; idempotency_key?: string },
): Promise<AnularVentaResponse> {
  const resp = await customInstance(`/api/ventas/${ventaId}/anular`, {
    method: 'POST',
    body: JSON.stringify(req),
  }) as { data: AnularVentaResponse };
  return resp.data;
}

export async function agregarPropinaVenta(
  ventaId: string,
  req: AgregarPropinaRequest,
): Promise<Record<string, unknown>> {
  const resp = await customInstance(`/api/ventas/${ventaId}/propina`, {
    method: 'POST',
    body: JSON.stringify(req),
  }) as { data: Record<string, unknown> };
  return resp.data;
}

export async function fetchPuntosCliente(clienteId: string): Promise<PuntosClienteResponse> {
  const resp = await customInstance(`/api/clientes/${clienteId}/puntos`, {
    method: 'GET',
  }) as { data: PuntosClienteResponse };
  return resp.data;
}

export async function sumarPuntosCliente(
  clienteId: string,
  req: SumarPuntosRequest,
): Promise<PuntoCliente> {
  const resp = await customInstance(`/api/clientes/${clienteId}/puntos`, {
    method: 'POST',
    body: JSON.stringify(req),
  }) as { data: PuntoCliente };
  return resp.data;
}

export async function llamarCamarero(mesaId: string): Promise<{ mensaje: string }> {
  const resp = await customInstance(`/api/plano-sala/mesas/${mesaId}/llamar-camarero`, {
    method: 'POST',
  }) as { data: { mensaje: string } };
  return resp.data;
}

