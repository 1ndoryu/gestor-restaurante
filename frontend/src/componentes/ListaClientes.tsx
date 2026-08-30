/* [263A-16] Lista de clientes — reescrita con shadcn Table + Dialog + Input.
 * [263A-26] Agregado: seleccionar 2 clientes y fusionarlos (merge).
 * CRM con búsqueda, paginación, modal crear/editar, merge duplicados.
 * [044A-8] Cabeceras de columna clicables para ordenar. */

import { useState } from 'react';
import useListaClientes from '../hooks/useListaClientes';
import { Button } from '@/components/ui/button';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { Input } from '@/components/ui/input';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter, DialogDescription } from '@/components/ui/dialog';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Checkbox } from '@/components/ui/checkbox';
import { Trash2, Pencil, Merge, Download, MoreHorizontal, Link2 } from 'lucide-react';
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import FormularioCliente from './FormularioCliente';
import { Badge } from '@/components/ui/badge';
import { Label } from '@/components/ui/label';
import { useObtenerConfiguracion } from '@/api/generated/configuracion/configuracion';
import { useIntegracionBdp } from '../hooks/useIntegracionBdp';

function ListaClientes() {
  const {
    pagina,
    setPagina,
    busqueda,
    buscar,
    sortBy,
    sortOrder,
    toggleSort,
    modalCrear,
    setModalCrear,
    clienteEditar,
    setClienteEditar,
    porPagina,
    clientes,
    isLoading,
    eliminarMut,
    cerrarModalYRefrescar,
    seleccionados,
    toggleSeleccion,
    modalMerge,
    setModalMerge,
    mergeMut,
  } = useListaClientes();

  /* Estado de integración BDP para el aviso de «Importar BDP» (duda 3/8 de Guillermo). */
  const { data: configData } = useObtenerConfiguracion();
  const bdpSyncEnabled = configData?.status === 200
    ? Boolean((configData.data as unknown as Record<string, unknown>).bdp_sync_enabled ?? false)
    : false;

  /* [263A-26] En el diálogo de merge el usuario elige quién sobrevive (destino) */
  const [destinoId, setDestinoId] = useState<string | null>(null);

  const clientesSeleccionados = clientes?.items.filter((c) => seleccionados.includes(c.id)) ?? [];

  /* Estado y handlers de la integración BDP (vincular/importar) viven en un hook
   * custom para no superar el límite de useState (protocolo usestate-excesivo). */
  const {
    clienteBdp,
    setClienteBdp,
    codigoBdp,
    setCodigoBdp,
    confirmacionBdp,
    setConfirmacionBdp,
    sincronizandoBdp,
    importarBdpAbierto,
    setImportarBdpAbierto,
    importandoBdp,
    confirmacionImportar,
    setConfirmacionImportar,
    previewImportar,
    setPreviewImportar,
    sincronizarClienteBdp,
    importarClientesBdp,
  } = useIntegracionBdp(cerrarModalYRefrescar);

  const ejecutarMerge = () => {
    if (seleccionados.length !== 2 || !destinoId) return;
    const origenId = seleccionados.find((id) => id !== destinoId);
    if (!origenId) return;
    mergeMut.mutate({ data: { origen_id: origenId, destino_id: destinoId } });
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <p className="text-sm text-muted-foreground">{clientes ? `${clientes.total} registros` : ''}</p>
          {seleccionados.length === 2 && (
            <Button
              variant="outline"
              onClick={() => { setDestinoId(null); setModalMerge(true); }}
            >
              <Merge className="size-4 mr-1.5" /> Fusionar seleccionados
            </Button>
          )}
          {seleccionados.length > 0 && seleccionados.length < 2 && (
            <span className="text-xs text-muted-foreground">Selecciona otro cliente para fusionar</span>
          )}
        </div>
        <div className="flex gap-2">
          <TooltipButton variant="outline" tooltip="Importar clientes desde BDP" onClick={() => { setImportarBdpAbierto(true); setPreviewImportar(null); setConfirmacionImportar(''); }}><Download className="size-4 mr-1" />Importar BDP</TooltipButton>
          <Button onClick={() => setModalCrear(true)}>+ Nuevo Cliente</Button>
        </div>
      </div>

      {!bdpSyncEnabled && (
        <p className="rounded-md border border-amber-300/70 bg-amber-50 px-3 py-2 text-xs text-amber-900 dark:border-amber-800 dark:bg-amber-950/30 dark:text-amber-200">
          Integración BDP desactivada: «Importar BDP» solo lee de BDP (no escribe clientes en BDP),
          pero conviene activar la integración en Configuración para que los vínculos se mantengan
          coherentes con el resto de la Aplicación Web.
        </p>
      )}

      <Input
        type="search"
        placeholder="Buscar por nombre, apellidos, teléfono, email, empresa o notas..."
        value={busqueda}
        onChange={(e) => buscar(e.target.value)}
        className="max-w-md"
      />

      <Dialog open={modalCrear} onOpenChange={setModalCrear}>
        <DialogContent className="sm:max-w-lg max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Nuevo Cliente</DialogTitle>
          </DialogHeader>
          <FormularioCliente onExito={cerrarModalYRefrescar} />
        </DialogContent>
      </Dialog>

      <Dialog open={!!clienteEditar} onOpenChange={(open) => { if (!open) setClienteEditar(null); }}>
        <DialogContent className="sm:max-w-lg max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Editar Cliente</DialogTitle>
          </DialogHeader>
          {clienteEditar && <FormularioCliente onExito={cerrarModalYRefrescar} cliente={clienteEditar} />}
        </DialogContent>
      </Dialog>

      {isLoading ? (
        <p className="text-sm text-muted-foreground">Cargando...</p>
      ) : clientes && clientes.items.length > 0 ? (
        <>
          <div className="rounded-md border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-10"></TableHead>
                  <TableHead className="cursor-pointer select-none" onClick={() => toggleSort('nombre')}>
                    Nombre {sortBy === 'nombre' && (sortOrder === 'asc' ? '↑' : '↓')}
                  </TableHead>
                  <TableHead className="cursor-pointer select-none" onClick={() => toggleSort('telefono')}>
                    Teléfono {sortBy === 'telefono' && (sortOrder === 'asc' ? '↑' : '↓')}
                  </TableHead>
                  <TableHead className="cursor-pointer select-none" onClick={() => toggleSort('email')}>
                    Email {sortBy === 'email' && (sortOrder === 'asc' ? '↑' : '↓')}
                  </TableHead>
                  <TableHead className="cursor-pointer select-none" onClick={() => toggleSort('empresa')}>
                    Empresa {sortBy === 'empresa' && (sortOrder === 'asc' ? '↑' : '↓')}
                  </TableHead>
                  <TableHead>Notas</TableHead>
                  <TableHead>BDP</TableHead>
                  <TableHead className="w-20 text-center">Acciones</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {clientes.items.map((c) => (
                  <TableRow key={c.id} className={seleccionados.includes(c.id) ? 'bg-muted/50' : ''}>
                    <TableCell>
                      <Checkbox
                        checked={seleccionados.includes(c.id)}
                        onCheckedChange={() => toggleSeleccion(c.id)}
                        disabled={!seleccionados.includes(c.id) && seleccionados.length >= 2}
                      />
                    </TableCell>
                    <TableCell>{c.nombre} {c.apellidos}</TableCell>
                    <TableCell>{c.telefono ? `${c.prefijo_telefono} ${c.telefono}` : '—'}</TableCell>
                    <TableCell>{c.email || '—'}</TableCell>
                    <TableCell>{c.empresa || '—'}</TableCell>
                    <TableCell className="max-w-32 truncate">{c.notas || '—'}</TableCell>
                    <TableCell>
                      {c.bdp_synced ? (
                        <Badge variant="outline">Código {c.bdp_customer_code}</Badge>
                      ) : c.bdp_sync_error ? (
                        <Badge variant="destructive" title={c.bdp_sync_error}>Error</Badge>
                      ) : (
                        <Badge variant="secondary">Sin vincular</Badge>
                      )}
                    </TableCell>
                    <TableCell>
                      {/* [208A-3] El cliente sin vínculo BDP tiene 3 acciones
                       * (Editar/Vincular/Eliminar) → menú de 3 puntos (regla global).
                       * El botón «Vincular a BDP» es una acción (abre el diálogo de
                       * vinculación), no un estado: el estado está en su columna. */}
                      <div className="flex items-center justify-center">
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button variant="outline" size="icon" aria-label="Acciones del cliente" className="bg-muted/40 hover:bg-muted">
                              <MoreHorizontal className="size-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end" className="w-52">
                            <DropdownMenuItem onClick={() => setClienteEditar(c)}>
                              <Pencil className="mr-2 size-3.5" />
                              Editar
                            </DropdownMenuItem>
                            {!c.bdp_synced && (
                              <DropdownMenuItem
                                onClick={() => { setClienteBdp(c); setCodigoBdp(''); setConfirmacionBdp(''); }}
                              >
                                <Link2 className="mr-2 size-3.5" />
                                Vincular a BDP
                              </DropdownMenuItem>
                            )}
                            <DropdownMenuSeparator />
                            <DropdownMenuItem
                              variant="destructive"
                              onClick={() => eliminarMut.mutate({ id: c.id })}
                              disabled={eliminarMut.isPending}
                            >
                              <Trash2 className="mr-2 size-3.5" />
                              Eliminar
                            </DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>

          <div className="flex items-center justify-between">
            <Button variant="outline" size="sm" disabled={pagina <= 1} onClick={() => setPagina(pagina - 1)}>Anterior</Button>
            <span className="text-sm text-muted-foreground">Página {pagina} de {Math.ceil(clientes.total / porPagina)}</span>
            <Button variant="outline" size="sm" disabled={pagina * porPagina >= clientes.total} onClick={() => setPagina(pagina + 1)}>Siguiente</Button>
          </div>
        </>
      ) : (
        <p className="text-sm text-muted-foreground">No hay clientes registrados</p>
      )}

      {/* [263A-26] Diálogo de merge: el usuario elige cuál de los 2 sobrevive */}
      <Dialog open={modalMerge} onOpenChange={(open) => { if (!open) { setModalMerge(false); } }}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>Fusionar clientes</DialogTitle>
            <DialogDescription>
              Selecciona cuál de los dos clientes debe sobrevivir. El otro será absorbido: sus reservas, etiquetas y campañas se migrarán, y sus campos vacíos se completarán.
            </DialogDescription>
          </DialogHeader>
          <div className="flex flex-col gap-3">
            {clientesSeleccionados.map((c) => (
              <button
                key={c.id}
                type="button"
                onClick={() => setDestinoId(c.id)}
                className={`flex items-center gap-3 rounded-md border p-3 text-left transition-colors ${
                  destinoId === c.id ? 'border-primary bg-primary/10' : 'border-border hover:bg-muted/50'
                }`}
              >
                <div className="flex-1">
                  <p className="font-medium">{c.nombre} {c.apellidos}</p>
                  <p className="text-sm text-muted-foreground">
                    {[c.telefono, c.email, c.empresa].filter(Boolean).join(' · ') || 'Sin datos adicionales'}
                  </p>
                </div>
                {destinoId === c.id && (
                  <span className="text-xs font-semibold text-primary">SOBREVIVE</span>
                )}
                {destinoId && destinoId !== c.id && (
                  <span className="text-xs text-destructive">SE ELIMINA</span>
                )}
              </button>
            ))}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setModalMerge(false)}>Cancelar</Button>
            <Button
              disabled={!destinoId || mergeMut.isPending}
              onClick={ejecutarMerge}
            >
              {mergeMut.isPending ? 'Fusionando...' : 'Confirmar fusión'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={!!clienteBdp} onOpenChange={(open) => { if (!open) setClienteBdp(null); }}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Vincular cliente con BDP</DialogTitle>
            <DialogDescription>
              Indica un código BDP reservado. El servidor verificará primero que no pertenezca a otra identidad y siempre enviará Overwrite=false.
            </DialogDescription>
          </DialogHeader>
          <div className="flex flex-col gap-3">
            <div className="flex flex-col gap-2">
              <Label htmlFor="codigo-bdp-cliente">Código BDP explícito</Label>
              <Input id="codigo-bdp-cliente" type="number" min={1} value={codigoBdp} onChange={(e) => setCodigoBdp(e.target.value)} />
            </div>
            <div className="flex flex-col gap-2">
              <Label htmlFor="confirmar-bdp-cliente">Escribe CREAR CLIENTE {clienteBdp?.nombre} {clienteBdp?.apellidos} {codigoBdp || '<código>'}</Label>
              <Input id="confirmar-bdp-cliente" value={confirmacionBdp} onChange={(e) => setConfirmacionBdp(e.target.value)} />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setClienteBdp(null)}>Cancelar</Button>
            <Button disabled={!codigoBdp || confirmacionBdp !== `CREAR CLIENTE ${clienteBdp?.nombre} ${clienteBdp?.apellidos} ${codigoBdp}` || sincronizandoBdp} onClick={sincronizarClienteBdp}>
              {sincronizandoBdp ? 'Verificando…' : 'Verificar y vincular'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={importarBdpAbierto} onOpenChange={setImportarBdpAbierto}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader><DialogTitle>Importar clientes desde BDP</DialogTitle><DialogDescription>Primero previsualiza. Esta operación solo lee BDP y, al aplicar, modifica la Aplicación Web; nunca escribe clientes en BDP.</DialogDescription></DialogHeader>
          {previewImportar && (
            <div className="grid grid-cols-2 gap-2 rounded-md border p-3 text-sm">
              <span>Nuevos: {previewImportar.imported}</span><span>Vínculos: {previewImportar.updated}</span>
              <span>Sin cambios: {previewImportar.unchanged}</span><span>Conflictos: {previewImportar.conflicts}</span>
              <span>Inválidos: {previewImportar.errors}</span><span>Total: {previewImportar.total}</span>
            </div>
          )}
          {previewImportar && (previewImportar.conflicts > 0 || previewImportar.errors > 0) && <p className="text-sm text-destructive">Hay registros no aplicables. Se omitirán; no se sobrescribirá ningún vínculo existente.</p>}
          {previewImportar && <div><Label htmlFor="confirmar-importar-bdp">Escribe IMPORTAR CLIENTES BDP</Label><Input id="confirmar-importar-bdp" value={confirmacionImportar} onChange={(e) => setConfirmacionImportar(e.target.value)} /></div>}
          <DialogFooter>
            <Button variant="outline" onClick={() => setImportarBdpAbierto(false)}>Cancelar</Button>
            <Button variant="secondary" disabled={importandoBdp} onClick={() => importarClientesBdp(false)}>{importandoBdp ? 'Consultando…' : 'Previsualizar sin cambios'}</Button>
            {previewImportar && <Button disabled={importandoBdp || confirmacionImportar !== 'IMPORTAR CLIENTES BDP'} onClick={() => importarClientesBdp(true)}>Aplicar en la Aplicación Web</Button>}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

export default ListaClientes;
