/* [BDP-EXPLORER-02] Explorador de menús, packs y fastfoods BDP.
 * Estructura coherente con ListaVentas/ListaGastos/ListaReservas.
 * En modo real permite consultar por código numérico a BDP.
 * Modo demo muestra una tabla con datos de ejemplo. */

import { useMemo, useState } from 'react';
import { Search, Loader2, Eye, Pencil, Plus, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { ErrorResponse } from '@/api/generated/gestionRestauranteAPI.schemas';
import {
  useGetMenuDefinition,
  useGetFastfoodDefinition,
  useGetPackDefinition,
} from '@/api/generated/bdp-mapeos/bdp-mapeos';
import {
  useBdpMenusLocales,
  useCrearBdpMenuLocal,
  useActualizarBdpMenuLocal,
  useEliminarBdpMenuLocal,
} from '@/api/bdp';
import type {
  ActualizarBdpMenuLocalRequest,
  BdpMenuLocalConLineas,
  CrearBdpMenuLocalRequest,
} from '@/api/bdp';
import { useBdpDemoMode } from '@/hooks/useBdpDemoMode';
import { mockExplorerItems, type BdpExplorerItem } from './bdp-mocks';
import { BdpDemoToggle } from './BdpDemoToggle';
import { BdpMenuLocalModal } from './BdpMenuLocalModal';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

type ExploreType = 'all' | 'menu' | 'fastfood' | 'pack';

const TIPOS: { value: ExploreType; label: string }[] = [
  { value: 'all', label: 'Todos' },
  { value: 'menu', label: 'Menú' },
  { value: 'fastfood', label: 'Fastfood' },
  { value: 'pack', label: 'Pack' },
];

function tipoLabel(tipo: string) {
  return TIPOS.find((t) => t.value === tipo)?.label ?? tipo;
}

interface BdpDefLine {
  article_code?: string | number;
  article_name?: string;
  quantity?: number;
}

interface BdpDefinition {
  code?: string | number;
  name?: string;
  description?: string;
  lines?: BdpDefLine[];
}

function DefinitionDetail({ data }: { data: BdpExplorerItem }) {
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

function BdpExplorador() {
  const queryClient = useQueryClient();
  const { demoMode, setDemoMode } = useBdpDemoMode();
  const [tipo, setTipo] = useState<ExploreType>('all');
  const [busqueda, setBusqueda] = useState('');
  const [identificador, setIdentificador] = useState('');
  const [buscado, setBuscado] = useState(false);
  const [seleccionado, setSeleccionado] = useState<BdpExplorerItem | null>(null);
  const [localModalOpen, setLocalModalOpen] = useState(false);
  const [localModalMenu, setLocalModalMenu] = useState<BdpMenuLocalConLineas | null>(null);

  /* [128A-1/F7] Menús/packs locales: CRUD local, siempre disponible sin BDP. */
  const { data: menusLocales, isLoading: isLoadingMenus } = useBdpMenusLocales();
  const crearMutation = useCrearBdpMenuLocal(queryClient);
  const actualizarMutation = useActualizarBdpMenuLocal(queryClient);
  const eliminarMutation = useEliminarBdpMenuLocal(queryClient);

  const idNum = Number(identificador);
  const idValido = Number.isInteger(idNum) && idNum > 0;

  const menuQuery = useGetMenuDefinition(idNum, {
    query: { enabled: buscado && tipo === 'menu' && !demoMode },
  });
  const fastfoodQuery = useGetFastfoodDefinition(idNum, {
    query: { enabled: buscado && tipo === 'fastfood' && !demoMode },
  });
  const packQuery = useGetPackDefinition(idNum, {
    query: { enabled: buscado && tipo === 'pack' && !demoMode },
  });

  const isLoading = menuQuery.isLoading || fastfoodQuery.isLoading || packQuery.isLoading;
  const error = menuQuery.error || fastfoodQuery.error || packQuery.error;

  function extraerMensajeError(err: unknown): string {
    if (typeof err === 'string') return err;
    if (err instanceof Error) return err.message;
    const er = err as ErrorResponse | undefined;
    if (er?.message) return er.message;
    return 'Error al consultar. Verifica que el código existe en BDP y que la integración está activa.';
  }

  const items = useMemo(() => {
    if (!demoMode) return [];
    return mockExplorerItems.filter((item) => {
      const matchesTipo = tipo === 'all' || item.type === tipo;
      const q = busqueda.trim().toLowerCase();
      const matchesText =
        !q ||
        item.name.toLowerCase().includes(q) ||
        item.code.toLowerCase().includes(q) ||
        item.description.toLowerCase().includes(q);
      return matchesTipo && matchesText;
    });
  }, [demoMode, tipo, busqueda]);

  /* [287A-4] Cada tipo consulta exclusivamente su endpoint. Esto evita que
   * Fastfood y Pack disparen también GetMenu y muestren una respuesta ajena. */
  const resultadoReal = buscado
    ? tipo === 'menu'
      ? (menuQuery.data as { data?: BdpDefinition })?.data
      : tipo === 'fastfood'
        ? (fastfoodQuery.data as { data?: BdpDefinition })?.data
        : tipo === 'pack'
          ? (packQuery.data as { data?: BdpDefinition })?.data
          : null
    : null;

  function buscar() {
    if (!idValido) {
      toast.warning('Introduce un código numérico válido');
      return;
    }
    if (tipo === 'all') {
      toast.warning('Selecciona un tipo (Menú, Fastfood o Pack)');
      return;
    }
    setBuscado(true);
  }

  function formatPrecio(value: string) {
    const n = Number(value);
    if (Number.isNaN(n)) return value;
    return new Intl.NumberFormat('es-ES', { style: 'currency', currency: 'EUR' }).format(n);
  }

  function abrirNuevo() {
    setLocalModalMenu(null);
    setLocalModalOpen(true);
  }

  function abrirEdicion(menu: BdpMenuLocalConLineas) {
    setLocalModalMenu(menu);
    setLocalModalOpen(true);
  }

  function handleMenuSubmit(req: CrearBdpMenuLocalRequest | ActualizarBdpMenuLocalRequest) {
    if (localModalMenu) {
      actualizarMutation.mutate(
        { id: localModalMenu.id, req },
        {
          onSuccess: () => {
            toast.success('Menú/pack actualizado');
            setLocalModalOpen(false);
          },
          onError: () => toast.error('No se pudo actualizar el menú/pack'),
        },
      );
    } else {
      crearMutation.mutate(req as CrearBdpMenuLocalRequest, {
        onSuccess: () => {
          toast.success('Menú/pack creado');
          setLocalModalOpen(false);
        },
        onError: () => toast.error('No se pudo crear el menú/pack'),
      });
    }
  }

  function handleEliminar(menu: BdpMenuLocalConLineas) {
    if (demoMode) {
      toast.info('En modo demo no se borran datos reales');
      return;
    }
    if (!window.confirm(`¿Eliminar «${menu.nombre}»?`)) return;
    eliminarMutation.mutate(menu.id, {
      onSuccess: () => toast.success('Menú/pack eliminado'),
      onError: () => toast.error('No se pudo eliminar el menú/pack'),
    });
  }

  return (
    <div className="flex flex-col gap-4">
      {/* [128A-1/F7] Sección de menús/packs locales — funciona sin BDP. */}
      <div className="rounded-md border overflow-x-auto">
        <div className="flex items-center justify-between px-4 py-3 border-b">
          <div>
            <h3 className="text-sm font-medium">Menús y packs locales</h3>
            <p className="text-xs text-muted-foreground">
              Agrupaciones del catálogo local. Funcionan sin conexión a BDP.
            </p>
          </div>
          <Button size="sm" onClick={abrirNuevo}>
            <Plus className="mr-1 size-3.5" />
            Nuevo menú/pack
          </Button>
        </div>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Nombre</TableHead>
              <TableHead>Tipo</TableHead>
              <TableHead className="text-right">Precio</TableHead>
              <TableHead className="text-right">Artículos</TableHead>
              <TableHead>Estado</TableHead>
              <TableHead>Origen</TableHead>
              <TableHead className="w-24 text-center">Acciones</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {isLoadingMenus ? (
              <TableRow>
                <TableCell colSpan={7} className="text-center text-[13px] text-muted-foreground">
                  <Loader2 className="size-4 animate-spin inline mr-1" />
                  Cargando menús locales...
                </TableCell>
              </TableRow>
            ) : (menusLocales ?? []).length === 0 ? (
              <TableRow>
                <TableCell colSpan={7} className="text-center text-[13px] text-muted-foreground">
                  No hay menús/packs locales. Crea el primero con «Nuevo menú/pack».
                </TableCell>
              </TableRow>
            ) : (
              (menusLocales ?? []).map((menu) => (
                <TableRow key={menu.id}>
                  <TableCell className="text-[13px] font-medium">{menu.nombre}</TableCell>
                  <TableCell>
                    <Badge variant="outline">{menu.tipo === 'menu' ? 'Menú' : 'Pack'}</Badge>
                  </TableCell>
                  <TableCell className="text-right text-[13px]">{formatPrecio(menu.precio)}</TableCell>
                  <TableCell className="text-right text-[13px]">{menu.lineas.length}</TableCell>
                  <TableCell>
                    {menu.activo ? (
                      <Badge variant="secondary">Activo</Badge>
                    ) : (
                      <Badge variant="outline">Inactivo</Badge>
                    )}
                  </TableCell>
                  <TableCell>
                    <Badge>Local</Badge>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center justify-center gap-1">
                      <Button
                        variant="outline"
                        size="icon"
                        className="bg-muted/40 hover:bg-muted"
                        onClick={() => abrirEdicion(menu)}
                        aria-label={`Editar ${menu.nombre}`}
                      >
                        <Pencil className="size-4" />
                      </Button>
                      <Button
                        variant="outline"
                        size="icon"
                        className="bg-muted/40 hover:bg-muted"
                        onClick={() => handleEliminar(menu)}
                        aria-label={`Eliminar ${menu.nombre}`}
                      >
                        <Trash2 className="size-4 text-destructive" />
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>

      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">
          {demoMode ? `${items.length} definiciones` : 'Consulta una definición de BDP por código'}
        </p>
        <BdpDemoToggle demoMode={demoMode} onToggle={setDemoMode} />
      </div>

      <div className="flex flex-col gap-3 lg:flex-row lg:items-end lg:justify-between">
        {demoMode ? (
          <>
            <div className="flex flex-wrap gap-3 items-center">
              <Input
                type="search"
                placeholder="Buscar nombre, código o descripción..."
                value={busqueda}
                onChange={(e) => setBusqueda(e.target.value)}
                className="max-w-xs"
              />
              <Select
                value={tipo}
                onValueChange={(v) => setTipo(v as ExploreType)}
              >
                <SelectTrigger className="w-full sm:w-44">
                  <SelectValue placeholder="Tipo" />
                </SelectTrigger>
                <SelectContent>
                  {TIPOS.map((t) => (
                    <SelectItem key={t.value} value={t.value}>
                      {t.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </>
        ) : (
          <div className="flex flex-wrap gap-3 items-end">
            <Select
              value={tipo}
              onValueChange={(v) => {
                setTipo(v as ExploreType);
                setBuscado(false);
              }}
            >
              <SelectTrigger className="w-full sm:w-44">
                <SelectValue placeholder="Tipo" />
              </SelectTrigger>
              <SelectContent>
                {TIPOS.filter((t) => t.value !== 'all').map((t) => (
                  <SelectItem key={t.value} value={t.value}>
                    {t.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Input
              type="number"
              min={1}
              className="w-36"
              placeholder="Código numérico..."
              value={identificador}
              onChange={(e) => {
                setIdentificador(e.target.value);
                setBuscado(false);
              }}
            />
            <Button onClick={buscar} disabled={isLoading}>
              {isLoading ? (
                <Loader2 className="size-3.5 animate-spin mr-1" />
              ) : (
                <Search className="size-3.5 mr-1" />
              )}
              Consultar
            </Button>
          </div>
        )}
      </div>

      {!demoMode && !buscado && (
        <p className="text-sm text-muted-foreground">
          Selecciona un tipo e introduce un código numérico para consultar BDP, o pulsa Cargar demo para ver datos de
          ejemplo.
        </p>
      )}

      {error && !demoMode && <p className="text-sm text-destructive">{extraerMensajeError(error)}</p>}

      {!demoMode && buscado && !isLoading && !resultadoReal && (
        <p className="text-sm text-muted-foreground">
          No se encontró un {tipo} con código {identificador}.
        </p>
      )}

      {!demoMode && resultadoReal && (
        <div className="rounded-md border p-3 text-sm space-y-3">
          <div className="flex items-center gap-2">
            <span className="font-medium">{resultadoReal.name ?? 'Sin nombre'}</span>
            <Badge variant="outline">{tipoLabel(tipo)}</Badge>
          </div>
          {resultadoReal.description && (
            <p className="text-xs text-muted-foreground">{resultadoReal.description}</p>
          )}
          <div className="grid grid-cols-2 gap-2 text-xs">
            <span>
              Código: <Badge variant="outline">{resultadoReal.code ?? '—'}</Badge>
            </span>
            <span>
              Artículos: <Badge variant="secondary">{resultadoReal.lines?.length ?? 0}</Badge>
            </span>
          </div>
          {resultadoReal.lines && resultadoReal.lines.length > 0 && (
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
                  {resultadoReal.lines.map((line) => (
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
      )}

      {demoMode && (
        <>
          <div className="rounded-md border overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Código</TableHead>
                  <TableHead>Nombre</TableHead>
                  <TableHead>Tipo</TableHead>
                  <TableHead>Descripción</TableHead>
                  <TableHead className="w-10 text-center">Acciones</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {items.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={5} className="text-center text-[13px] text-muted-foreground">
                      No hay definiciones que coincidan.
                    </TableCell>
                  </TableRow>
                ) : (
                  items.map((item) => (
                    <TableRow key={item.id}>
                      <TableCell className="font-mono text-xs">{item.code}</TableCell>
                      <TableCell className="text-[13px] font-medium">{item.name}</TableCell>
                      <TableCell>
                        <Badge variant="outline">{tipoLabel(item.type)}</Badge>
                      </TableCell>
                      <TableCell className="text-xs text-muted-foreground max-w-xs truncate">
                        {item.description}
                      </TableCell>
                      <TableCell>
                        <div className="flex justify-center">
                          <Button variant="outline" size="icon" className="bg-muted/40 hover:bg-muted" onClick={() => setSeleccionado(item)}>
                            <Eye className="size-4" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))
                )}
              </TableBody>
            </Table>
          </div>
        </>
      )}

      <Dialog open={!!seleccionado} onOpenChange={(open) => !open && setSeleccionado(null)}>
        <DialogContent className="max-w-2xl max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Detalle de definición BDP</DialogTitle>
            <DialogDescription>{seleccionado?.name}</DialogDescription>
          </DialogHeader>
          {seleccionado && <DefinitionDetail data={seleccionado} />}
        </DialogContent>
      </Dialog>

      <BdpMenuLocalModal
        open={localModalOpen}
        menu={localModalMenu}
        isSubmitting={crearMutation.isPending || actualizarMutation.isPending}
        onClose={() => setLocalModalOpen(false)}
        onSubmit={handleMenuSubmit}
      />
    </div>
  );
}

export default BdpExplorador;
