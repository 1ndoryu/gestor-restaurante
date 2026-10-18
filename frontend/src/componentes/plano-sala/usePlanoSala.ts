/* [263A-14] Hook con lógica de negocio del plano de sala.
 * [263A-28] Reemplazados prompt/confirm/alert nativos por estado React + toast.
 * [014A-13] Tras cada mutación se invalidate también /api/plano-sala/ocupacion
 * para que PlanoOcupacion (reservas) refleje cambios en tiempo real.
 * Los diálogos se renderizan en PlanoSala.tsx usando shadcn Dialog.
 * Las acciones de mesas y paredes viven en usePlanoSalaMesas y usePlanoSalaParedes. */

import { useState, useCallback, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import axios from '@/api/axios-instance';
import { useZoomStore } from '../../stores/zoomStore';
import type { CanvasTool } from './CanvasToolbar';
import type { DialogoEntrada, DialogoConfirmar, DialogoCombinacion } from './tiposDialogosPlano';
import { usePlanoSalaMesas } from './usePlanoSalaMesas';
import { usePlanoSalaParedes } from './usePlanoSalaParedes';
import {
  useObtenerPlano,
  getObtenerOcupacionQueryKey,
  CrearZonaRequest,
  ActualizarZonaRequest,
  CrearCombinacionRequest,
  PlanoExport,
  Mesa,
  ParedSala,
  crearZona,
  eliminarZona,
  actualizarZona as actualizarZonaApi,
  importarPlano,
  crearCombinacion,
  eliminarCombinacion,
} from '../../api/generated';

// Re-exportados para que PlanoDialogs.tsx siga importándolos desde aquí.
export type { DialogoEntrada, DialogoConfirmar, DialogoCombinacion } from './tiposDialogosPlano';

/* [283A-25] Clamp de mesas gestionado por MesaDraggable nativamente (pointer events).
 * [134A-21] Se eliminó canvasRef — ya no es necesario para arrastrar mesas. */
export function usePlanoSala() {
  const queryClient = useQueryClient();
  const { data, refetch } = useObtenerPlano();
  const plano = data?.status === 200 ? data.data : null;

  /* [014A-13] Invalida tanto /api/plano-sala como /api/plano-sala/ocupacion
   * para que PlanoOcupacion (reservas) refleje cambios de mesas al instante. */
  const refetchPlano = useCallback(async () => {
    await refetch();
    queryClient.invalidateQueries({ queryKey: getObtenerOcupacionQueryKey() });
  }, [refetch, queryClient]);

  const [zonaActiva, setZonaActiva] = useState<string | null>(null);
  const [mesaSeleccionada, setMesaSeleccionada] = useState<Mesa | null>(null);
  const [paredSeleccionada, setParedSeleccionada] = useState<ParedSala | null>(null);
  const [posicionesLocales, setPosicionesLocales] = useState<
    Record<string, { x: number; y: number }>
  >({});
  const [dimensionesLocales, setDimensionesLocales] = useState<
    Record<string, { ancho: number; alto: number }>
  >({});
  const zoom = useZoomStore(s => s.zoom);
  const setZoom = useZoomStore(s => s.setZoom);
  const canvasHeight = useZoomStore(s => s.canvasHeight);

  /* [134A-15] Sistema de herramientas tipo Illustrator */
  const [activeTool, setActiveToolRaw] = useState<CanvasTool>('select');
  /* Estado para dibujar paredes arrastrando de punto A a punto B */
  const [wallDrawStart, setWallDrawStart] = useState<{ x: number; y: number } | null>(null);
  const [wallDrawPreview, setWallDrawPreview] = useState<{
    x: number; y: number; w: number; rotation: number;
  } | null>(null);

  const setActiveTool = useCallback((tool: CanvasTool) => {
    setActiveToolRaw(tool);
    setMesaSeleccionada(null);
    setParedSeleccionada(null);
    setWallDrawStart(null);
    setWallDrawPreview(null);
  }, []);

  /* Estado de diálogos — reemplazan prompt/confirm/alert nativos */
  const [dialogoEntrada, setDialogoEntrada] = useState<DialogoEntrada | null>(null);
  const [dialogoConfirmar, setDialogoConfirmar] = useState<DialogoConfirmar | null>(null);
  const [dialogoCombinacion, setDialogoCombinacion] = useState<DialogoCombinacion | null>(null);
  /* Ref para archivo importado pendiente de confirmación */
  const archivoImportRef = useRef<PlanoExport | null>(null);

  const zonaData = plano?.zonas.find((z) => z.id === zonaActiva);
  const mesasZona = zonaData?.mesas ?? [];
  const paredesZona: ParedSala[] = zonaData?.paredes ?? [];

  if (plano && plano.zonas.length > 0 && !zonaActiva) {
    setZonaActiva(plano.zonas[0].id);
  }

  const cambiarZona = (id: string) => {
    setZonaActiva(id);
    setMesaSeleccionada(null);
    setPosicionesLocales({});
    setDimensionesLocales({});
  };

  const handleCrearZona = () => {
    setDialogoEntrada({
      titulo: 'Nueva zona', label: 'Nombre de la zona', valorInicial: '', tipo: 'text',
      onConfirmar: async (nombre) => {
        await crearZona({ nombre } as CrearZonaRequest);
        refetchPlano();
      },
    });
  };

  const handleEliminarZona = () => {
    if (!zonaActiva) return;
    setDialogoConfirmar({
      titulo: 'Eliminar zona',
      mensaje: '¿Eliminar esta zona y todas sus mesas?',
      onConfirmar: async () => {
        await eliminarZona(zonaActiva);
        setZonaActiva(null);
        setMesaSeleccionada(null);
        refetchPlano();
      },
    });
  };

  const handleEditarZona = () => {
    if (!zonaActiva || !zonaData) return;
    setDialogoEntrada({
      titulo: 'Renombrar zona', label: 'Nuevo nombre', valorInicial: zonaData.nombre, tipo: 'text',
      onConfirmar: async (nombre) => {
        await actualizarZonaApi(zonaActiva, { nombre } as ActualizarZonaRequest);
        refetchPlano();
      },
    });
  };

  const {
    handleCrearMesa, handleCrearMesaRapida, handleEliminarMesaDirecta,
    handleGuardarMesa, handleResizeMesa, handleEliminarMesa,
    handleMoverMesa, handleDuplicarMesa,
  } = usePlanoSalaMesas({
    zonaActiva, mesasZona, mesaSeleccionada, setMesaSeleccionada,
    posicionesLocales, setPosicionesLocales, dimensionesLocales, setDimensionesLocales,
    refetchPlano, setDialogoEntrada, setDialogoConfirmar,
  });

  const {
    handleEliminarParedDirecta, handleWallDrawStart, handleWallDrawMove, handleWallDrawEnd,
    handleCrearPared, handleEliminarPared, handleGuardarPared,
    handleMoverPared, handleRotarPared, handleRedimensionarPared, handleDuplicarPared,
  } = usePlanoSalaParedes({
    zonaActiva, paredesZona, setParedSeleccionada,
    wallDrawStart, wallDrawPreview, setWallDrawStart, setWallDrawPreview,
    refetchPlano, setDialogoConfirmar,
  });

  const handleDragStart = (_event: { active: { id: string | number } }) => {
    /* [134A-21] Drag gestionado por MesaDraggable con pointer events — no se usa dnd-kit */
  };

  /* [303A-2] Migrado de raw fetch a axios para usar interceptors JWT/401 */
  const handleExportar = async () => {
    try {
      const resp = await axios.get('/api/plano-sala/export');
      const json = resp.data;
      const blob = new Blob([JSON.stringify(json, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'plano-sala.json';
      a.click();
      URL.revokeObjectURL(url);
    } catch { toast.error('Error al exportar'); }
  };

  const handleImportar = () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;
      try {
        const text = await file.text();
        const d: PlanoExport = JSON.parse(text);
        archivoImportRef.current = d;
        setDialogoConfirmar({
          titulo: 'Importar plano',
          mensaje: 'Esto reemplazará todo el plano actual. ¿Continuar?',
          onConfirmar: async () => {
            if (!archivoImportRef.current) return;
            await importarPlano(archivoImportRef.current);
            archivoImportRef.current = null;
            setZonaActiva(null);
            setMesaSeleccionada(null);
            refetchPlano();
          },
        });
      } catch { toast.error('Error al importar: archivo inválido'); }
    };
    input.click();
  };

  /* [283A-17] Ahora recibe mesa_ids desde el diálogo (selección visual de mesas). */
  const handleCrearCombinacion = () => {
    setDialogoCombinacion({
      onConfirmar: async (nombre, maxPersonas, mesaIds) => {
        try {
          await crearCombinacion({ nombre, max_personas: maxPersonas, mesa_ids: mesaIds } as CrearCombinacionRequest);
          refetchPlano();
        } catch {
          toast.error('Error al crear combinación');
        }
      },
    });
  };

  const handleEliminarCombinacion = (id: string) => {
    setDialogoConfirmar({
      titulo: 'Eliminar combinación',
      mensaje: '¿Eliminar esta combinación?',
      onConfirmar: async () => {
        await eliminarCombinacion(id);
        refetchPlano();
      },
    });
  };

  return {
    plano, zonaActiva, zonaData, mesasZona, paredesZona, mesaSeleccionada,
    paredSeleccionada, setParedSeleccionada,
    posicionesLocales, dimensionesLocales, setMesaSeleccionada, cambiarZona, zoom, setZoom,
    canvasHeight,
    /* [134A-15] Herramientas del toolbar */
    activeTool, setActiveTool,
    wallDrawStart, wallDrawPreview,
    handleCrearMesaRapida, handleEliminarMesaDirecta, handleEliminarParedDirecta,
    handleWallDrawStart, handleWallDrawMove, handleWallDrawEnd,
    handleCrearZona, handleEliminarZona, handleEditarZona,
    handleCrearMesa, handleMoverMesa, handleGuardarMesa, handleResizeMesa, handleEliminarMesa,
    handleCrearPared, handleEliminarPared, handleGuardarPared,
    handleMoverPared, handleRotarPared, handleRedimensionarPared,
    handleDuplicarPared, handleDuplicarMesa,
    handleDragStart,
    handleExportar, handleImportar,
    handleCrearCombinacion, handleEliminarCombinacion,
    dialogoEntrada, setDialogoEntrada,
    dialogoConfirmar, setDialogoConfirmar,
    dialogoCombinacion, setDialogoCombinacion,
  };
}
