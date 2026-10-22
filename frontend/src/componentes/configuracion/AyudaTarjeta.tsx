/* [D33] Botón de ayuda de una pestaña de Configuración: abre un modal con la
 * información completa de lo que hay dentro —qué hace cada ajuste, qué pasa si
 * se deja vacío, dónde se consigue el dato y qué se rompe si está mal—. El texto
 * no vive en las tarjetas: así se puede revisar la explicación sin tocar el
 * formulario, y una pestaña sin ayuda se nota al instante. */

import { HelpCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import type { IdSeccion } from './secciones';

export interface BloqueAyuda {
  /** Qué ajuste es, con el mismo nombre que ve la usuaria en la tarjeta. */
  titulo: string;
  /** Qué hace, en una frase sin jerga. */
  queHace: string;
  /** Dónde se consigue el dato, o qué pasa si se deja vacío. */
  detalle: string;
}

interface AyudaSeccionProps {
  seccion: IdSeccion;
  titulo: string;
  intro: string;
  bloques: readonly BloqueAyuda[];
}

export function AyudaSeccion({ seccion, titulo, intro, bloques }: AyudaSeccionProps) {
  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button variant="ghost" size="sm" aria-label={`Ayuda sobre ${titulo}`}>
          <HelpCircle className="size-4 mr-1" />
          Ayuda
        </Button>
      </DialogTrigger>
      <DialogContent className="max-h-[80vh] overflow-y-auto sm:max-w-2xl" id={`ayuda-${seccion}`}>
        <DialogHeader>
          <DialogTitle>Qué hay en «{titulo}»</DialogTitle>
          <DialogDescription>{intro}</DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-4">
          {bloques.map((bloque) => (
            <section key={bloque.titulo} className="flex flex-col gap-1">
              <h3 className="text-sm font-semibold">{bloque.titulo}</h3>
              <p className="text-sm">{bloque.queHace}</p>
              <p className="text-sm text-muted-foreground">{bloque.detalle}</p>
            </section>
          ))}
        </div>
      </DialogContent>
    </Dialog>
  );
}
