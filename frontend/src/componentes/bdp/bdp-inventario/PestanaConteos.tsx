/* Pestaña «Conteos» del inventario: historial de conteos guardados con paginación y
 * acción de retomar. Sin estado propio: la página y las acciones llegan desde BdpInventario. */
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { RotateCcw, Save } from 'lucide-react';
import type { BdpConteoInventario } from '@/api/bdp';
import { formatFecha } from './formato';

interface PestanaConteosProps {
  conteos: BdpConteoInventario[] | undefined;
  conteosPagina: BdpConteoInventario[];
  pagina: number;
  totalPaginas: number;
  tamanoPagina: number;
  retomando: string | null;
  onRetomar: (conteoId: string) => void;
  onAnterior: () => void;
  onSiguiente: () => void;
}

export function PestanaConteos({
  conteos,
  conteosPagina,
  pagina,
  totalPaginas,
  tamanoPagina,
  retomando,
  onRetomar,
  onAnterior,
  onSiguiente,
}: PestanaConteosProps) {
  return (
    <>
      {!conteos || conteos.length === 0 ? (
        <p className="text-xs text-muted-foreground">Todavía no hay conteos guardados.</p>
      ) : (
        <>
        <div className="rounded-md border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Fecha</TableHead>
                {/* [L7] Sin el almacén, un historial de varios almacenes es
                    indistinguible: dos conteos del mismo día serían iguales. */}
                <TableHead>Almacén</TableHead>
                <TableHead>Observaciones</TableHead>
                <TableHead className="text-right">Líneas</TableHead>
                <TableHead>Estado</TableHead>
                <TableHead className="w-32 text-center">Acciones</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {conteosPagina.map((c) => (
                <TableRow key={c.id}>
                  <TableCell className="text-xs tabular-nums">{formatFecha(c.fecha)}</TableCell>
                  <TableCell className="text-xs tabular-nums">
                    {c.warehouse_id === '0' ? 'General (0)' : c.warehouse_id}
                  </TableCell>
                  <TableCell className="max-w-64 truncate text-xs" title={c.observaciones || undefined}>
                    {c.observaciones || '—'}
                  </TableCell>
                  <TableCell className="text-right text-xs tabular-nums">{c.total_lineas}</TableCell>
                  <TableCell>
                    <Badge variant="secondary" className="gap-1">
                      <Save className="size-3" />
                      {c.estado === 'aplicado' ? 'aplicado' : c.estado}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-center">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => onRetomar(c.id)}
                      disabled={retomando === c.id}
                    >
                      <RotateCcw className="size-3.5 mr-1" />
                      Retomar
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
        {(conteos?.length ?? 0) > tamanoPagina && (
          <div className="flex flex-col sm:flex-row items-center justify-between gap-3 text-sm">
            <span className="text-muted-foreground">
              Mostrando {conteosPagina.length} de {conteos?.length} conteos
            </span>
            <div className="flex items-center gap-2">
              <Button variant="outline" size="sm" onClick={() => onAnterior()} disabled={pagina <= 1}>
                Anterior
              </Button>
              <span className="text-muted-foreground">
                Página {pagina} de {totalPaginas}
              </span>
              <Button
                variant="outline"
                size="sm"
                onClick={() => onSiguiente()}
                disabled={pagina >= totalPaginas}
              >
                Siguiente
              </Button>
            </div>
          </div>
        )}
        </>
      )}
    </>
  );
}
