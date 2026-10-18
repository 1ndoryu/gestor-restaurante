import type { Dispatch, SetStateAction } from 'react';
import { Button } from '@/components/ui/button';

interface Props {
  total: number;
  paginaActual: number;
  totalPaginas: number;
  setPagina: Dispatch<SetStateAction<number>>;
}

/* Pie de la tabla de mapeos: contador y paginación (la paginación es en cliente). */
export function BdpArticleMapPaginacion({ total, paginaActual, totalPaginas, setPagina }: Props) {
  return (
    <div className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between">
      <span className="text-xs text-muted-foreground">
        {total} artículo{total === 1 ? '' : 's'}
        {totalPaginas > 1 && ` · página ${paginaActual + 1} de ${totalPaginas}`}
      </span>
      {totalPaginas > 1 && (
        <div className="flex items-center gap-2">
          <Button
            size="sm"
            variant="outline"
            disabled={paginaActual === 0}
            onClick={() => setPagina((p) => Math.max(0, p - 1))}
          >
            Anterior
          </Button>
          <Button
            size="sm"
            variant="outline"
            disabled={paginaActual >= totalPaginas - 1}
            onClick={() => setPagina((p) => Math.min(totalPaginas - 1, p + 1))}
          >
            Siguiente
          </Button>
        </div>
      )}
    </div>
  );
}
