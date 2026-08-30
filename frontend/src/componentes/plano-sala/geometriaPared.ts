/* [134A-3] Geometría pura de paredes del plano de sala.
 * Separada de usePlanoSala.ts para reducir el hook (protocolo de limite-lineas)
 * y para poder testear la matemática de clamp/snap sin montar React.
 * No depende de estado global ni de React: solo de la forma de la pared. */

/* Información de la pared que la geometría necesita. */
export interface ParedGeometria {
  ancho: number;
  alto: number;
  rotacion: number;
}

/*
 * CLAMP DE PAREDES — REGRESIONES HISTÓRICAS (no repetir):
 *
 * pos_x / pos_y = esquina top-left del rect SIN ROTAR (coords canónicas).
 * CSS aplica rotate(θ) alrededor del centro → la esquina top-left NO
 * representa ningún borde visual real cuando θ ≠ 0.
 *
 *  ❌ Math.max(0, pos_x) directo → "límite horizontal imaginario": una pared
 *     vertical tiene pos_x = centro_x - largo/2 < 0 aunque esté dentro
 *     del plano. El clamp la mueve como si fuera horizontal.
 *  ❌ Sin clamp ninguno → las paredes salen por arriba/izquierda.
 *  ❌ Math.min(zonaW - bbW/2, ...) → bloquea también derecha/abajo, a diferencia
 *     de las mesas que son libres en esa dirección. Inconsistente con el resto.
 *  ✅ Solo Math.max(bbW/2, centro_x) → evita salir por arriba/izquierda,
 *     libre hacia abajo/derecha. Igual que mesas pero sobre el centro rotado.
 *
 * FÓRMULA CORRECTA:
 *  centro visual  = (pos_x + w/2, pos_y + h/2)
 *  bounding box   = { bbW = w|cosθ| + h|sinθ|, bbH = w|sinθ| + h|cosθ| }
 *  clamp centro   = [bbW/2, ∞) × [bbH/2, ∞)    ← sin límite superior
 *  top-left final = centro_clampado - (w/2, h/2)
 */
export function calcularClampPared(
  pared: ParedGeometria,
  posX: number,
  posY: number,
): { x: number; y: number } {
  const { ancho: w, alto: h, rotacion } = pared;
  const rad = (rotacion * Math.PI) / 180;
  const bbW = w * Math.abs(Math.cos(rad)) + h * Math.abs(Math.sin(rad));
  const bbH = w * Math.abs(Math.sin(rad)) + h * Math.abs(Math.cos(rad));
  /* Solo límite inferior (arriba/izquierda): el centro no puede salir del canvas
   * por la parte negativa. Sin límite superior → libres hacia abajo/derecha. */
  const cx = Math.max(bbW / 2, posX + w / 2);
  const cy = Math.max(bbH / 2, posY + h / 2);
  return {
    x: Math.round(cx - w / 2),
    y: Math.round(cy - h / 2),
  };
}

/* Largo mínimo de pared al crear: por debajo se descarta. */
export const LARGO_MINIMO_PARED = 30;

/*
 * [154A-1] Snap a horizontal/vertical: solo se permite dibujar paredes a 0° o 90°.
 * Se elige el eje según si |dx| >= |dy| (horizontal) o viceversa (vertical).
 * La longitud se proyecta sobre ese eje para que el preview sea recto.
 * Devuelve null si el movimiento es menor a 5px (se ignora).
 */
export function calcularPreviewPared(
  startX: number,
  startY: number,
  endX: number,
  endY: number,
): { x: number; y: number; w: number; rotation: number } | null {
  const dx = endX - startX;
  const dy = endY - startY;
  const isHorizontal = Math.abs(dx) >= Math.abs(dy);
  const length = isHorizontal ? Math.abs(dx) : Math.abs(dy);
  if (length < 5) return null;
  const snappedAngle = isHorizontal
    ? (dx >= 0 ? 0 : 180)
    : (dy >= 0 ? 90 : -90);
  const finalX = startX + (isHorizontal ? dx : 0);
  const finalY = startY + (isHorizontal ? 0 : dy);
  const midX = (startX + finalX) / 2;
  const midY = (startY + finalY) / 2;
  return {
    x: midX - length / 2,
    y: midY - 5,
    w: Math.max(length, LARGO_MINIMO_PARED),
    rotation: snappedAngle,
  };
}