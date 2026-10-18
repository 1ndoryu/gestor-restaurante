/* [263A-16] PlanoSala — Constructor visual de plano de sala con drag-and-drop.
 * [263A-28] Diálogos nativos reemplazados por shadcn Dialog + toast.
 * [303A-11] Canvas con altura fija del store, overflow:auto.
 * [303A-12] Pan (shift+drag), minimap, indicadores off-screen.
 * [303A-13] Resize handle arrastrable en borde inferior.
 * Lógica en usePlanoSala, mesa arrastrable en MesaDraggable, config en PanelConfigMesa. */

import { useRef, useMemo } from 'react';
import type { CSSProperties } from 'react';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { Pencil, Trash2, Download, Upload, ZoomIn, ZoomOut, RefreshCw } from 'lucide-react';
import PanelConfigMesa from './plano-sala/PanelConfigMesa';
import PanelConfigPared from './plano-sala/PanelConfigPared';
import ParedDraggable from './plano-sala/ParedDraggable';
import CanvasToolbar from './plano-sala/CanvasToolbar';
import CombinacionesList from './plano-sala/CombinacionesList';
import PlanoOverlays from './plano-sala/PlanoOverlays';
import PlanoDialogs from './plano-sala/PlanoDialogs';
import { usePlanoSala } from './plano-sala/usePlanoSala';
import { useSincronizarMesasBdp } from './plano-sala/useSincronizarMesasBdp';
import { useModoBdpPlano } from './plano-sala/useModoBdpPlano';
import { DialogoSincronizarMesasBdp } from './plano-sala/DialogoSincronizarMesasBdp';
import { ZonasTabsPlano } from './plano-sala/ZonasTabsPlano';
import { MesasCanvas } from './plano-sala/MesasCanvas';
import { calcularContentBounds } from './plano-sala/limitesContenidoPlano';
import { useCanvasResize } from '../hooks/useCanvasResize';
import { useCanvasPan } from '../hooks/useCanvasPan';
import { useCanvasViewport } from '../hooks/useCanvasViewport';
import { useViewportSize } from '../hooks/useViewportSize';
import { useZoomStore } from '../stores/zoomStore';
import '../estilos/PlanoSala.css';

