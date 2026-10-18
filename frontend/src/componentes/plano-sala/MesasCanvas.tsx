/* Mesas del canvas del plano: cada mesa en su posición escalada por el zoom.
 * Con la herramienta de borrado un clic elimina la mesa; si no, la selecciona para configurarla. */
import MesaDraggable from './MesaDraggable';
import type { EstadoPlanoSala } from './tipoEstadoPlano';

interface MesasCanvasProps {
  mesasZona: EstadoPlanoSala['mesasZona'];
  posicionesLocales: EstadoPlanoSala['posicionesLocales'];
  dimensionesLocales: EstadoPlanoSala['dimensionesLocales'];
  zoom: EstadoPlanoSala['zoom'];
  zonaAncho: number;
  zonaAlto: number;
  mesaSeleccionada: EstadoPlanoSala['mesaSeleccionada'];
  activeTool: EstadoPlanoSala['activeTool'];
  onResize: EstadoPlanoSala['handleResizeMesa'];
  onMoveEnd: EstadoPlanoSala['handleMoverMesa'];
  onEliminarDirecta: EstadoPlanoSala['handleEliminarMesaDirecta'];
  setMesaSeleccionada: EstadoPlanoSala['setMesaSeleccionada'];
  setParedSeleccionada: EstadoPlanoSala['setParedSeleccionada'];
}

export function MesasCanvas({
  mesasZona,
  posicionesLocales,
  dimensionesLocales,
  zoom,
  zonaAncho,
  zonaAlto,
  mesaSeleccionada,
  activeTool,
  onResize,
  onMoveEnd,
  onEliminarDirecta,
  setMesaSeleccionada,
  setParedSeleccionada,
}: MesasCanvasProps) {
  return (
    <>
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
            zonaAncho={zonaAncho}
            zonaAlto={zonaAlto}
            onResize={onResize}
            onMoveEnd={onMoveEnd}
            onClick={() => {
              if (activeTool === 'delete') {
                onEliminarDirecta(mesa.id);
              } else {
                setMesaSeleccionada(base);
                setParedSeleccionada(null);
              }
            }}
          />
        );
      })}
    </>
  );
}
