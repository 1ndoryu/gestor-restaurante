/* [263A-16] Vista de ocupación del plano de sala — solo lectura.
 * Reescrito imports a shadcn Button. Mantiene PlanoOcupacion.css para canvas/mesas.
 * Mesas coloreadas: libre (gris), ocupada (verde), no-show (rojo), inactiva.
 * Se integra dentro de ListaReservas. Click en mesa muestra reservas en tooltip.
 * [283A-36] Zoom sincronizado con PlanoSala via zoomStore.
 * [303A-12] Pan, minimap, indicadores off-screen. */

import { useState, useMemo, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { ZoomIn, ZoomOut } from 'lucide-react';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { useObtenerOcupacion, MesaOcupacion, ZonaOcupacion, ParedSala } from '../api/generated';
import { useZoomStore } from '../stores/zoomStore';
import { useCanvasResize } from '../hooks/useCanvasResize';
import { useCanvasPan } from '../hooks/useCanvasPan';
import { useViewportSize } from '../hooks/useViewportSize';
import CanvasMinimap from './plano-sala/CanvasMinimap';
import OffScreenIndicators from './plano-sala/OffScreenIndicators';
import '../estilos/PlanoOcupacion.css';

interface Props {
  fecha: string;
  turno: string;
}

function estadoMesa(mesa: MesaOcupacion): 'libre' | 'ocupada' | 'no_show' | 'inactiva' {
  if (!mesa.activa) return 'inactiva';
  if (mesa.reservas.length === 0) return 'libre';
  if (mesa.reservas.some((r) => r.estado === 'no_show')) return 'no_show';
  return 'ocupada';
}

function PlanoOcupacion({ fecha, turno }: Props) {
  const { data, isLoading } = useObtenerOcupacion(
    { fecha, turno: turno || undefined },
    { query: { enabled: !!fecha } }
  );

  const plano = data?.status === 200 ? data.data : null;
  const [zonaActiva, setZonaActiva] = useState<string | null>(null);
  const [mesaHover, setMesaHover] = useState<string | null>(null);
  const zoom = useZoomStore(s => s.zoom);
  const zoomIn = useZoomStore(s => s.zoomIn);
  const zoomOut = useZoomStore(s => s.zoomOut);
  const canvasHeight = useZoomStore(s => s.canvasHeight);
  const setCanvasHeight = useZoomStore(s => s.setCanvasHeight);

  /* [303A-13] Resize del canvas arrastrando el borde inferior — sincronizado via store */
  const { resizing, onResizeStart } = useCanvasResize({ canvasHeight, setCanvasHeight });

  const zonaData = plano?.zonas.find((z: ZonaOcupacion) => z.id === zonaActiva);

  /* [313A-13] Medición compartida del viewport.
   * PlanoOcupacion y PlanoSala usan el mismo hook para evitar divergencias: si el viewport
   * se renderiza condicionalmente, el hook espera al layout real antes de fijar su tamaño. */
  const { viewportRef, viewportSize } = useViewportSize(Boolean(zonaData), [canvasHeight, zonaActiva]);

  useEffect(() => {
    if (!plano || plano.zonas.length === 0) {
      if (zonaActiva !== null) setZonaActiva(null);
      return;
    }
    if (!zonaActiva || !plano.zonas.some((zona) => zona.id === zonaActiva)) {
      setZonaActiva(plano.zonas[0].id);
    }
  }, [plano, zonaActiva]);

  /* [313A-10] contentBounds = misma fórmula que PlanoSala.
   * Base: zonaData.ancho/alto (el "mundo" de diseño del plano).
   * Extensión: si alguna mesa se sale de la zona, el mundo crece.
   * SIN canvasHeight (eso era el bug original — igualaba content a viewport
   * haciendo que el rect del minimap cubriera 100% del alto y fuera invisible).
   * SIN calcular solo bounding box de mesas (eso hacía que el "mundo" fuera
   * diferente al de PlanoSala, desproporcionando el rect del viewport).
   * Ver Agente/planes/plan-minimap-viewport-rect-2026-03-31.md para historial. */
  const contentBounds = useMemo(() => {
    if (!zonaData) return { w: 0, h: 0 };
    let maxX = zonaData.ancho * zoom;
    let maxY = zonaData.alto * zoom;
    for (const m of zonaData.mesas) {
      const x = (m.pos_x + m.ancho) * zoom;
      const y = (m.pos_y + m.alto) * zoom;
      if (x > maxX) maxX = x;
      if (y > maxY) maxY = y;
    }
    /* [134A-12] Incluir paredes en contentBounds */
    for (const p of (zonaData.paredes ?? [])) {
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
  }, [zonaData, zoom]);

  const maxPanOffset = useMemo(() => ({
    x: Math.max(0, contentBounds.w - viewportSize.w),
    y: Math.max(0, contentBounds.h - viewportSize.h),
  }), [contentBounds.h, contentBounds.w, viewportSize.h, viewportSize.w]);

  /* [303A-12] Pan del canvas
   * [303A-20] Transform-based + clamp al área real del plano. */
  const { panning, panOffset, setPanOffset, onPanMouseDown } = useCanvasPan(maxPanOffset);

  if (isLoading) return <p className="text-sm text-muted-foreground text-center py-4">Cargando plano...</p>;
  if (!plano || plano.zonas.length === 0) return null;

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <h3 className="font-semibold text-sm">Plano de sala</h3>
        <div className="flex items-center gap-1">
          <TooltipButton size="sm" variant="outline" onClick={zoomOut} tooltip="Alejar (zoom −10%)"><ZoomOut className="size-4" /></TooltipButton>
          <span className="text-xs w-12 text-center tabular-nums">{Math.round(zoom * 100)}%</span>
          <TooltipButton size="sm" variant="outline" onClick={zoomIn} tooltip="Acercar (zoom +10%)"><ZoomIn className="size-4" /></TooltipButton>
        </div>
      </div>

      {/* Tabs de zonas */}
      <div className="flex gap-1 flex-wrap">
        {plano.zonas.map((z: ZonaOcupacion) => (
          <Button
            key={z.id}
            variant={z.id === zonaActiva ? 'default' : 'outline'}
            size="sm"
            onClick={() => { setZonaActiva(z.id); setMesaHover(null); }}
          >
            {z.nombre}
          </Button>
        ))}
      </div>

      {/* Leyenda */}
      <div className="flex gap-4 text-xs text-muted-foreground">
        <span className="flex items-center gap-1"><span className="inline-block w-3 h-3 rounded-sm bg-muted" /> Libre</span>
        <span className="flex items-center gap-1"><span className="inline-block w-3 h-3 rounded-sm bg-green-500" /> Ocupada</span>
        <span className="flex items-center gap-1"><span className="inline-block w-3 h-3 rounded-sm bg-destructive" /> No Show</span>
        <span className="flex items-center gap-1"><span className="inline-block w-3 h-3 rounded-sm bg-muted-foreground/30" /> Inactiva</span>
      </div>

      {/* [303A-20] Canvas con overflow:hidden + transform para pan. */}
      {zonaData && (
        <div style={{ position: 'relative' }}>
          <div
            ref={viewportRef}
            className={`planoOcupacionCanvas ${panning ? 'planoPanning' : ''}`}
            style={{ height: canvasHeight, width: '100%' }}
            onMouseDown={onPanMouseDown}
          >
            <div
              className="planoOcupacionContent"
              style={{
                width: contentBounds.w,
                height: contentBounds.h,
                transform: `translate(${-panOffset.x}px, ${-panOffset.y}px)`,
              }}
            >
              {/* [134A-12] Paredes read-only visibles en reservas */}
              {(zonaData.paredes ?? []).map((pared: ParedSala) => (
                <div
                  key={pared.id}
                  className="absolute"
                  style={{
                    left: pared.pos_x * zoom,
                    top: pared.pos_y * zoom,
                    width: pared.ancho * zoom,
                    height: pared.alto * zoom,
                    background: 'var(--background)',
                    border: '2px solid var(--border)',
                    borderRadius: 'var(--radius)',
                    transform: pared.rotacion ? `rotate(${pared.rotacion}deg)` : undefined,
                    transformOrigin: 'center center',
                    pointerEvents: 'none',
                    zIndex: 0,
                  }}
                />
              ))}
              {zonaData.mesas.map((mesa: MesaOcupacion) => {
                const estado = estadoMesa(mesa);
                const esHover = mesaHover === mesa.id;

                return (
                  <div
                    key={mesa.id}
                    className={`mesaOcupacion ${estado} ${mesa.forma} ${esHover ? 'hover' : ''}`}
                    style={{
                      left: mesa.pos_x * zoom,
                      top: mesa.pos_y * zoom,
                      width: mesa.ancho * zoom,
                      height: mesa.alto * zoom,
                    }}
                    onMouseEnter={() => setMesaHover(mesa.id)}
                    onMouseLeave={() => setMesaHover(null)}
                  >
                    <span className="mesaOcupacionNumero">{mesa.numero}</span>
                  </div>
                );
              })}
            </div>
          </div>

          {/* [014A-10] Tooltip position:fixed con coordenadas de pantalla reales.
           * position:fixed escapa de overflow:hidden en cualquier ancestro — el tooltip
           * nunca se recorta por el límite del plano ni por contenedores padre.
           * Las coordenadas se calculan sumando el rect del viewport al offset lógico. */}
          {mesaHover && (() => {
            const mesa = zonaData.mesas.find((m: MesaOcupacion) => m.id === mesaHover);
            if (!mesa || mesa.reservas.length === 0) return null;
            const canvasRect = viewportRef.current?.getBoundingClientRect();
            const originX = canvasRect?.left ?? 0;
            const originY = canvasRect?.top ?? 0;
            const tooltipLeft = originX + mesa.pos_x * zoom - panOffset.x + (mesa.ancho * zoom) / 2;
            const tooltipTop = originY + mesa.pos_y * zoom - panOffset.y;
            return (
              <div
                className="mesaOcupacionTooltip"
                style={{ left: tooltipLeft, top: tooltipTop }}
              >
                {mesa.reservas.map((r) => (
                  <div key={r.reserva_id} className="mesaOcupacionReserva">
                    <strong>{r.hora.slice(0, 5)}</strong>{' '}
                    {r.nombre_cliente} {r.apellidos_cliente}
                    <span className="mesaOcupacionPersonas">{r.num_personas} pers.</span>
                  </div>
                ))}
              </div>
            );
          })()}

          {/* [303A-12] Minimap + indicadores off-screen */}
          <CanvasMinimap
            mesas={zonaData.mesas.map((m: MesaOcupacion) => ({
              x: m.pos_x * zoom,
              y: m.pos_y * zoom,
              ancho: m.ancho * zoom,
              alto: m.alto * zoom,
            }))}
            paredes={(zonaData.paredes ?? []).map((p: ParedSala) => ({
              x: p.pos_x * zoom,
              y: p.pos_y * zoom,
              ancho: p.ancho * zoom,
              alto: p.alto * zoom,
              rotacion: p.rotacion,
            }))}
            contentWidth={contentBounds.w}
            contentHeight={contentBounds.h}
            viewportWidth={viewportSize.w}
            viewportHeight={viewportSize.h}
            scrollLeft={panOffset.x}
            scrollTop={panOffset.y}
            onNavigate={(x, y) => setPanOffset({ x, y })}
          />
          <OffScreenIndicators
            mesas={zonaData.mesas.map((m: MesaOcupacion) => ({
              x: m.pos_x * zoom,
              y: m.pos_y * zoom,
              ancho: m.ancho * zoom,
              alto: m.alto * zoom,
            }))}
            viewportWidth={viewportSize.w}
            viewportHeight={viewportSize.h}
            scrollLeft={panOffset.x}
            scrollTop={panOffset.y}
            onNavigate={(x, y) => setPanOffset({ x, y })}
          />
        </div>
      )}
      {/* [303A-13] Handle de resize — misma clase CSS que PlanoSala */}
      {zonaData && (
        <div
          className={`planoResizeHandle ${resizing ? 'activo' : ''}`}
          onMouseDown={onResizeStart}
          title="Arrastrar para cambiar altura del plano"
        />
      )}
    </div>
  );
}

export default PlanoOcupacion;
