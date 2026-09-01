/* [134A-8] Pared arrastrable — simplificada: solo largo editable, grosor fijo.
 * ancho = largo del bar (editable con handle), alto = grosor (fijo 10).
 * La rotación CSS orienta la pared. El handle de resize proyecta el delta
 * del mouse sobre el eje local del bar (cos/sin de rotación) para que
 * funcione naturalmente en cualquier ángulo. */

import { useRef, useState, useEffect } from 'react';
import type { CSSProperties } from 'react';
import { RotateCw } from 'lucide-react';
import type { ParedSala } from '../../api/generated/gestionRestauranteAPI.schemas';
import type { ActualizarParedRequest } from '../../api/generated';

type DragMode = 'none' | 'move' | 'rotate' | 'resize';
const MIN_LARGO = 20;
const GROSOR = 10;

/* Cursor de resize adaptado al ángulo de la pared.
 * La pared es una barra horizontal — su extremo derecho apunta en dirección (cosθ, sinθ).
 * Mapeamos ese ángulo al cursor CSS más cercano (snap cada 45°). */
function resizeCursorForAngle(deg: number): string {
  const norm = ((deg % 180) + 180) % 180;
  if (norm < 22.5 || norm >= 157.5) return 'ew-resize';
  if (norm < 67.5) return 'nwse-resize';
  if (norm < 112.5) return 'ns-resize';
  return 'nesw-resize';
}

interface Props {
  pared: ParedSala;       /* display coords (× zoom) */
  canonical: ParedSala;   /* canonical coords (sin escala) — usado en callbacks */
  zoom: number;
  seleccionada: boolean;
  onClick: () => void;
  onMoveEnd: (id: string, x: number, y: number) => void;
  onRotateEnd: (id: string, deg: number) => void;
  onResizeEnd: (id: string, req: ActualizarParedRequest) => void;
}

