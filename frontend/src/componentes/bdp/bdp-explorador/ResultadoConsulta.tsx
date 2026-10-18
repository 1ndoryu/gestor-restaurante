/* Resultado de una consulta real a BDP: nombre, código, número de artículos y su detalle. */
import { Badge } from '@/components/ui/badge';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { tipoLabel, type BdpDefinition, type ExploreType } from './constantes';

interface ResultadoConsultaProps {
  resultado: BdpDefinition;
  tipo: ExploreType;
}

export function ResultadoConsulta({ resultado, tipo }: ResultadoConsultaProps) {
  return (
    <div className="rounded-md border p-3 text-sm space-y-3">
      <div className="flex items-center gap-2">
        <span className="font-medium">{resultado.name ?? 'Sin nombre'}</span>
        <Badge variant="outline">{tipoLabel(tipo)}</Badge>
      </div>
      {resultado.description && (
        <p className="text-xs text-muted-foreground">{resultado.description}</p>
      )}
      <div className="grid grid-cols-2 gap-2 text-xs">
        <span>
          Código: <Badge variant="outline">{resultado.code ?? '—'}</Badge>
        </span>
        <span>
          Artículos: <Badge variant="secondary">{resultado.lines?.length ?? 0}</Badge>
        </span>
      </div>
      {resultado.lines && resultado.lines.length > 0 && (
        <div className="rounded-md border overflow-hidden">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="text-xs">Artículo</TableHead>
                <TableHead className="text-xs">Código</TableHead>
                <TableHead className="text-xs text-right">Cantidad</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {resultado.lines.map((line) => (
                <TableRow key={String(line.article_code ?? line.article_name ?? 'linea')}>
                  <TableCell className="text-xs">{line.article_name ?? '—'}</TableCell>
                  <TableCell className="font-mono text-xs">{line.article_code ?? '—'}</TableCell>
                  <TableCell className="text-xs text-right">{line.quantity ?? '—'}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}
    </div>
  );
}
