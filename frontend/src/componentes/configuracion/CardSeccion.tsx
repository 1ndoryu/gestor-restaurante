/* [D33] Una sección de configuración es un grupo de tarjetas con un único
 * punto de guardado: el mismo botón arriba (para quien ya sabe qué toca) y
 * abajo (para quien acaba de reeditar), con el estado en medio. Lo que guarda
 * al instante NO se envuelve aquí: lleva `GuardadoAlInstante` para decirlo. */

import { Save } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';

interface CardSeccionProps {
  /** Qué grupo es, en las palabras de la usuaria («Mi restaurante»). */
  titulo: string;
  /** Una línea con el paraqué del grupo. */
  descripcion: string;
  /** Sin guardar pendiente en esta sección: se avisa al intentar salir. */
  sucio: boolean;
  guardando: boolean;
  onGuardar: () => void;
  /** Resultado del último guardado (éxito o error), solo de esta sección. */
  mensaje?: string;
  children: React.ReactNode;
}

export function CardSeccion({
  titulo,
  descripcion,
  sucio,
  guardando,
  onGuardar,
  mensaje,
  children,
}: CardSeccionProps) {
  const estado = guardando ? 'Guardando…' : sucio ? 'Cambios sin guardar' : 'Sin cambios pendientes';
  return (
    <Card>
      <CardContent className="flex flex-col gap-6 pt-6">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <h2 className="text-lg font-semibold">{titulo}</h2>
            <p className="text-sm text-muted-foreground">{descripcion}</p>
          </div>
          <div className="flex items-center gap-3">
            {mensaje && (
              <span className={`text-sm ${mensaje.includes('Error') ? 'text-destructive' : 'text-green-600'}`}>
                {mensaje}
              </span>
            )}
            <span className={`text-xs ${sucio ? 'font-medium text-amber-700 dark:text-amber-400' : 'text-muted-foreground'}`}>
              {estado}
            </span>
            <Button onClick={onGuardar} disabled={guardando || !sucio}>
              <Save className="size-4 mr-1" />
              {guardando ? 'Guardando…' : 'Guardar'}
            </Button>
          </div>
        </div>
        {children}
        {sucio && (
          <div className="flex items-center justify-between gap-4 rounded-md border border-amber-300 bg-amber-50 p-3 dark:border-amber-700 dark:bg-amber-950/20">
            <p className="text-sm text-amber-800 dark:text-amber-300">
              Quedan cambios sin guardar en esta sección.
            </p>
            <Button onClick={onGuardar} disabled={guardando}>
              {guardando ? 'Guardando…' : 'Guardar cambios'}
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

/** Marca para lo que no espera al botón: sin ella, la pantalla mezcla dos modos
 * de guardado y la usuaria no sabe cuál es cuál. */
export function GuardadoAlInstante() {
  return (
    <p className="text-xs text-muted-foreground">
      Esto se guarda al instante, sin pulsar «Guardar».
    </p>
  );
}
