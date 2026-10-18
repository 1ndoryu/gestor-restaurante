/* Tamaño del contenido del plano (zona, mesas y paredes). Las paredes entran por la caja
 * envolvente de su rectángulo rotado. No incluye canvasHeight: eso es el viewport. */
import type { EstadoPlanoSala } from './tipoEstadoPlano';

type ParamsContenido = Pick<
  EstadoPlanoSala,
  'zonaData' | 'mesasZona' | 'paredesZona' | 'posicionesLocales' | 'dimensionesLocales' | 'zoom'
>;

export function calcularContentBounds({
  zonaData,
  mesasZona,
  paredesZona,
  posicionesLocales,
  dimensionesLocales,
  zoom,
}: ParamsContenido) {
  let maxX = (zonaData?.ancho ?? 0) * zoom;
  let maxY = (zonaData?.alto ?? 0) * zoom;
  for (const m of mesasZona) {
    const p = posicionesLocales[m.id];
    const d = dimensionesLocales[m.id];
    const x = ((p?.x ?? m.pos_x) + (d?.ancho ?? m.ancho)) * zoom;
    const y = ((p?.y ?? m.pos_y) + (d?.alto ?? m.alto)) * zoom;
    if (x > maxX) maxX = x;
    if (y > maxY) maxY = y;
  }
  /* [134A-12] Incluir paredes en contentBounds para que el minimap y el canvas
   * las consideren. Usamos la bounding box del rect rotado. */
  for (const p of paredesZona) {
    const rad = (p.rotacion * Math.PI) / 180;
    const w = p.ancho * zoom;
    const h = p.alto * zoom;
    const bbW = w * Math.abs(Math.cos(rad)) + h * Math.abs(Math.sin(rad));
    const bbH = w * Math.abs(Math.sin(rad)) + h * Math.abs(Math.cos(rad));
    const cx = p.pos_x * zoom + w / 2;
    const cy = p.pos_y * zoom + h / 2;
    const ex = cx + bbW / 2;
    const ey = cy + bbH / 2;
    if (ex > maxX) maxX = ex;
    if (ey > maxY) maxY = ey;
  }
  return { w: maxX, h: maxY };
}
