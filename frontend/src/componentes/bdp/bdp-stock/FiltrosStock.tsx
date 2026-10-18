/* Barra de filtros del stock: búsqueda por código o nombre, con/sin stock y activo/inactivo.
 * Sin estado propio: los valores y setters llegan del hook useBdpStockFilters vía BdpStock. */
import { Search } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import type { useBdpStockFilters } from '@/hooks/useBdpStockFilters';

type Filtros = ReturnType<typeof useBdpStockFilters>;
type FiltrosStockProps = Pick<Filtros, 'filtro' | 'setFiltro' | 'stockFilter' | 'setStockFilter' | 'activeFilter' | 'setActiveFilter' | 'setPage'>;

export function FiltrosStock({ filtro, setFiltro, stockFilter, setStockFilter, activeFilter, setActiveFilter, setPage }: FiltrosStockProps) {
  return (
    <div className="flex flex-wrap gap-3 items-center">
      <div className="relative w-full sm:w-64">
        <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
        <Input
          placeholder="Código o nombre..."
          value={filtro}
          onChange={(e) => {
            setFiltro(e.target.value);
            setPage(1);
          }}
          className="pl-9 max-w-xs"
        />
      </div>
      <Select
        value={stockFilter}
        onValueChange={(v) => {
          setStockFilter(v as Filtros['stockFilter']);
          setPage(1);
        }}
      >
        <SelectTrigger className="w-full sm:w-44">
          <SelectValue placeholder="Stock" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="all">Cualquier stock</SelectItem>
          <SelectItem value="with">Con stock</SelectItem>
          <SelectItem value="without">Sin stock</SelectItem>
        </SelectContent>
      </Select>
      <Select
        value={activeFilter}
        onValueChange={(v) => {
          setActiveFilter(v as Filtros['activeFilter']);
          setPage(1);
        }}
      >
        <SelectTrigger className="w-full sm:w-44">
          <SelectValue placeholder="Estado" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="all">Cualquier estado</SelectItem>
          <SelectItem value="active">Activos</SelectItem>
          <SelectItem value="inactive">Inactivos</SelectItem>
        </SelectContent>
      </Select>
    </div>
  );
}
