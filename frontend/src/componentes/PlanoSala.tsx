/* [263A-16] PlanoSala — Constructor visual de plano de sala con drag-and-drop.
 * [263A-28] Diálogos nativos reemplazados por shadcn Dialog + toast.
 * [303A-11] Canvas con altura fija del store, overflow:auto.
 * [303A-12] Pan (shift+drag), minimap, indicadores off-screen.
 * [303A-13] Resize handle arrastrable en borde inferior.
 * Lógica en usePlanoSala, mesa arrastrable en MesaDraggable, config en PanelConfigMesa. */

import { useRef, useMemo, useState } from 'react';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Pencil, Trash2, Download, Upload, ZoomIn, ZoomOut, RefreshCw } from 'lucide-react';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { useSyncTables } from '../api/generated/bdp-mapeos/bdp-mapeos';
import MesaDraggable from './plano-sala/MesaDraggable';
import PanelConfigMesa from './plano-sala/PanelConfigMesa';
import PanelConfigPared from './plano-sala/PanelConfigPared';
import ParedDraggable from './plano-sala/ParedDraggable';
import CanvasToolbar from './plano-sala/CanvasToolbar';
import CombinacionesList from './plano-sala/CombinacionesList';
import PlanoOverlays from './plano-sala/PlanoOverlays';
import PlanoDialogs from './plano-sala/PlanoDialogs';
import { usePlanoSala } from './plano-sala/usePlanoSala';
import { useCanvasResize } from '../hooks/useCanvasResize';
import { useCanvasPan } from '../hooks/useCanvasPan';
import { useCanvasViewport } from '../hooks/useCanvasViewport';
import { useViewportSize } from '../hooks/useViewportSize';
import { useZoomStore } from '../stores/zoomStore';
import '../estilos/PlanoSala.css';

