import { useCallback, type Dispatch, type SetStateAction } from 'react';
import { toast } from 'sonner';
import {
  ActualizarMesaRequest,
  ActualizarPosicionesRequest,
  CrearMesaRequest,
  Mesa,
  crearMesa,
  actualizarMesa as actualizarMesaApi,
  eliminarMesa as eliminarMesaApi,
  actualizarPosiciones,
} from '../../api/generated';
import type { DialogoConfirmar, DialogoEntrada } from './tiposDialogosPlano';

/* Estado compartido que necesitan las acciones de mesas (lo posee usePlanoSala). */
export interface ContextoMesas {
  zonaActiva: string | null;
  mesasZona: Mesa[];
  mesaSeleccionada: Mesa | null;
  setMesaSeleccionada: Dispatch<SetStateAction<Mesa | null>>;
  posicionesLocales: Record<string, { x: number; y: number }>;
  setPosicionesLocales: Dispatch<SetStateAction<Record<string, { x: number; y: number }>>>;
  dimensionesLocales: Record<string, { ancho: number; alto: number }>;
  setDimensionesLocales: Dispatch<SetStateAction<Record<string, { ancho: number; alto: number }>>>;
  refetchPlano: () => Promise<void>;
  setDialogoEntrada: Dispatch<SetStateAction<DialogoEntrada | null>>;
  setDialogoConfirmar: Dispatch<SetStateAction<DialogoConfirmar | null>>;
}

