/* Peticiones de menús y packs locales (CRUD local, sin BDP). */

import { customInstance } from '../axios-instance';
import type { ActualizarBdpMenuLocalRequest, BdpMenuLocalConLineas, BdpMenuLocalFilters, CrearBdpMenuLocalRequest } from './tipos';

/** Listar menús/packs locales (GET /api/bdp/menus-locales). */
export async function fetchBdpMenusLocales(
  filters: BdpMenuLocalFilters = {},
): Promise<BdpMenuLocalConLineas[]> {
  const params = new URLSearchParams();
  if (filters.tipo) params.set('tipo', filters.tipo);
  if (filters.activo !== undefined) params.set('activo', String(filters.activo));
  if (filters.busqueda) params.set('busqueda', filters.busqueda);
  const query = params.toString();
  const url = query ? `/api/bdp/menus-locales?${query}` : '/api/bdp/menus-locales';
  const resp = await customInstance(url, { method: 'GET' }) as { data: BdpMenuLocalConLineas[] };
  return resp.data;
}

/** Obtener un menú/pack local con sus líneas (GET /api/bdp/menus-locales/:id). */
export async function fetchBdpMenuLocal(id: string): Promise<BdpMenuLocalConLineas> {
  const resp = await customInstance(`/api/bdp/menus-locales/${id}`, {
    method: 'GET',
  }) as { data: BdpMenuLocalConLineas };
  return resp.data;
}

/** Crear un menú/pack local (POST /api/bdp/menus-locales). */
export async function crearBdpMenuLocal(
  req: CrearBdpMenuLocalRequest,
): Promise<BdpMenuLocalConLineas> {
  const resp = await customInstance('/api/bdp/menus-locales', {
    method: 'POST',
    body: JSON.stringify(req),
  }) as { data: BdpMenuLocalConLineas };
  return resp.data;
}

/** Actualizar un menú/pack local (PUT /api/bdp/menus-locales/:id). */
export async function actualizarBdpMenuLocal(
  id: string,
  req: ActualizarBdpMenuLocalRequest,
): Promise<BdpMenuLocalConLineas> {
  const resp = await customInstance(`/api/bdp/menus-locales/${id}`, {
    method: 'PUT',
    body: JSON.stringify(req),
  }) as { data: BdpMenuLocalConLineas };
  return resp.data;
}

/** Eliminar un menú/pack local (DELETE /api/bdp/menus-locales/:id). */
export async function eliminarBdpMenuLocal(id: string): Promise<{ mensaje: string }> {
  const resp = await customInstance(`/api/bdp/menus-locales/${id}`, {
    method: 'DELETE',
  }) as { data: { mensaje: string } };
  return resp.data;
}

