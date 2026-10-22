/* [D33/D-auto] Un solo punto de guardado por pestaña, fijo abajo a la derecha.
 *
 * Antes cada pestaña tenía dos: uno en la cabecera y otro al final, y el de la
 * cabecera de la segunda tarjeta parecía guardar solo esa tarjeta. Aquí hay uno
 * por pestaña y siempre en el mismo sitio, así que la posición no depende de
 * cuánto se haya desplazado la página ni de en qué tarjeta se esté editando. */

import { Loader2, RotateCcw, Save } from 'lucide-react';
import { Button } from '@/components/ui/button';

interface BarraGuardadoProps {
  /** Sin guardar pendiente en esta pestaña. */
  sucio: boolean;
  guardando: boolean;
  onGuardar: () => void;
  /** Vuelve a los valores del servidor. */
  onDeshacer: () => void;
  /** Resultado del último guardado (éxito o error) de esta pestaña. */
  mensaje?: string;
}

export function BarraGuardado({ sucio, guardando, onGuardar, onDeshacer, mensaje }: BarraGuardadoProps) {
  const estado = guardando ? 'Guardando…' : sucio ? 'Cambios sin guardar' : 'Sin cambios pendientes';
  return (
    <div className="sticky bottom-4 z-30 ml-auto flex w-fit max-w-full flex-wrap items-center gap-3 rounded-lg border bg-background/95 p-2 pl-3 shadow-lg backdrop-blur">
      {mensaje && (
        <span className={`text-sm ${mensaje.includes('Error') ? 'text-destructive' : 'text-green-600'}`}>
          {mensaje}
        </span>
      )}
      <span
        className={`text-xs ${sucio ? 'font-medium text-amber-700 dark:text-amber-400' : 'text-muted-foreground'}`}
      >
        {estado}
      </span>
      {sucio && (
        <Button variant="ghost" onClick={onDeshacer} disabled={guardando}>
          <RotateCcw className="mr-1 size-4" />
          Deshacer
        </Button>
      )}
      <Button onClick={onGuardar} disabled={guardando || !sucio}>
        {guardando ? <Loader2 className="mr-1 size-4 animate-spin" /> : <Save className="mr-1 size-4" />}
        {guardando ? 'Guardando…' : 'Guardar'}
      </Button>
    </div>
  );
}
