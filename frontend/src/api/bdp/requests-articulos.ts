/* Peticiones BDP de mapeos y stock de artículos, catálogo y departamentos. */

import { customInstance } from '../axios-instance';
import type { AjustarBdpArticleStockRequest, BdpArticleMapItem, BdpArticleStockItem, BdpCatalogoClasificacion, BdpCatalogoTipo, BdpImportDepartamentosResultado, CrearBdpClasificacionRequest } from './tipos';

/** Listar todos los mapeos de artículos Glory → BDP. */
export async function fetchBdpArticleMaps(): Promise<BdpArticleMapItem[]> {
  const resp = await customInstance('/api/bdp/article-maps', {
    method: 'GET',
  }) as { data: BdpArticleMapItem[] };
  return resp.data;
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

export async function fetchBdpCatalogo(tipo: BdpCatalogoTipo): Promise<BdpCatalogoClasificacion[]> {
  const resp = await customInstance(`/api/bdp/catalogo/${tipo}`, {
    method: 'GET',
  }) as { data: BdpCatalogoClasificacion[] };
  return resp.data;
}

export async function crearBdpClasificacion(
  req: CrearBdpClasificacionRequest,
): Promise<BdpCatalogoClasificacion> {
  const resp = await customInstance('/api/bdp/catalogo', {
    method: 'POST',
    body: JSON.stringify(req),
  }) as { data: BdpCatalogoClasificacion };
  return resp.data;
}

/** Lee ExportDepartments del BDP y crea/vincula clasificaciones locales.
 *  Nunca pisa ediciones locales ni escribe en BDP. */
export async function importarDepartamentosBdp(): Promise<BdpImportDepartamentosResultado> {
  const resp = await customInstance('/api/bdp/catalogo/importar-departamentos', {
    method: 'POST',
  }) as { data: BdpImportDepartamentosResultado };
  return resp.data;
}