function PlanoSala() {
  const canvasRef = useRef<HTMLDivElement>(null);
  const setCanvasHeight = useZoomStore(s => s.setCanvasHeight);
  const queryClient = useQueryClient();
  const [sincronizandoMesas, setSincronizandoMesas] = useState(false);
  const [dialogoSyncBdp, setDialogoSyncBdp] = useState(false);
  const [confirmacionSyncBdp, setConfirmacionSyncBdp] = useState('');
  const [previewSyncBdp, setPreviewSyncBdp] = useState<{ salones_bdp: number; zonas_creadas: number; mesas_creadas: number; applied: boolean } | null>(null);

  const syncTablesMutation = useSyncTables({
    mutation: {
      onSuccess: (resp) => {
        const d = (resp as unknown as { data: { salones_bdp: number; zonas_creadas: number; mesas_creadas: number; applied: boolean } }).data;
        setPreviewSyncBdp(d);
        if (d.applied) {
          toast.success(`Importación local aplicada: ${d.mesas_creadas} mesas en ${d.zonas_creadas} zonas`);
          queryClient.invalidateQueries({ queryKey: ['/api/plano-sala'] });
          setDialogoSyncBdp(false);
        } else {
          toast.success('Previsualización completada sin cambios');
        }
      },
      onError: () => toast.error('Error al sincronizar mesas desde BDP'),
      onSettled: () => setSincronizandoMesas(false),
    },
  });

  const {
    plano, zonaActiva, zonaData, mesasZona, paredesZona, mesaSeleccionada,
    paredSeleccionada, setParedSeleccionada,
    posicionesLocales, dimensionesLocales, setMesaSeleccionada, cambiarZona, zoom, setZoom,
    canvasHeight,
    activeTool, setActiveTool,
    wallDrawStart, wallDrawPreview,
    handleCrearMesaRapida, handleEliminarMesaDirecta, handleEliminarParedDirecta,
    handleWallDrawStart, handleWallDrawMove, handleWallDrawEnd,
    handleCrearZona, handleEliminarZona, handleEditarZona,
    handleGuardarMesa, handleResizeMesa, handleEliminarMesa,
    handleEliminarPared, handleGuardarPared,
    handleMoverPared, handleRotarPared, handleRedimensionarPared,
    handleDuplicarPared, handleDuplicarMesa,
    handleMoverMesa,
    handleExportar, handleImportar,
    handleCrearCombinacion, handleEliminarCombinacion,
    dialogoEntrada, setDialogoEntrada,
    dialogoConfirmar, setDialogoConfirmar,
    dialogoCombinacion, setDialogoCombinacion,
  } = usePlanoSala();

  /* [313A-13] PlanoSala sufría la misma clase de bug que reservas: el viewport se mide
   * sobre un nodo que aparece condicionalmente cuando existe zonaData. Con deps=[] el
   * effect podía correr con ref=null y dejar viewportSize en 0x0 para todo el ciclo.
   * Se unifica con el hook compartido para que ambos minimaps sigan exactamente la
   * misma estrategia de medición. */
  const { viewportRef, viewportSize } = useViewportSize(Boolean(zonaData), [canvasHeight, zonaActiva]);

  /* [313A-1] El tamaño real del plano sale de la zona y de la mesa más lejana.
   * [313A-2] contentBounds NO incluye canvasHeight — ese es el viewport, no el contenido.
   * Incluirlo inflaba las proporciones del minimap y hacía que el viewport rect
   * cubriera 100% del alto, volviéndose invisible. */
  const contentBounds = useMemo(() => {
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
  }, [dimensionesLocales, mesasZona, paredesZona, posicionesLocales, zonaData, zoom]);

  const maxPanOffset = useMemo(() => ({
    x: Math.max(0, contentBounds.w - viewportSize.w),
    y: Math.max(0, contentBounds.h - viewportSize.h),
  }), [contentBounds.h, contentBounds.w, viewportSize.h, viewportSize.w]);

  /* [303A-13] Resize del canvas arrastrando el borde inferior */
  const { resizing, onResizeStart } = useCanvasResize({ canvasHeight, setCanvasHeight });

  /* [303A-12] Pan del canvas shift+drag o middle-click
   * [303A-20] Transform-based: panOffset state en vez de scrollLeft/scrollTop */
  const { panning, panOffset, setPanOffset, onPanMouseDown } = useCanvasPan(maxPanOffset, activeTool === 'pan');

  /* [134A-15+16] Handlers del viewport delegados a useCanvasViewport */
  /* [134A-21] sensors/DndContext eliminados — mesa usa pointer events nativos. */
  const { onViewportClick, onViewportMouseDown, onViewportMouseMove } = useCanvasViewport({
    activeTool, wallDrawStart, zonaActiva, panOffset, zoom,
    setMesaSeleccionada, setParedSeleccionada,
    handleCrearCombinacion, handleCrearMesaRapida,
    onPanMouseDown, handleWallDrawStart, handleWallDrawMove, handleWallDrawEnd,
  });

  return (
    <div className="flex flex-col gap-4">
      {/* [134A-15] Toolbar simplificado: zona + export/import + zoom. Herramientas en toolbar flotante */}
      <div className="flex items-center gap-2 flex-wrap">
        {zonaActiva && (
          <>
            <TooltipButton size="sm" variant="ghost" onClick={handleEditarZona} tooltip="Cambiar el nombre de la zona activa"><Pencil className="size-4 mr-1" />Renombrar</TooltipButton>
            <TooltipButton size="sm" variant="destructive" onClick={handleEliminarZona} tooltip="Eliminar la zona activa y todas sus mesas. No se puede deshacer."><Trash2 className="size-4 mr-1" />Zona</TooltipButton>
          </>
        )}
        <div className="ml-auto flex gap-2">
          <TooltipButton size="sm" variant="ghost" onClick={handleExportar} tooltip="Exportar el plano de sala a un archivo JSON"><Download className="size-4 mr-1" />Exportar</TooltipButton>
          <TooltipButton size="sm" variant="ghost" onClick={handleImportar} tooltip="Importar un plano de sala desde un archivo JSON"><Upload className="size-4 mr-1" />Importar</TooltipButton>
          <TooltipButton size="sm" variant="outline" onClick={() => { setDialogoSyncBdp(true); setPreviewSyncBdp(null); setConfirmacionSyncBdp(''); }} disabled={sincronizandoMesas} tooltip="Importar salones y mesas desde BDP. Crea solo lo que falte, no modifica lo existente.">
            {sincronizandoMesas ? <RefreshCw className="size-4 mr-1 animate-spin" /> : <RefreshCw className="size-4 mr-1" />}
            Sync BDP
          </TooltipButton>
        </div>
        <div className="flex items-center gap-1">
          <TooltipButton size="sm" variant="outline" onClick={() => setZoom(z => Math.max(0.5, +(z - 0.1).toFixed(2)))} tooltip="Alejar (zoom −10%)"><ZoomOut className="size-4" /></TooltipButton>
          <span className="text-xs w-12 text-center tabular-nums">{Math.round(zoom * 100)}%</span>
          <TooltipButton size="sm" variant="outline" onClick={() => setZoom(z => Math.min(2, +(z + 0.1).toFixed(2)))} tooltip="Acercar (zoom +10%)"><ZoomIn className="size-4" /></TooltipButton>
        </div>
      </div>

      {/* Zonas (tabs) */}
      <div className="flex gap-1 flex-wrap items-center border-b border-border">
        {plano?.zonas.map(z => (
          <button
            key={z.id}
            type="button"
            className={`relative inline-flex items-center justify-center gap-1.5 px-3 py-1.5 text-sm font-medium whitespace-nowrap transition-colors rounded-t-md ${z.id === zonaActiva ? 'text-foreground after:absolute after:inset-x-0 after:bottom-[-1px] after:h-0.5 after:bg-foreground' : 'text-muted-foreground hover:text-foreground'}`}
            onClick={() => cambiarZona(z.id)}
          >
            {z.nombre} ({z.mesas.length})
          </button>
        ))}
        <button
          type="button"
          className="inline-flex items-center gap-1 px-3 py-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors rounded-t-md"
          onClick={handleCrearZona}
        >
          + Zona
        </button>
      </div>

      {/* [283A-17] Canvas 100% ancho + Panel lateral derecho */}
      {/* [303A-20] Canvas con overflow:hidden + transform para pan.
         * El contenido se desplaza via CSS transform, no scroll nativo. */}
      <div className="flex gap-4">
        <div className="flex-1 min-w-0">
          {zonaData ? (
            <div style={{ position: 'relative' }}>
              <div
                ref={viewportRef}
                className={`planoCanvas ${panning ? 'planoPanning' : ''} ${
                  activeTool === 'pan' ? (panning ? 'cursor-grabbing' : 'cursor-grab') :
                  activeTool === 'delete' ? 'planoCanvasDelete' :
                  (activeTool === 'mesa-cuadrada' || activeTool === 'mesa-redonda' || activeTool === 'pared') ? 'cursor-crosshair' : ''
                }`}
                style={{ height: canvasHeight }}
                onClick={onViewportClick}
                onMouseDown={onViewportMouseDown}
                onMouseMove={onViewportMouseMove}
              >
                <div
                  ref={canvasRef}
                  className="planoCanvasContent"
                  style={{
                    width: contentBounds.w,
                    height: contentBounds.h,
                    transform: `translate(${-panOffset.x}px, ${-panOffset.y}px)`,
                  }}
                >
                  {/* [134A-3] Paredes arrastrables — mueve con drag, rota con handle circular. */}
                  {paredesZona.map(pared => {
                    const paredDisplay = {
                      ...pared,
                      pos_x: pared.pos_x * zoom,
                      pos_y: pared.pos_y * zoom,
                      ancho: pared.ancho * zoom,
                      alto: pared.alto * zoom,
                    };
                    return (
                      <ParedDraggable
                        key={pared.id}
                        pared={paredDisplay}
                        canonical={pared}
                        zoom={zoom}
                        seleccionada={paredSeleccionada?.id === pared.id}
                        onClick={() => {
                          if (activeTool === 'delete') {
                            handleEliminarParedDirecta(pared.id);
                          } else {
                            setParedSeleccionada(pared);
                            setMesaSeleccionada(null);
                          }
                        }}
                        onMoveEnd={handleMoverPared}
                        onRotateEnd={handleRotarPared}
                        onResizeEnd={handleRedimensionarPared}
                      />
                    );
                  })}
                  {/* [134A-16] Preview de pared durante dibujo A→B */}
                  {wallDrawPreview && (
                    <div
                      className="absolute pointer-events-none"
                      style={{
                        left: wallDrawPreview.x * zoom,
                        top: wallDrawPreview.y * zoom,
                        width: wallDrawPreview.w * zoom,
                        height: 10 * zoom,
                        background: 'var(--foreground)',
                        opacity: 0.3,
                        transform: `rotate(${wallDrawPreview.rotation}deg)`,
                        transformOrigin: 'center center',
                        borderRadius: 'var(--radius)',
                      }}
                    />
                  )}
                  {mesasZona.map(mesa => {
                    const pos = posicionesLocales[mesa.id];
                    const dims = dimensionesLocales[mesa.id];
                    const base = {
                      ...mesa,
                      pos_x: pos?.x ?? mesa.pos_x,
                      pos_y: pos?.y ?? mesa.pos_y,
                      ancho: dims?.ancho ?? mesa.ancho,
                      alto: dims?.alto ?? mesa.alto,
                    };
                    const mesaZoom = zoom === 1 ? base : {
                      ...base,
                      pos_x: base.pos_x * zoom,
                      pos_y: base.pos_y * zoom,
                      ancho: base.ancho * zoom,
                      alto: base.alto * zoom,
                    };
                    return (
                      <MesaDraggable
                        key={mesa.id}
                        mesa={mesaZoom}
                        seleccionada={mesaSeleccionada?.id === mesa.id}
                        zoom={zoom}
                        zonaAncho={zonaData.ancho}
                        zonaAlto={zonaData.alto}
                        onResize={handleResizeMesa}
                        onMoveEnd={handleMoverMesa}
                        onClick={() => {
                          if (activeTool === 'delete') {
                            handleEliminarMesaDirecta(mesa.id);
                          } else {
                            setMesaSeleccionada(base);
                            setParedSeleccionada(null);
                          }
                        }}
                      />
                    );
                  })}
                </div>
              </div>
              {/* [134A-15] Toolbar flotante tipo Illustrator */}
              <CanvasToolbar
                activeTool={activeTool}
                onToolChange={setActiveTool}
                disabled={!zonaActiva}
              />
              <PlanoOverlays
                mesasZona={mesasZona}
                paredesZona={paredesZona}
                posicionesLocales={posicionesLocales}
                dimensionesLocales={dimensionesLocales}
                zoom={zoom}
                contentBounds={contentBounds}
                viewportSize={viewportSize}
                panOffset={panOffset}
                setPanOffset={setPanOffset}
              />
            </div>
          ) : (
            <div className="planoCanvas planoCanvasVacio" style={{ height: canvasHeight }}>
              {plano && plano.zonas.length === 0
                ? 'Crea tu primera zona para empezar a diseñar el plano'
                : 'Selecciona una zona'}
            </div>
          )}
          {/* [303A-13] Handle de resize — barra arrastrable debajo del canvas */}
          <div
            className={`planoResizeHandle ${resizing ? 'activo' : ''}`}
            onMouseDown={onResizeStart}
            title="Arrastrar para cambiar altura del plano"
          />
        </div>
        {/* [283A-17] Panel lateral derecho: config de mesa seleccionada */}
        {/* [303A-9][DataIV-6] key incluye todos los campos editables para forzar
         * remount al cambiar de mesa o cuando cambian valores tras un refetch. */}
        {mesaSeleccionada && !paredSeleccionada && (
          <div className="w-72 shrink-0">
            <PanelConfigMesa
              key={`${mesaSeleccionada.id}:${mesaSeleccionada.ancho}:${mesaSeleccionada.alto}:${mesaSeleccionada.forma}:${mesaSeleccionada.min_personas}:${mesaSeleccionada.max_personas}:${mesaSeleccionada.numero}:${mesaSeleccionada.activa}`}
              mesa={mesaSeleccionada}
              onGuardar={handleGuardarMesa}
              onEliminar={handleEliminarMesa}
              onDuplicar={handleDuplicarMesa}
              onCerrar={() => setMesaSeleccionada(null)}
            />
          </div>
        )}
        {paredSeleccionada && (
          <div className="w-72 shrink-0">
            <PanelConfigPared
              key={`${paredSeleccionada.id}:${paredSeleccionada.ancho}:${paredSeleccionada.alto}:${paredSeleccionada.color}:${paredSeleccionada.rotacion}`}
              pared={paredSeleccionada}
              onGuardar={handleGuardarPared}
              onEliminar={handleEliminarPared}
              onDuplicar={handleDuplicarPared}
              onCerrar={() => setParedSeleccionada(null)}
            />
          </div>
        )}
      </div>

      {plano && (
        <CombinacionesList
          combinaciones={plano.combinaciones}
          onEliminar={handleEliminarCombinacion}
        />
      )}
      {/* [283A-25] Diálogos extraídos a PlanoDialogs para mantener < 300 líneas */}
      <PlanoDialogs
        dialogoEntrada={dialogoEntrada} setDialogoEntrada={setDialogoEntrada}
        dialogoConfirmar={dialogoConfirmar} setDialogoConfirmar={setDialogoConfirmar}
        dialogoCombinacion={dialogoCombinacion} setDialogoCombinacion={setDialogoCombinacion}
        mesasZona={mesasZona}
      />
      <Dialog open={dialogoSyncBdp} onOpenChange={setDialogoSyncBdp}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader><DialogTitle>Importar salones y mesas desde BDP</DialogTitle><DialogDescription>Primero consulta el impacto. BDP solo se lee; al aplicar se crean únicamente zonas/mesas faltantes en la Aplicación Web y no se elimina nada.</DialogDescription></DialogHeader>
          {previewSyncBdp && <div className="rounded-md border p-3 text-sm">Se crearían {previewSyncBdp.zonas_creadas} zonas y {previewSyncBdp.mesas_creadas} mesas a partir de {previewSyncBdp.salones_bdp} salones BDP.</div>}
          {previewSyncBdp && <div><Label htmlFor="confirmar-sync-mesas">Escribe IMPORTAR MESAS BDP</Label><Input id="confirmar-sync-mesas" value={confirmacionSyncBdp} onChange={(e) => setConfirmacionSyncBdp(e.target.value)} /></div>}
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialogoSyncBdp(false)}>Cancelar</Button>
            <Button variant="secondary" disabled={sincronizandoMesas} onClick={() => { setSincronizandoMesas(true); syncTablesMutation.mutate({ data: { aplicar: false } }); }}>Previsualizar sin cambios</Button>
            {previewSyncBdp && <Button disabled={sincronizandoMesas || confirmacionSyncBdp !== 'IMPORTAR MESAS BDP'} onClick={() => { setSincronizandoMesas(true); syncTablesMutation.mutate({ data: { aplicar: true, confirmacion: confirmacionSyncBdp } }); }}>Aplicar en la Aplicación Web</Button>}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

export default PlanoSala;
