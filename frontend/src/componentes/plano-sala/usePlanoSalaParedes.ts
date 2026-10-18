import { useCallback, type Dispatch, type SetStateAction } from 'react';
import { toast } from 'sonner';
import {
  calcularClampPared,
  calcularPreviewPared,
  LARGO_MINIMO_PARED,
} from './geometriaPared';
import {
  CrearParedRequest,
  ActualizarParedRequest,
  ParedSala,
  crearPared,
  eliminarPared as eliminarParedApi,
  actualizarPared as actualizarParedApi,
} from '../../api/generated';
import type { DialogoConfirmar } from './tiposDialogosPlano';

type PuntoCanvas = { x: number; y: number };
type PreviewPared = { x: number; y: number; w: number; rotation: number };

/* Estado compartido que necesitan las acciones de paredes (lo posee usePlanoSala). */
export interface ContextoParedes {
  zonaActiva: string | null;
  paredesZona: ParedSala[];
  setParedSeleccionada: Dispatch<SetStateAction<ParedSala | null>>;
  wallDrawStart: PuntoCanvas | null;
  wallDrawPreview: PreviewPared | null;
  setWallDrawStart: Dispatch<SetStateAction<PuntoCanvas | null>>;
  setWallDrawPreview: Dispatch<SetStateAction<PreviewPared | null>>;
  refetchPlano: () => Promise<void>;
  setDialogoConfirmar: Dispatch<SetStateAction<DialogoConfirmar | null>>;
}

/* Acciones sobre paredes del plano: dibujo, CRUD, movimiento, rotación y duplicado. */
export function usePlanoSalaParedes(ctx: ContextoParedes) {
  const {
    zonaActiva, paredesZona, setParedSeleccionada,
    wallDrawStart, wallDrawPreview, setWallDrawStart, setWallDrawPreview,
    refetchPlano, setDialogoConfirmar,
  } = ctx;

  const handleEliminarParedDirecta = async (id: string) => {
    try {
      await eliminarParedApi(id);
      setParedSeleccionada(null);
      refetchPlano();
    } catch { toast.error('Error al eliminar pared'); }
  };

  /* [134A-16] Dibujar pared arrastrando de punto A a punto B.
   * mousedown → marca start. mousemove → calcula preview. mouseup → crea pared. */
  const handleWallDrawStart = useCallback((canvasX: number, canvasY: number) => {
    setWallDrawStart({ x: canvasX, y: canvasY });
    setWallDrawPreview(null);
  }, []);

  /* [154A-1] Snap a horizontal/vertical: solo se permite dibujar paredes a 0° o 90°.
   * Se elige el eje según si |dx| >= |dy| (horizontal) o viceversa (vertical).
   * La longitud se proyecta sobre ese eje para que el preview sea recto.
   * Largo mínimo: 30px — por debajo se ignora el movimiento. */
  const handleWallDrawMove = useCallback((canvasX: number, canvasY: number) => {
    if (!wallDrawStart) return;
    const preview = calcularPreviewPared(wallDrawStart.x, wallDrawStart.y, canvasX, canvasY);
    if (!preview) return;
    setWallDrawPreview(preview);
  }, [wallDrawStart]);

  /* [154A-1] Largo mínimo 30px al crear. Si el usuario suelta antes de alcanzarlo,
   * se descarta silenciosamente en lugar de crear una pared diminuta. */
  const handleWallDrawEnd = useCallback(async () => {
    if (!wallDrawStart || !wallDrawPreview || !zonaActiva || wallDrawPreview.w < LARGO_MINIMO_PARED) {
      setWallDrawStart(null);
      setWallDrawPreview(null);
      return;
    }
    const rot = ((wallDrawPreview.rotation % 360) + 360) % 360;
    try {
      await crearPared({
        zona_id: zonaActiva,
        ancho: Math.round(wallDrawPreview.w),
        alto: 10,
        rotacion: rot,
        pos_x: Math.round(wallDrawPreview.x),
        pos_y: Math.round(wallDrawPreview.y),
      } as CrearParedRequest);
      refetchPlano();
    } catch { toast.error('Error al crear pared'); }
    setWallDrawStart(null);
    setWallDrawPreview(null);
  }, [wallDrawStart, wallDrawPreview, zonaActiva, refetchPlano]);

  /* [134A-8] Crear pared: grosor fijo 10, largo por defecto 120 */
  const handleCrearPared = async () => {
    if (!zonaActiva) return;
    try {
      await crearPared({ zona_id: zonaActiva, ancho: 120, alto: 10 } as CrearParedRequest);
      refetchPlano();
    } catch {
      toast.error('Error al crear pared');
    }
  };

  const handleEliminarPared = (id: string) => {
    setDialogoConfirmar({
      titulo: 'Eliminar pared',
      mensaje: '¿Eliminar esta pared?',
      onConfirmar: async () => {
        try {
          await eliminarParedApi(id);
          setParedSeleccionada(null);
          refetchPlano();
        } catch {
          toast.error('Error al eliminar pared');
        }
      },
    });
  };

  const handleGuardarPared = async (id: string, req: ActualizarParedRequest) => {
    try {
      await actualizarParedApi(id, req);
      setParedSeleccionada(null);
      refetchPlano();
    } catch {
      toast.error('Error al actualizar pared');
    }
  };

  /* [134A-3] Mover/rotar pared via drag directo — no limpia la selección para UX fluida. */
  const handleMoverPared = async (id: string, pos_x: number, pos_y: number) => {
    const pared = paredesZona.find(p => p.id === id);
    if (!pared) return;
    const clamp = calcularClampPared(pared, pos_x, pos_y);
    try {
      await actualizarParedApi(id, {
        ancho: pared.ancho, alto: pared.alto,
        rotacion: pared.rotacion,
        pos_x: clamp.x, pos_y: clamp.y,
      });
      refetchPlano();
    } catch {
      toast.error('Error al mover pared');
    }
  };

  const handleRotarPared = async (id: string, rotacion: number) => {
    const pared = paredesZona.find(p => p.id === id);
    if (!pared) return;
    try {
      await actualizarParedApi(id, {
        ancho: pared.ancho, alto: pared.alto,
        rotacion,
        pos_x: pared.pos_x, pos_y: pared.pos_y,
      });
      refetchPlano();
    } catch {
      toast.error('Error al rotar pared');
    }
  };

  const handleRedimensionarPared = async (id: string, req: ActualizarParedRequest) => {
    try {
      await actualizarParedApi(id, req);
      refetchPlano();
    } catch {
      toast.error('Error al redimensionar pared');
    }
  };

  /* [134A-7] Duplicar pared — crea copia con offset de 20px */
  const handleDuplicarPared = async (pared: ParedSala) => {
    if (!zonaActiva) return;
    try {
      await crearPared({
        zona_id: zonaActiva,
        ancho: pared.ancho, alto: pared.alto,
        rotacion: pared.rotacion,
        pos_x: pared.pos_x + 20, pos_y: pared.pos_y + 20,
      } as CrearParedRequest);
      refetchPlano();
      toast.success('Pared duplicada');
    } catch {
      toast.error('Error al duplicar pared');
    }
  };

  return {
    handleEliminarParedDirecta, handleWallDrawStart, handleWallDrawMove, handleWallDrawEnd,
    handleCrearPared, handleEliminarPared, handleGuardarPared,
    handleMoverPared, handleRotarPared, handleRedimensionarPared, handleDuplicarPared,
  };
}
