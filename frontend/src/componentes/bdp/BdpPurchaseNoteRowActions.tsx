/* [208A-3] Acciones de un albarán agrupadas en menú contextual de 3 puntos.
 * Los albaranes locales con más de 2 acciones (Editar/Eliminar + Borrador o
 * Conciliar según estado) usan menú; así la fila no acumula botones sueltos.
 * Los importados de BDP no se editan ni eliminan (solo Borrador/Conciliar). */

import { MoreHorizontal, Pencil, Trash2, FilePen, CheckCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import type { BdpPurchaseNote } from '@/api/bdp';

interface BdpPurchaseNoteRowActionsProps {
  note: BdpPurchaseNote;
  isUpdating: boolean;
  onEditar: (note: BdpPurchaseNote) => void;
  onEliminar: (note: BdpPurchaseNote) => void;
  onBorrador: (note: BdpPurchaseNote) => void;
  onConciliar: (note: BdpPurchaseNote) => void;
}

export function BdpPurchaseNoteRowActions({
  note,
  isUpdating,
  onEditar,
  onEliminar,
  onBorrador,
  onConciliar,
}: BdpPurchaseNoteRowActionsProps) {
  const esLocal = note.origen === 'local';

  return (
    <div className="flex items-center justify-center">
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            variant="outline"
            size="icon"
            aria-label="Acciones del albarán"
            className="bg-muted/40 hover:bg-muted"
          >
            <MoreHorizontal className="size-4" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-52">
          {esLocal && (
            <DropdownMenuItem onClick={() => onEditar(note)} disabled={isUpdating}>
              <Pencil className="mr-2 size-3.5" />
              Editar
            </DropdownMenuItem>
          )}
          {note.estado === 'pendiente' && (
            <DropdownMenuItem onClick={() => onBorrador(note)} disabled={isUpdating}>
              <FilePen className="mr-2 size-3.5" />
              Marcar borrador
            </DropdownMenuItem>
          )}
          {note.estado === 'borrador' && (
            <DropdownMenuItem onClick={() => onConciliar(note)} disabled={isUpdating}>
              <CheckCircle className="mr-2 size-3.5" />
              Conciliar
            </DropdownMenuItem>
          )}
          {esLocal && note.estado !== 'conciliado' && (
            <>
              <DropdownMenuSeparator />
              <DropdownMenuItem variant="destructive" onClick={() => onEliminar(note)} disabled={isUpdating}>
                <Trash2 className="mr-2 size-3.5" />
                Eliminar
              </DropdownMenuItem>
            </>
          )}
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