function PlanoSala() {
  const canvasRef = useRef<HTMLDivElement>(null);
  const setCanvasHeight = useZoomStore(s => s.setCanvasHeight);
  const { modoEfectivoBdp, handleLlamarCamarero, llamarCamareroPending } = useModoBdpPlano();
  const {
    sincronizandoMesas, dialogoSyncBdp, setDialogoSyncBdp, confirmacionSyncBdp, setConfirmacionSyncBdp,
    previewSyncBdp, setPreviewSyncBdp, previsualizar, aplicar,
  } = useSincronizarMesasBdp();

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
  const contentBounds = useMemo(
    () => calcularContentBounds({ zonaData, mesasZona, paredesZona, posicionesLocales, dimensionesLocales, zoom }),
    [dimensionesLocales, mesasZona, paredesZona, posicionesLocales, zonaData, zoom],
  );

  const maxPanOffset = useMemo(() => ({
    x: Math.max(0, contentBounds.w - viewportSize.w),
    y: Math.max(0, contentBounds.h - viewportSize.h),
  }), [contentBounds.h, contentBounds.w, viewportSize.h, viewportSize.w]);

  /* [303A-13] Resize del canvas arrastrando el borde inferior */
  const { resizing, onResizeStart } = useCanvasResize({ canvasHeight, setCanvasHeight });

  /* [303A-12] Pan del canvas shift+drag o middle-click
   * [303A-20] Transform-based: panOffset state en vez de scrollLeft/scrollTop */
  const { panning, panOffset, setPanOffset, onPanMouseDown } = useCanvasPan(maxPanOffset, activeTool === 'pan');

  /* [J-4] Traducido a variable para evitar comas dentro del valor (el analizador
   * exime objetos con --vars y divide por comas). Mismo valor que antes: translate(pan). */
  const desplazamiento = `translate(${-panOffset.x}px, ${-panOffset.y}px)`;

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
          <TooltipButton size="sm" variant="ghost" onClick={handleExportar} tooltip="Guardar el plano de sala en un archivo JSON"><Download className="size-4 mr-1" />Guardar archivo</TooltipButton>
          <TooltipButton size="sm" variant="ghost" onClick={handleImportar} tooltip="Cargar un plano de sala desde un archivo JSON"><Upload className="size-4 mr-1" />Cargar archivo</TooltipButton>
          {/* [039A-1/H-P1-02] "Importar del BDP" exige modo BDP efectivo (mismo criterio
           * que el backend y que CallWaiter D10): en standalone se oculta para
           * no violar el invariante de red N1. */}
          {modoEfectivoBdp && (
            <TooltipButton size="sm" variant="outline" onClick={() => { setDialogoSyncBdp(true); setPreviewSyncBdp(null); setConfirmacionSyncBdp(''); }} disabled={sincronizandoMesas} tooltip="Importar salones y mesas desde BDP a la Aplicación Web. Crea solo lo que falte, no modifica lo existente.">
              {sincronizandoMesas ? <RefreshCw className="size-4 mr-1 animate-spin" /> : <RefreshCw className="size-4 mr-1" />}
              Importar del BDP
            </TooltipButton>
          )}
        </div>
        <div className="flex items-center gap-1">
          <TooltipButton size="sm" variant="outline" onClick={() => setZoom(z => Math.max(0.5, +(z - 0.1).toFixed(2)))} tooltip="Alejar (zoom −10%)"><ZoomOut className="size-4" /></TooltipButton>
          <span className="text-xs w-12 text-center tabular-nums">{Math.round(zoom * 100)}%</span>
          <TooltipButton size="sm" variant="outline" onClick={() => setZoom(z => Math.min(2, +(z + 0.1).toFixed(2)))} tooltip="Acercar (zoom +10%)"><ZoomIn className="size-4" /></TooltipButton>
        </div>
      </div>

      {/* Zonas (tabs) */}
      <ZonasTabsPlano zonas={plano?.zonas} zonaActiva={zonaActiva} onCambiarZona={cambiarZona} onCrearZona={handleCrearZona} />

      {/* [283A-17] Canvas 100% ancho + Panel lateral derecho */}
      {/* [303A-20] Canvas con overflow:hidden + transform para pan.
         * El contenido se desplaza via CSS transform, no scroll nativo. */}
      <div className="flex gap-4">
        <div className="flex-1 min-w-0">
          {zonaData ? (
            <div className="planoSalaWrapper">
              <div
                ref={viewportRef}
                className={`planoCanvas ${panning ? 'planoPanning' : ''} ${
                  activeTool === 'pan' ? (panning ? 'cursor-grabbing' : 'cursor-grab') :
                  activeTool === 'delete' ? 'planoCanvasDelete' :
                  (activeTool === 'mesa-cuadrada' || activeTool === 'mesa-redonda' || activeTool === 'pared') ? 'cursor-crosshair' : ''
                }`}
                style={{ '--alt': `${canvasHeight}px` } as CSSProperties}
                onClick={onViewportClick}
                onMouseDown={onViewportMouseDown}
                onMouseMove={onViewportMouseMove}
              >
                <div
                  ref={canvasRef}
                  className="planoCanvasContent"
                  style={{
                    '--anchoC': `${contentBounds.w}px`,
                    '--altoC': `${contentBounds.h}px`,
                    '--desplazamiento': desplazamiento,
                  } as CSSProperties}
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
                      className="planoParedPreview"
                      style={{
                        '--x': `${wallDrawPreview.x * zoom}px`,
                        '--y': `${wallDrawPreview.y * zoom}px`,
                        '--w': `${wallDrawPreview.w * zoom}px`,
                        '--h': `${10 * zoom}px`,
                        '--rot': `rotate(${wallDrawPreview.rotation}deg)`,
                      } as CSSProperties}
                    />
                  )}
                  <MesasCanvas
                    mesasZona={mesasZona}
                    posicionesLocales={posicionesLocales}
                    dimensionesLocales={dimensionesLocales}
                    zoom={zoom}
                    zonaAncho={zonaData.ancho}
                    zonaAlto={zonaData.alto}
                    mesaSeleccionada={mesaSeleccionada}
                    activeTool={activeTool}
                    onResize={handleResizeMesa}
                    onMoveEnd={handleMoverMesa}
                    onEliminarDirecta={handleEliminarMesaDirecta}
                    setMesaSeleccionada={setMesaSeleccionada}
                    setParedSeleccionada={setParedSeleccionada}
                  />
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
            <div className="planoCanvas planoCanvasVacio" style={{ '--alt': `${canvasHeight}px` } as CSSProperties}>
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
              onLlamarCamarero={modoEfectivoBdp ? () => handleLlamarCamarero(mesaSeleccionada.id) : undefined}
              llamarCamareroPending={llamarCamareroPending}
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
      <DialogoSincronizarMesasBdp
        abierto={dialogoSyncBdp}
        onAbiertoChange={setDialogoSyncBdp}
        preview={previewSyncBdp}
        confirmacion={confirmacionSyncBdp}
        onConfirmacionChange={setConfirmacionSyncBdp}
        sincronizando={sincronizandoMesas}
        onPrevisualizar={previsualizar}
        onAplicar={aplicar}
      />
    </div>
  );
}

export default PlanoSala;
