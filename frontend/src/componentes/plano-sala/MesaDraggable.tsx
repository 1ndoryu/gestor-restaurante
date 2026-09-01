/* [263A-14] Mesa arrastrable con pointer events nativos — sin dnd-kit transform.
 * [134A-21] Reescrito con refs + forceUpdate (patrón ParedDraggable) para eliminar
 * el flicker causado por el desfase entre el reset del CSS transform de dnd-kit
 * y el update de posicionesLocales (doble-delta por un frame).
 * Resize via useMesaResize mantiene su propio previewRect state. */

import { useRef, useState, useEffect } from 'react';
import type { CSSProperties } from 'react';
import type { ActualizarMesaRequest, Mesa } from '../../api/generated';
import { useMesaResize } from '../../hooks/useMesaResize';

/* Threshold en px para distinguir click de drag */
const CLICK_THRESHOLD = 4;

interface MesaDraggableProps {
  mesa: Mesa;       /* coords ya multiplicadas por zoom */
  seleccionada: boolean;
  zoom: number;
  zonaAncho: number;
  zonaAlto: number;
  onResize: (id: string, data: ActualizarMesaRequest) => void | Promise<void>;
  onClick: () => void;
  onMoveEnd: (id: string, canonicalX: number, canonicalY: number) => void;
}

