/* Sección «Menús y packs locales»: CRUD del catálogo local, disponible sin conexión a BDP.
 * Sin estado propio: los datos y las acciones llegan desde BdpExplorador. */
import { Loader2, Pencil, Plus, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import type { BdpMenuLocalConLineas } from '@/api/bdp';

interface MenusLocalesProps {
  menusLocales: BdpMenuLocalConLineas[] | undefined;
  cargando: boolean;
  onNuevo: () => void;
  onEditar: (menu: BdpMenuLocalConLineas) => void;
  onEliminar: (menu: BdpMenuLocalConLineas) => void;
}

/* Precio en euros; si no es numérico se muestra tal cual. */
function formatPrecio(value: string) {
  const n = Number(value);
  if (Number.isNaN(n)) return value;
  return new Intl.NumberFormat('es-ES', { style: 'currency', currency: 'EUR' }).format(n);
}

export function MenusLocales({ menusLocales, cargando, onNuevo, onEditar, onEliminar }: MenusLocalesProps) {
  // [128A-1/F7] Sección de menús/packs locales — funciona sin BDP.
  return (
    <div className="rounded-md border overflow-x-auto">
      <div className="flex items-center justify-between px-4 py-3 border-b">
        <div>
          <h3 className="text-sm font-medium">Menús y packs locales</h3>
          <p className="text-xs text-muted-foreground">
            Agrupaciones del catálogo local. Funcionan sin conexión a BDP.
          </p>
        </div>
        <Button size="sm" onClick={onNuevo}>
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
          {cargando ? (
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
                      onClick={() => onEditar(menu)}
                      aria-label={`Editar ${menu.nombre}`}
                    >
                      <Pencil className="size-4" />
                    </Button>
                    <Button
                      variant="outline"
                      size="icon"
                      className="bg-muted/40 hover:bg-muted"
                      onClick={() => onEliminar(menu)}
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
  );
}