/* Acciones sobre mesas del plano: alta, borrado, movimiento, redimensión y duplicado. */
export function usePlanoSalaMesas(ctx: ContextoMesas) {
  const {
    zonaActiva, mesasZona, mesaSeleccionada, setMesaSeleccionada,
    posicionesLocales, setPosicionesLocales, dimensionesLocales, setDimensionesLocales,
    refetchPlano, setDialogoEntrada, setDialogoConfirmar,
  } = ctx;

  /* [283A-17] Acepta posición opcional para drag-to-create. Try-catch para
   * mostrar toast si el backend retorna Conflict (número duplicado). */
  const handleCrearMesa = (pos?: { x: number; y: number }) => {
    if (!zonaActiva) return;
    setDialogoEntrada({
      titulo: 'Nueva mesa', label: 'Número de mesa', valorInicial: '', tipo: 'number',
      onConfirmar: async (val) => {
        const numero = Number(val);
        if (Number.isNaN(numero) || numero < 1) return;
        try {
          await crearMesa({ zona_id: zonaActiva, numero, pos_x: pos?.x ?? 50, pos_y: pos?.y ?? 50 } as CrearMesaRequest);
          refetchPlano();
        } catch (err: unknown) {
          const axiosErr = err as { response?: { data?: { message?: string } } };
          toast.error(axiosErr?.response?.data?.message || 'Error al crear mesa');
        }
      },
    });
  };

  /* [134A-15+134A-18] Creación rápida de mesa via herramienta del toolbar.
   * Auto-genera el número (max actual + 1). Sin diálogo. */
  const handleCrearMesaRapida = async (pos: { x: number; y: number }, forma: string) => {
    if (!zonaActiva) return;
    const nextNum = mesasZona.length > 0
      ? Math.max(...mesasZona.map(m => m.numero)) + 1
      : 1;
    try {
      await crearMesa({
        zona_id: zonaActiva, numero: nextNum, forma,
        ancho: forma === 'rectangular' ? 144 : 80,
        alto: 80,
        pos_x: pos.x, pos_y: pos.y,
      } as CrearMesaRequest);
      refetchPlano();
      toast.success(`Mesa ${nextNum} creada`);
    } catch (err: unknown) {
      const axiosErr = err as { response?: { data?: { message?: string } } };
      toast.error(axiosErr?.response?.data?.message || 'Error al crear mesa');
    }
  };

  /* [134A-15] Eliminar directamente (herramienta borrar) sin diálogo de confirmación */
  const handleEliminarMesaDirecta = async (id: string) => {
    try {
      await eliminarMesaApi(id);
      setMesaSeleccionada(null);
      refetchPlano();
    } catch { toast.error('Error al eliminar mesa'); }
  };

  const handleGuardarMesa = async (id: string, req: ActualizarMesaRequest) => {
    if (req.pos_x !== undefined || req.pos_y !== undefined) {
      setPosicionesLocales((prev) => ({ ...prev, [id]: { x: req.pos_x ?? prev[id]?.x ?? 0, y: req.pos_y ?? prev[id]?.y ?? 0 } }));
    }
    if (req.ancho !== undefined || req.alto !== undefined) {
      const mesaActual = mesasZona.find((m) => m.id === id);
      setDimensionesLocales((prev) => ({ ...prev, [id]: { ancho: req.ancho ?? prev[id]?.ancho ?? mesaActual?.ancho ?? 80, alto: req.alto ?? prev[id]?.alto ?? mesaActual?.alto ?? 80 } }));
    }
    await actualizarMesaApi(id, req);
    setMesaSeleccionada(null);
    refetchPlano();
  };

  const handleResizeMesa = async (id: string, req: ActualizarMesaRequest) => {
    const mesaActual = mesasZona.find((m) => m.id === id);
    if (!mesaActual) return;

    const prevPosicion = posicionesLocales[id];
    const prevDimension = dimensionesLocales[id];
    const prevSeleccionada = mesaSeleccionada;

    setPosicionesLocales((prev) => ({ ...prev, [id]: { x: req.pos_x ?? prev[id]?.x ?? mesaActual.pos_x, y: req.pos_y ?? prev[id]?.y ?? mesaActual.pos_y } }));
    setDimensionesLocales((prev) => ({ ...prev, [id]: { ancho: req.ancho ?? prev[id]?.ancho ?? mesaActual.ancho, alto: req.alto ?? prev[id]?.alto ?? mesaActual.alto } }));

    if (mesaSeleccionada?.id === id) {
      setMesaSeleccionada({ ...mesaSeleccionada, pos_x: req.pos_x ?? mesaSeleccionada.pos_x, pos_y: req.pos_y ?? mesaSeleccionada.pos_y, ancho: req.ancho ?? mesaSeleccionada.ancho, alto: req.alto ?? mesaSeleccionada.alto });
    }

    // Si el backend rechaza el cambio, se revierte el estado local optimista.
    try {
      await actualizarMesaApi(id, req);
      refetchPlano();
    } catch {
      setPosicionesLocales((prev) => {
        const next = { ...prev };
        if (prevPosicion) next[id] = prevPosicion; else delete next[id];
        return next;
      });
      setDimensionesLocales((prev) => {
        const next = { ...prev };
        if (prevDimension) next[id] = prevDimension; else delete next[id];
        return next;
      });
      if (prevSeleccionada?.id === id) setMesaSeleccionada(prevSeleccionada);
      toast.error('No se pudo redimensionar la mesa');
    }
  };

  const handleEliminarMesa = (id: string) => {
    setDialogoConfirmar({
      titulo: 'Eliminar mesa',
      mensaje: '¿Eliminar esta mesa?',
      onConfirmar: async () => {
        await eliminarMesaApi(id);
        setMesaSeleccionada(null);
        refetchPlano();
      },
    });
  };

  /* [134A-21] Mover mesa vía pointer events nativos (sin dnd-kit transform).
   * Recibe coordenadas canónicas ya calculadas y clampadas por MesaDraggable. */
  const handleMoverMesa = useCallback(async (mesaId: string, canonicalX: number, canonicalY: number) => {
    const mesa = mesasZona.find((m) => m.id === mesaId);
    if (!mesa) return;
    const prev = posicionesLocales[mesaId];
    setPosicionesLocales((p) => ({ ...p, [mesaId]: { x: canonicalX, y: canonicalY } }));
    const req: ActualizarPosicionesRequest = {
      posiciones: [{ id: mesaId, pos_x: canonicalX, pos_y: canonicalY }],
    };
    try {
      await actualizarPosiciones(req);
      refetchPlano();
    } catch {
      setPosicionesLocales((p) => ({ ...p, [mesaId]: { x: prev?.x ?? mesa.pos_x, y: prev?.y ?? mesa.pos_y } }));
      toast.error('No se pudo guardar la posición');
    }
  }, [mesasZona, posicionesLocales, refetchPlano]);

  /* [134A-7] Duplicar mesa — crea copia con siguiente número disponible + offset 20px */
  const handleDuplicarMesa = async (mesa: Mesa) => {
    if (!zonaActiva) return;
    const maxNum = mesasZona.reduce((max, m) => Math.max(max, m.numero), 0);
    try {
      await crearMesa({
        zona_id: zonaActiva,
        numero: maxNum + 1,
        forma: mesa.forma, ancho: mesa.ancho, alto: mesa.alto,
        min_personas: mesa.min_personas, max_personas: mesa.max_personas,
        pos_x: mesa.pos_x + 20, pos_y: mesa.pos_y + 20,
      } as CrearMesaRequest);
      refetchPlano();
      toast.success(`Mesa ${maxNum + 1} duplicada`);
    } catch (err: unknown) {
      const axiosErr = err as { response?: { data?: { message?: string } } };
      toast.error(axiosErr?.response?.data?.message || 'Error al duplicar mesa');
    }
  };

  return {
    handleCrearMesa, handleCrearMesaRapida, handleEliminarMesaDirecta,
    handleGuardarMesa, handleResizeMesa, handleEliminarMesa,
    handleMoverMesa, handleDuplicarMesa,
  };
}