export default function ParedDraggable({
  pared, canonical, zoom, seleccionada, onClick, onMoveEnd, onRotateEnd, onResizeEnd,
}: Props) {
  const divRef = useRef<HTMLDivElement>(null);
  const [, forceUpdate] = useState(0);

  /* Modo activo del drag */
  const mode = useRef<DragMode>('none');
  const startMouse = useRef({ x: 0, y: 0 });
  const startRect = useRef({ x: 0, y: 0, w: 0, h: 0 });

  /* Valores de preview (display coords) */
  const previewX = useRef(pared.pos_x);
  const previewY = useRef(pared.pos_y);
  const previewW = useRef(pared.ancho);
  /* alto = grosor fijo, no se usa como preview editable */
  const previewRot = useRef(canonical.rotacion);

  /* Refs para callbacks y valores que cambian sin reinstalar handlers */
  const canonicalRef = useRef(canonical);
  const zoomRef = useRef(zoom);

  const onMoveEndRef = useRef(onMoveEnd);
  const onRotateEndRef = useRef(onRotateEnd);
  const onResizeEndRef = useRef(onResizeEnd);

  useEffect(() => { canonicalRef.current = canonical; }, [canonical]);
  useEffect(() => { zoomRef.current = zoom; }, [zoom]);

  useEffect(() => { onMoveEndRef.current = onMoveEnd; }, [onMoveEnd]);
  useEffect(() => { onRotateEndRef.current = onRotateEnd; }, [onRotateEnd]);
  useEffect(() => { onResizeEndRef.current = onResizeEnd; }, [onResizeEnd]);

  /* Sincronizar preview cuando llega dato nuevo del servidor (post-guardar) */
  useEffect(() => {
    if (mode.current === 'none') {
      previewX.current = pared.pos_x;
      previewY.current = pared.pos_y;
      previewW.current = pared.ancho;
      previewRot.current = canonical.rotacion;
      forceUpdate(n => n + 1);
    }
  }, [pared.pos_x, pared.pos_y, pared.ancho, canonical.rotacion]);

  /* Todos los modos capturan el pointer en el div padre para recibir pointermove/up
   * incluso si el cursor sale del elemento. */
  const captureOnParent = (e: React.PointerEvent) => {
    divRef.current?.setPointerCapture(e.pointerId);
  };

  const onPointerDownBody = (e: React.PointerEvent) => {
    if (e.shiftKey) return;  /* shift reservado para pan del canvas */
    e.stopPropagation();     /* evita que dnd-kit vea el evento */
    e.preventDefault();
    onClick();
    mode.current = 'move';
    startMouse.current = { x: e.clientX, y: e.clientY };
    startRect.current = { x: previewX.current, y: previewY.current, w: previewW.current, h: 0 };
    captureOnParent(e);
  };

  const onPointerDownRotate = (e: React.PointerEvent) => {
    e.stopPropagation();
    e.preventDefault();
    mode.current = 'rotate';
    captureOnParent(e);
  };

  const onPointerDownResizeE = (e: React.PointerEvent) => {
    e.stopPropagation();
    e.preventDefault();
    mode.current = 'resize';
    startMouse.current = { x: e.clientX, y: e.clientY };
    startRect.current = { x: previewX.current, y: previewY.current, w: previewW.current, h: 0 };
    captureOnParent(e);
  };

  const onPointerMove = (e: React.PointerEvent) => {
    if (mode.current === 'none') return;
    const dx = e.clientX - startMouse.current.x;
    const dy = e.clientY - startMouse.current.y;
    const z = zoomRef.current;

    if (mode.current === 'move') {
      /* [134A-11] Sin maxX/maxY — las paredes se mueven libremente igual que las mesas.
       * El canvas content puede ser más grande que zonaData.ancho/alto cuando hay elementos
       * fuera de los límites de zona, así que no hay razón para restringir artificialmente. */
      previewX.current = startRect.current.x + dx;
      previewY.current = startRect.current.y + dy;
    } else if (mode.current === 'rotate' && divRef.current) {
      const rect = divRef.current.getBoundingClientRect();
      const cx = rect.left + rect.width / 2;
      const cy = rect.top + rect.height / 2;
      let angle = Math.atan2(e.clientY - cy, e.clientX - cx) * (180 / Math.PI) + 90;
      angle = Math.round(angle / 15) * 15;
      previewRot.current = ((angle % 360) + 360) % 360;
    } else if (mode.current === 'resize') {
      /* [134A-9] Proyectar delta del mouse sobre el eje local del bar (cos θ, sin θ).
       * CSS rotate(θ) gira alrededor del centro — si solo cambiamos width el centro
       * se desplaza y la pared se mueve. Compensamos pos para anclar el extremo izquierdo:
       *   deltaW = newW - startW
       *   newLeft = startLeft - deltaW/2 × (1 - cos θ)
       *   newTop  = startTop  + deltaW/2 × sin θ */
      const rad = (previewRot.current * Math.PI) / 180;
      const cosR = Math.cos(rad);
      const sinR = Math.sin(rad);
      const proj = dx * cosR + dy * sinR;
      const newW = Math.max(MIN_LARGO * z, startRect.current.w + proj);
      const deltaW = newW - startRect.current.w;
      previewW.current = newW;
      previewX.current = startRect.current.x - (deltaW / 2) * (1 - cosR);
      previewY.current = startRect.current.y + (deltaW / 2) * sinR;
    }
    forceUpdate(n => n + 1);
  };

  const onPointerUp = () => {
    if (mode.current === 'none') return;
    const z = zoomRef.current;
    const c = canonicalRef.current;
    const m = mode.current;
    mode.current = 'none';

    if (m === 'move') {
      onMoveEndRef.current(c.id, Math.round(previewX.current / z), Math.round(previewY.current / z));
    } else if (m === 'rotate') {
      onRotateEndRef.current(c.id, previewRot.current);
    } else if (m === 'resize') {
      onResizeEndRef.current(c.id, {
        pos_x: Math.round(previewX.current / z),
        pos_y: Math.round(previewY.current / z),
        ancho: Math.round(previewW.current / z),
        alto: GROSOR,
        rotacion: previewRot.current,
      });
    }
    forceUpdate(n => n + 1);
  };

  return (
    <div
      ref={divRef}
      className="planoPared"
      style={{
        '--x': `${previewX.current}px`,
        '--y': `${previewY.current}px`,
        '--w': `${previewW.current}px`,
        '--h': `${GROSOR * zoom}px`,
        '--rot': previewRot.current ? `rotate(${previewRot.current}deg)` : 'none',
        '--sombra': seleccionada ? '0 0 0 2px hsl(var(--primary))' : 'none',
        '--cursor': mode.current === 'move' ? 'grabbing' : mode.current === 'resize' ? resizeCursorForAngle(previewRot.current) : 'grab',
      } as CSSProperties}
      onPointerDown={onPointerDownBody}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onClick={e => e.stopPropagation()}
    >
      {seleccionada && (
        <>
          {/* Handle de rotación — encima del centro */}
          <div
            className="planoParedHandle planoParedHandleRotar absolute -top-7 left-1/2 -translate-x-1/2 w-5 h-5 rounded-full bg-primary flex items-center justify-center shadow-md hover:scale-110 transition-transform"
            onPointerDown={onPointerDownRotate}
            title="Arrastrar para rotar (snap 15°)"
          >
            <RotateCw className="size-3 text-primary-foreground pointer-events-none" />
          </div>
          {/* Handle resize largo (extremo derecho del bar) */}
          <div
            className="planoParedHandle planoParedHandleResize absolute top-1/2 -right-1 -translate-y-1/2 w-2 h-5 rounded-sm bg-primary/80 hover:bg-primary"
            style={{ '--cursorResize': resizeCursorForAngle(previewRot.current) } as CSSProperties}
            onPointerDown={onPointerDownResizeE}
            title="Arrastrar para cambiar largo"
          />
        </>
      )}
    </div>
  );
}