function MesaDraggable({
  mesa, seleccionada, zoom, zonaAncho, zonaAlto, onResize, onClick, onMoveEnd,
}: MesaDraggableProps) {
  const divRef = useRef<HTMLDivElement>(null);
  const [, forceUpdate] = useState(0);

  /* Estado de drag — refs para evitar re-renders innecesarios durante panning */
  const dragging = useRef(false);
  const hasMoved = useRef(false);
  const startMouse = useRef({ x: 0, y: 0 });
  const startPos = useRef({ x: 0, y: 0 });

  /* Posición visual en px escalados — fuente de verdad durante el drag */
  const previewX = useRef(mesa.pos_x);
  const previewY = useRef(mesa.pos_y);

  /* Refs estables para valores que cambian sin reinstalar handlers */
  const zoomRef = useRef(zoom);
  const onMoveEndRef = useRef(onMoveEnd);
  const onClickRef = useRef(onClick);

  useEffect(() => { zoomRef.current = zoom; }, [zoom]);
  useEffect(() => { onMoveEndRef.current = onMoveEnd; }, [onMoveEnd]);
  useEffect(() => { onClickRef.current = onClick; }, [onClick]);

  /* Sincronizar preview desde el padre cuando no hay drag activo
   * (post refetch o cambio de posicionesLocales). */
  useEffect(() => {
    if (!dragging.current) {
      previewX.current = mesa.pos_x;
      previewY.current = mesa.pos_y;
    }
  }, [mesa.pos_x, mesa.pos_y]);

  const { previewRect, resizing, onResizeStart } = useMesaResize({
    rect: { x: mesa.pos_x, y: mesa.pos_y, width: mesa.ancho, height: mesa.alto },
    bounds: { width: zonaAncho * zoom, height: zonaAlto * zoom },
    zoom,
    onCommit: (rect) => onResize(mesa.id, {
      pos_x: Math.round(rect.x / zoom),
      pos_y: Math.round(rect.y / zoom),
      ancho: Math.round(rect.width / zoom),
      alto: Math.round(rect.height / zoom),
    }),
  });

  const onPointerDown = (e: React.PointerEvent) => {
    /* shift reservado para pan del canvas; solo botón izquierdo; no interferir con resize */
    if (e.shiftKey || e.button !== 0 || resizing) return;
    e.stopPropagation();
    /* preventDefault suprime el click event del browser → evita doble-disparo */
    e.preventDefault();
    dragging.current = true;
    hasMoved.current = false;
    startMouse.current = { x: e.clientX, y: e.clientY };
    startPos.current = { x: previewX.current, y: previewY.current };
    divRef.current?.setPointerCapture(e.pointerId);
    forceUpdate(n => n + 1);
  };

  const onPointerMove = (e: React.PointerEvent) => {
    if (!dragging.current) return;
    const dx = e.clientX - startMouse.current.x;
    const dy = e.clientY - startMouse.current.y;
    if (!hasMoved.current && (Math.abs(dx) > CLICK_THRESHOLD || Math.abs(dy) > CLICK_THRESHOLD)) {
      hasMoved.current = true;
    }
    if (!hasMoved.current) return;
    /* [134A-21] Sin maxX/maxY — las mesas se mueven libremente por el canvas
     * (mismo patrón que ParedDraggable). El clamp a zonaAncho/zonaAlto creaba
     * un "límite imaginario" porque el canvas visible (contentBounds) puede ser
     * mayor que la zona. Solo evitamos coordenadas negativas. */
    previewX.current = Math.max(0, startPos.current.x + dx);
    previewY.current = Math.max(0, startPos.current.y + dy);
    forceUpdate(n => n + 1);
  };

  const onPointerUp = () => {
    if (!dragging.current) return;
    dragging.current = false;
    if (!hasMoved.current) {
      /* Click sin movimiento: delegar al handler del padre (seleccionar o borrar) */
      onClickRef.current();
    } else {
      const z = zoomRef.current;
      onMoveEndRef.current(
        mesa.id,
        Math.round(previewX.current / z),
        Math.round(previewY.current / z),
      );
    }
    forceUpdate(n => n + 1);
  };

  /* Durante drag: refs. Durante resize: previewRect. */
  const displayX = resizing ? previewRect.x : previewX.current;
  const displayY = resizing ? previewRect.y : previewY.current;
  const displayW = resizing ? previewRect.width : mesa.ancho;
  const displayH = resizing ? previewRect.height : mesa.alto;

  const clases = [
    'planoMesa',
    mesa.forma,
    !mesa.activa ? 'inactiva' : '',
    seleccionada ? 'seleccionada' : '',
    dragging.current && hasMoved.current ? 'arrastrando' : '',
  ].filter(Boolean).join(' ');

  return (
    <div
      ref={divRef}
      className={clases}
      style={{
        '--x': `${displayX}px`,
        '--y': `${displayY}px`,
        '--w': `${displayW}px`,
        '--h': `${displayH}px`,
      } as CSSProperties}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      /* [134A-28] stopPropagation en click nativo: onPointerUp llama onClick() que
       * selecciona la mesa, pero el browser luego dispara un click event que burbujea
       * hasta el viewport → onViewportClick con tool='select' llama setMesaSeleccionada(null)
       * → deselección inmediata. Mismo patrón que ParedDraggable line 199. */
      onClick={e => e.stopPropagation()}
    >
      <span className="planoMesaNumero">{mesa.numero}</span>
      <span className="planoMesaCapacidad">
        {mesa.min_personas}-{mesa.max_personas}
      </span>
      {seleccionada && (
        <>
          {/* onPointerDown en handles detiene propagación para no iniciar drag en el padre */}
          <div className="planoMesaResizeHandle norte" onMouseDown={onResizeStart('n')} onPointerDown={(e) => e.stopPropagation()} role="separator" aria-label="Redimensionar alto desde arriba" />
          <div className="planoMesaResizeHandle sur" onMouseDown={onResizeStart('s')} onPointerDown={(e) => e.stopPropagation()} role="separator" aria-label="Redimensionar alto desde abajo" />
          <div className="planoMesaResizeHandle este" onMouseDown={onResizeStart('e')} onPointerDown={(e) => e.stopPropagation()} role="separator" aria-label="Redimensionar ancho desde la derecha" />
          <div className="planoMesaResizeHandle oeste" onMouseDown={onResizeStart('w')} onPointerDown={(e) => e.stopPropagation()} role="separator" aria-label="Redimensionar ancho desde la izquierda" />
        </>
      )}
    </div>
  );
}

export default MesaDraggable;

