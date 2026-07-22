/* [263A-16] Lista de gastos — reescrita con shadcn Table + Dialog + Button.
 * Iconos para eliminar (tarea 17). Columnas en flex (tarea 18).
 * [283A-22] Botón de edición con Dialog reutilizando FormularioGasto.
 * [283A-34] Filtros de fecha (desde/hasta) via hook useListaGastos.
 * [044A-8+9] Buscador + cabeceras de columna clicables para ordenar.
 * [064A-3] Filtros por columna en Tipo doc y Método con ColumnFilterHeader. */

import { Button } from '@/components/ui/button';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Trash2, Pencil } from 'lucide-react';
import FormularioGasto from './FormularioGasto';
import useListaGastos from '@/hooks/useListaGastos';
import ColumnFilterHeader from '@/components/column-filter-header';

function formatearMoneda(valor: string): string {
  return new Intl.NumberFormat('es-ES', { style: 'currency', currency: 'EUR' }).format(parseFloat(valor));
}

function ListaGastos() {
  const {
    filtros, cambiarFiltro, toggleSort, cambiarFiltroColumna,
    modalAbierto, setModalAbierto,
    gastoEditando, setGastoEditando,
    porPagina, gastos, isLoading,
    eliminarMutation,
    cerrarModalYRefrescar,
    cerrarEdicionYRefrescar,
  } = useListaGastos();

  return (
    <div className="flex flex-col gap-4">
      {/* [044A-12] Contador y botón en fila separada, consistente con ListaVentas/ListaReservas */}
      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">{gastos ? `${gastos.total} registros` : ''}</p>
        <Button onClick={() => setModalAbierto(true)}>+ Nuevo Gasto</Button>
      </div>

      {/* [044A-9] Buscador + filtros de fecha */}
      <div className="flex flex-wrap gap-3 items-center">
        <Input
          type="search"
          placeholder="Buscar por proveedor, tipo documento, número..."
          value={filtros.busqueda}
          onChange={(e) => cambiarFiltro('busqueda', e.target.value)}
          className="max-w-xs"
        />
        <label className="text-sm text-muted-foreground">Desde</label>
        <Input type="date" className="max-w-40" value={filtros.desde} onChange={e => cambiarFiltro('desde', e.target.value)} />
        <label className="text-sm text-muted-foreground">Hasta</label>
        <Input type="date" className="max-w-40" value={filtros.hasta} onChange={e => cambiarFiltro('hasta', e.target.value)} />
      </div>

      <Dialog open={modalAbierto} onOpenChange={setModalAbierto}>
        {/* [283A-30] sm:max-w-lg = 512px (tarea 18) */}
        <DialogContent className="sm:max-w-lg max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Nuevo Gasto</DialogTitle>
          </DialogHeader>
          <FormularioGasto onExito={cerrarModalYRefrescar} />
        </DialogContent>
      </Dialog>

      {/* [283A-22] Dialog de edición — reutiliza FormularioGasto con gasto pre-cargado */}
      <Dialog open={!!gastoEditando} onOpenChange={(open) => { if (!open) setGastoEditando(null); }}>
        <DialogContent className="sm:max-w-lg max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Editar Gasto</DialogTitle>
          </DialogHeader>
          {gastoEditando && (
            <FormularioGasto onExito={cerrarEdicionYRefrescar} gasto={gastoEditando} />
          )}
        </DialogContent>
      </Dialog>

      {isLoading ? (
        <p className="text-sm text-muted-foreground">Cargando...</p>
      ) : gastos && gastos.items.length > 0 ? (
        <>
          <div className="rounded-md border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="cursor-pointer select-none" onClick={() => toggleSort('fecha')}>
                    Fecha {filtros.sortBy === 'fecha' && (filtros.sortOrder === 'asc' ? '↑' : '↓')}
                  </TableHead>
                  <TableHead className="cursor-pointer select-none" onClick={() => toggleSort('proveedor')}>
                    Proveedor {filtros.sortBy === 'proveedor' && (filtros.sortOrder === 'asc' ? '↑' : '↓')}
                  </TableHead>
                  <TableHead>
                    <ColumnFilterHeader
                      title="Tipo doc."
                      sortKey="tipo_documento"
                      currentSortBy={filtros.sortBy}
                      currentSortOrder={filtros.sortOrder}
                      onSort={() => toggleSort('tipo_documento')}
                      options={[
                        { value: 'factura', label: 'Factura' },
                        { value: 'albaran', label: 'Albarán' },
                        { value: 'ticket', label: 'Ticket' },
                      ]}
                      selectedValues={filtros.tipoDocumento}
                      onFilterChange={(v) => cambiarFiltroColumna('tipoDocumento', v)}
                    />
                  </TableHead>
                  <TableHead>
                    <ColumnFilterHeader
                      title="Método"
                      sortKey="metodo_pago"
                      currentSortBy={filtros.sortBy}
                      currentSortOrder={filtros.sortOrder}
                      onSort={() => toggleSort('metodo_pago')}
                      options={[
                        { value: 'efectivo', label: 'Efectivo' },
                        { value: 'tarjeta', label: 'Tarjeta' },
                        { value: 'transferencia', label: 'Transferencia' },
                      ]}
                      selectedValues={filtros.metodoPago}
                      onFilterChange={(v) => cambiarFiltroColumna('metodoPago', v)}
                    />
                  </TableHead>
                  <TableHead className="text-right cursor-pointer select-none" onClick={() => toggleSort('importe_base')}>
                    Base {filtros.sortBy === 'importe_base' && (filtros.sortOrder === 'asc' ? '↑' : '↓')}
                  </TableHead>
                  <TableHead className="text-right">IVA</TableHead>
                  <TableHead className="text-right">Total</TableHead>
                  <TableHead className="w-20"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {gastos.items.map((g) => (
                  <TableRow key={g.id}>
                    <TableCell>{g.fecha}</TableCell>
                    <TableCell>{g.proveedor || '—'}</TableCell>
                    <TableCell className="capitalize">{g.tipo_documento}</TableCell>
                    <TableCell className="capitalize">{g.metodo_pago}</TableCell>
                    <TableCell className="text-right">{formatearMoneda(g.importe_base)}</TableCell>
                    <TableCell className="text-right">{formatearMoneda(g.importe_iva)}</TableCell>
                    <TableCell className="text-right font-medium">{formatearMoneda((parseFloat(g.importe_base) + parseFloat(g.importe_iva)).toFixed(2))}</TableCell>
                    <TableCell>
                      <div className="flex gap-1">
                        <TooltipButton
                          variant="ghost"
                          size="icon"
                          onClick={() => setGastoEditando(g)}
                          tooltip="Editar gasto"
                        >
                          <Pencil className="size-4" />
                        </TooltipButton>
                        <TooltipButton
                          variant="ghost"
                          size="icon"
                          onClick={() => eliminarMutation.mutate({ id: g.id })}
                          disabled={eliminarMutation.isPending}
                          tooltip="Eliminar gasto"
                        >
                          <Trash2 className="size-4 text-destructive" />
                        </TooltipButton>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>

          <div className="flex items-center justify-between">
            <Button variant="outline" size="sm" disabled={filtros.pagina <= 1} onClick={() => cambiarFiltro('pagina', filtros.pagina - 1)}>Anterior</Button>
            <span className="text-sm text-muted-foreground">Página {filtros.pagina} de {Math.ceil(gastos.total / porPagina)}</span>
            <Button variant="outline" size="sm" disabled={filtros.pagina * porPagina >= gastos.total} onClick={() => cambiarFiltro('pagina', filtros.pagina + 1)}>Siguiente</Button>
          </div>
        </>
      ) : (
        <p className="text-sm text-muted-foreground">No hay gastos registrados</p>
      )}
    </div>
  );
}

export default ListaGastos;
