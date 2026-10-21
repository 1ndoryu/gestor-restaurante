/* [D26d] Tabla de stock con sus acciones por fila (ajustar y traspasar).
 * Se extrae de BdpStock para que la página quepa en el límite de líneas. */
import { Package, SlidersHorizontal, ArrowRightLeft, ChevronDown, ChevronUp } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import type { BdpArticleMap } from '@/api/generated/gestionRestauranteAPI.schemas';
import { formatPrice, formatStock, type SortKey } from '../bdp-stock-utils';

export function TablaStock({
  filas,
  stockPorCodigo,
  codigosStockLocal,
  sortKey,
  sortDir,
  onOrdenar,
  onAjustar,
  onTraspasar,
}: {
  filas: BdpArticleMap[];
  stockPorCodigo: Map<string, string>;
  codigosStockLocal: Set<string>;
  sortKey: SortKey;
  sortDir: 'asc' | 'desc';
  onOrdenar: (clave: SortKey) => void;
  onAjustar: (articulo: BdpArticleMap) => void;
  onTraspasar: (articulo: BdpArticleMap) => void;
}) {
  function SortIcon({ column }: { column: SortKey }) {
    if (sortKey !== column) return <span className="inline-block w-3" />;
    return sortDir === 'asc' ? <ChevronUp className="size-3" /> : <ChevronDown className="size-3" />;
  }

  return (
          <div className="rounded-md border overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="cursor-pointer" onClick={() => onOrdenar('articulo_glory_codigo')}>
                    <span className="flex items-center gap-1">Código Aplicación Web <SortIcon column="articulo_glory_codigo" /></span>
                  </TableHead>
                  <TableHead className="cursor-pointer" onClick={() => onOrdenar('articulo_bdp_codigo')}>
                    <span className="flex items-center gap-1">Código BDP <SortIcon column="articulo_bdp_codigo" /></span>
                  </TableHead>
                  <TableHead className="cursor-pointer" onClick={() => onOrdenar('articulo_bdp_nombre')}>
                    <span className="flex items-center gap-1">Nombre BDP <SortIcon column="articulo_bdp_nombre" /></span>
                  </TableHead>
                  <TableHead className="cursor-pointer" onClick={() => onOrdenar('precio_tarifa1')}>
                    <span className="flex items-center gap-1">Precio <SortIcon column="precio_tarifa1" /></span>
                  </TableHead>
                  <TableHead className="cursor-pointer" onClick={() => onOrdenar('stock_actual')}>
                    <span className="flex items-center gap-1">Stock <SortIcon column="stock_actual" /></span>
                  </TableHead>
                  <TableHead className="text-center">Acciones</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filas.map((m) => {
                  const stockLocalVal = stockPorCodigo.get(m.articulo_glory_codigo);
                  const origen = codigosStockLocal.has(m.articulo_glory_codigo) ? 'local' : 'bdp';
                  const stock = formatStock(stockLocalVal ?? m.stock_actual);
                  return (
                    <TableRow key={m.id}>
                      <TableCell className="font-mono text-xs">{m.articulo_glory_codigo || '—'}</TableCell>
                      <TableCell className="font-mono text-xs">{m.articulo_bdp_codigo || '—'}</TableCell>
                      <TableCell className="text-xs">{m.articulo_bdp_nombre || '—'}</TableCell>
                      <TableCell className="text-xs tabular-nums">{formatPrice(m.precio_tarifa1)}</TableCell>
                      <TableCell>
                        {stock.hasStock ? (
                          <span className="inline-flex items-center gap-1">
                            <Badge variant={origen === 'local' ? 'secondary' : 'outline'} className="gap-1">
                              <Package className="size-3" />
                              {stock.text}
                            </Badge>
                            <Badge variant={origen === 'local' ? 'secondary' : 'outline'}>
                              {origen}
                            </Badge>
                          </span>
                        ) : (
                          <span className="text-xs text-muted-foreground">—</span>
                        )}
                      </TableCell>
                      <TableCell>
                        <div className="flex justify-center gap-2">
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => onAjustar(m)}
                          >
                            <SlidersHorizontal className="size-3" />
                            Ajustar
                          </Button>
                          {/* Sin código BDP numérico no hay traspaso posible: BDP
                              identifica el artículo por ese número. */}
                          <Button
                            variant="outline"
                            size="sm"
                            disabled={!/^\d+$/.test((m.articulo_bdp_codigo ?? '').trim())}
                            title={'Mover unidades entre almacenes de BDP'}
                            onClick={() => onTraspasar(m)}
                          >
                            <ArrowRightLeft className="size-3" />
                            Traspasar
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          </div>

  );
}
