/* Tabla de albaranes de compra: una fila por albarán con sus acciones.
 * Sin estado propio: las acciones llegan del componente padre. */
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import type { BdpPurchaseNote } from '@/api/bdp';
import { BdpPurchaseNoteRowActions } from '../BdpPurchaseNoteRowActions';
import { formatCurrency, formatDate, formatEstado } from './formato';

interface BdpComprasTablaProps {
  notas: BdpPurchaseNote[];
  actualizando: boolean;
  onEditar: (note: BdpPurchaseNote) => void;
  onEliminar: (note: BdpPurchaseNote) => void;
  onBorrador: (note: BdpPurchaseNote) => void;
  onConciliar: (note: BdpPurchaseNote) => void;
}

export function BdpComprasTabla({ notas, actualizando, onEditar, onEliminar, onBorrador, onConciliar }: BdpComprasTablaProps) {
  return (
    <div className="rounded-md border overflow-x-auto">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Fecha</TableHead>
            <TableHead>Serie</TableHead>
            <TableHead>Número</TableHead>
            <TableHead>Proveedor</TableHead>
            <TableHead>Origen</TableHead>
            <TableHead className="text-right">Total</TableHead>
            <TableHead>Estado</TableHead>
            <TableHead className="text-center">Acciones</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {notas.map((note) => (
            <TableRow key={note.id}>
              <TableCell className="text-xs">{formatDate(note.fecha)}</TableCell>
              <TableCell className="font-mono text-xs">{note.serie || '—'}</TableCell>
              <TableCell className="font-mono text-xs">{note.numero || '—'}</TableCell>
              <TableCell className="text-xs">
                {note.nombre_proveedor || note.codigo_proveedor || '—'}
              </TableCell>
              <TableCell className="text-xs">
                <Badge variant={note.origen === 'local' ? 'secondary' : 'outline'}>
                  {note.origen === 'local' ? 'local' : 'BDP'}
                </Badge>
              </TableCell>
              <TableCell className="text-right text-xs tabular-nums">
                {formatCurrency(note.total)}
              </TableCell>
              <TableCell className="text-xs">{formatEstado(note.estado)}</TableCell>
              <TableCell className="text-right">
                <BdpPurchaseNoteRowActions
                  note={note}
                  isUpdating={actualizando}
                  onEditar={onEditar}
                  onEliminar={onEliminar}
                  onBorrador={onBorrador}
                  onConciliar={onConciliar}
                />
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}
