/* Tabla de definiciones de ejemplo del modo demo; cada fila abre su detalle. */
import { Eye } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import type { BdpExplorerItem } from '../bdp-mocks';
import { tipoLabel } from './constantes';

interface TablaDefinicionesDemoProps {
  items: BdpExplorerItem[];
  onVer: (item: BdpExplorerItem) => void;
}

export function TablaDefinicionesDemo({ items, onVer }: TablaDefinicionesDemoProps) {
  return (
    <div className="rounded-md border overflow-x-auto">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Código</TableHead>
            <TableHead>Nombre</TableHead>
            <TableHead>Tipo</TableHead>
            <TableHead>Descripción</TableHead>
            <TableHead className="w-10 text-center">Acciones</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {items.length === 0 ? (
            <TableRow>
              <TableCell colSpan={5} className="text-center text-[13px] text-muted-foreground">
                No hay definiciones que coincidan.
              </TableCell>
            </TableRow>
          ) : (
            items.map((item) => (
              <TableRow key={item.id}>
                <TableCell className="font-mono text-xs">{item.code}</TableCell>
                <TableCell className="text-[13px] font-medium">{item.name}</TableCell>
                <TableCell>
                  <Badge variant="outline">{tipoLabel(item.type)}</Badge>
                </TableCell>
                <TableCell className="text-xs text-muted-foreground max-w-xs truncate">
                  {item.description}
                </TableCell>
                <TableCell>
                  <div className="flex justify-center">
                    <Button variant="outline" size="icon" className="bg-muted/40 hover:bg-muted" onClick={() => onVer(item)}>
                      <Eye className="size-4" />
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            ))
          )}
        </TableBody>
      </Table>
    </div>
  );
}
