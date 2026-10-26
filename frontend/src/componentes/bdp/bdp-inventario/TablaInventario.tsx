/* Tabla y paginación de la pestaña «Inventario»: filas del catálogo con la cantidad
 * contada editable. Sin estado propio: la página, el filtro y las contadas llegan
 * desde BdpInventario. */
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import type { BdpArticleMap } from '@/api/generated/gestionRestauranteAPI.schemas';

export interface FilaInventario {
  m: BdpArticleMap;
  esperada: number;
  contada: number;
  diferencia: number | null;
}

interface TablaInventarioProps {
  filas: FilaInventario[];
  contadas: Record<string, string>;
  onContada: (codigo: string, valor: string) => void;
}

export function TablaInventario({ filas, contadas, onContada }: TablaInventarioProps) {
  return (
    <div className="rounded-md border overflow-x-auto">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Código</TableHead>
            <TableHead>Nombre</TableHead>
            <TableHead className="text-right">Esperadas</TableHead>
            <TableHead className="text-right">Contadas</TableHead>
            <TableHead className="text-right">Diferencia</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {filas.map(({ m, esperada: e, diferencia }) => (
            <TableRow key={m.id}>
              <TableCell className="font-mono text-xs">{m.articulo_glory_codigo || '—'}</TableCell>
              <TableCell className="text-xs">{m.articulo_bdp_nombre || '—'}</TableCell>
              <TableCell className="text-right tabular-nums">{e.toFixed(0)}</TableCell>
              <TableCell className="text-right">
                <Input
                  type="number"
                  step="any"
                  className="w-24 ml-auto text-right"
                  value={contadas[m.articulo_glory_codigo] ?? ''}
                  onChange={(ev) => onContada(m.articulo_glory_codigo, ev.target.value)}
                  placeholder="0"
                />
              </TableCell>
              <TableCell className={`text-right tabular-nums ${diferencia !== null && diferencia !== 0 ? 'font-semibold text-amber-700' : 'text-muted-foreground'}`}>
                {diferencia === null ? '—' : diferencia === 0 ? '0' : `${diferencia > 0 ? '+' : ''}${diferencia.toFixed(0)}`}
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}

interface PaginacionInventarioProps {
  mostrados: number;
  totalFiltrados: number;
  totalCatalogo: number;
  filtrado: boolean;
  pagina: number;
  totalPaginas: number;
  onAnterior: () => void;
  onSiguiente: () => void;
}

export function PaginacionInventario({
  mostrados, totalFiltrados, totalCatalogo, filtrado, pagina, totalPaginas, onAnterior, onSiguiente,
}: PaginacionInventarioProps) {
  return (
    <div className="flex flex-col sm:flex-row items-center justify-between gap-3 text-sm">
      <span className="text-muted-foreground">
        Mostrando {mostrados} de {totalFiltrados} artículos
        {filtrado ? ` (filtrados de ${totalCatalogo})` : ''}
      </span>
      <div className="flex items-center gap-2">
        <Button variant="outline" size="sm" onClick={onAnterior} disabled={pagina <= 1}>
          Anterior
        </Button>
        <span className="text-muted-foreground">
          Página {pagina} de {totalPaginas}
        </span>
        <Button variant="outline" size="sm" onClick={onSiguiente} disabled={pagina >= totalPaginas}>
          Siguiente
        </Button>
      </div>
    </div>
  );
}
