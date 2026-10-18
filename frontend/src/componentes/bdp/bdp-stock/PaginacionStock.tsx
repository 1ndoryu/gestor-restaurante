/* Paginación del stock: página actual, tamaño de página y contador de artículos mostrados.
 * Sin estado propio: página y tamaño llegan del hook useBdpStockFilters vía BdpStock. */
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import type { useBdpStockFilters } from '@/hooks/useBdpStockFilters';
import { PAGE_SIZES } from '../bdp-stock-utils';

type Filtros = ReturnType<typeof useBdpStockFilters>;
type PaginacionStockProps = Pick<Filtros, 'page' | 'totalPages' | 'setPage' | 'pageSize' | 'setPageSize'> & {
  mostrados: number;
  total: number;
};

export function PaginacionStock({ mostrados, total, page, totalPages, setPage, pageSize, setPageSize }: PaginacionStockProps) {
  return (
    <div className="flex flex-col sm:flex-row items-center justify-between gap-3 text-sm">
      <span className="text-muted-foreground">
        Mostrando {mostrados} de {total} artículos
      </span>
      <div className="flex items-center gap-2">
        <Button variant="outline" size="sm" onClick={() => setPage((p) => Math.max(1, p - 1))} disabled={page <= 1}>
          Anterior
        </Button>
        <span className="text-muted-foreground">
          Página {page} de {totalPages}
        </span>
        <Button
          variant="outline"
          size="sm"
          onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
          disabled={page >= totalPages}
        >
          Siguiente
        </Button>
      </div>
      <Select
        value={String(pageSize)}
        onValueChange={(v) => {
          setPageSize(Number(v) as 10 | 25 | 50);
          setPage(1);
        }}
      >
        <SelectTrigger className="w-20">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {PAGE_SIZES.map((s) => (
            <SelectItem key={s} value={String(s)}>
              {s}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
