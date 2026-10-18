/* Detalle de una definición de ejemplo (modo demo): código, tipo, descripción y artículos. */
import { Badge } from '@/components/ui/badge';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import type { BdpExplorerItem } from '../bdp-mocks';
import { tipoLabel } from './constantes';

export function DefinitionDetail({ data }: { data: BdpExplorerItem }) {
  return (
    <div className="space-y-3 text-sm">
      <div className="grid grid-cols-2 gap-2">
        <div>
          <p className="text-xs text-muted-foreground">Código</p>
          <p className="font-medium">{data.code}</p>
        </div>
        <div>
          <p className="text-xs text-muted-foreground">Tipo</p>
          <Badge variant="outline">{tipoLabel(data.type)}</Badge>
        </div>
      </div>
      <div>
        <p className="text-xs text-muted-foreground">Descripción</p>
        <p>{data.description}</p>
      </div>
      <div>
        <p className="text-xs text-muted-foreground">Artículos</p>
        <div className="rounded-md border overflow-hidden mt-1">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="text-xs">Artículo</TableHead>
                <TableHead className="text-xs">Código</TableHead>
                <TableHead className="text-xs text-right">Cantidad</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {data.lines.map((line) => (
                <TableRow key={String(line.articleCode)}>
                  <TableCell className="text-xs">{line.articleName}</TableCell>
                  <TableCell className="font-mono text-xs">{line.articleCode}</TableCell>
                  <TableCell className="text-xs text-right">{line.quantity}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      </div>
    </div>
  );
}
