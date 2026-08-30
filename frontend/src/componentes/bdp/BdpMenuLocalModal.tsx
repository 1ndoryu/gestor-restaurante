/* [128A-1/F7] Modal de alta/edición de menús/packs locales (D2, §4.10).
 * Solo aplica a la agrupación local; no se envía nada a BDP.
 * Las líneas referencian artículos del catálogo local (`useBdpArticleMaps`). */

import { Plus, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useBdpArticleMaps } from '@/api/bdp';
import { useBdpMenuLocalForm } from '@/hooks/useBdpMenuLocalForm';
import type {
  ActualizarBdpMenuLocalRequest,
  BdpMenuLocalConLineas,
  BdpMenuLocalTipo,
  CrearBdpMenuLocalRequest,
} from '@/api/bdp';

interface BdpMenuLocalModalProps {
  open: boolean;
  /** null = crear; con valor = editar ese menú/pack. */
  menu: BdpMenuLocalConLineas | null;
  isSubmitting: boolean;
  onClose: () => void;
  onSubmit: (req: CrearBdpMenuLocalRequest | ActualizarBdpMenuLocalRequest) => void;
}

export function BdpMenuLocalModal({
  open,
  menu,
  isSubmitting,
  onClose,
  onSubmit,
}: BdpMenuLocalModalProps) {
  const isEdit = menu !== null;
  const { data: catalog } = useBdpArticleMaps();

  const {
    tipo,
    setTipo,
    nombre,
    setNombre,
    descripcion,
    setDescripcion,
    precio,
    setPrecio,
    activo,
    setActivo,
    lineas,
    addLinea,
    updateLinea,
    seleccionarArticulo,
    removeLinea,
    error,
    handleSubmit,
  } = useBdpMenuLocalForm(open, isEdit, menu);

  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && onClose()}>
      <DialogContent className="sm:max-w-2xl max-h-[85vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{isEdit ? 'Editar menú/pack local' : 'Nuevo menú/pack local'}</DialogTitle>
          <DialogDescription>
            {isEdit
              ? `Editando «${menu.nombre}». Los cambios se guardan localmente, sin conexión a BDP.`
              : 'Agrupa artículos del catálogo local. No se envía nada a BDP.'}
          </DialogDescription>
        </DialogHeader>

        <form id="menu-local-form" onSubmit={(e) => handleSubmit(e, onSubmit)} className="space-y-4">
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-2">
              <Label htmlFor="menu-tipo">Tipo</Label>
              <Select value={tipo} onValueChange={(v) => setTipo(v as BdpMenuLocalTipo)}>
                <SelectTrigger id="menu-tipo" className="w-full">
                  <SelectValue placeholder="Tipo" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="menu">Menú</SelectItem>
                  <SelectItem value="pack">Pack</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="menu-nombre">Nombre *</Label>
              <Input
                id="menu-nombre"
                value={nombre}
                onChange={(e) => setNombre(e.target.value)}
                placeholder="Ej. Menú del día"
                maxLength={200}
              />
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="menu-descripcion">Descripción</Label>
            <Input
              id="menu-descripcion"
              value={descripcion}
              onChange={(e) => setDescripcion(e.target.value)}
              placeholder="Opcional"
            />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-2">
              <Label htmlFor="menu-precio">Precio (si no indicas, suma de líneas)</Label>
              <Input
                id="menu-precio"
                inputMode="decimal"
                value={precio}
                onChange={(e) => setPrecio(e.target.value)}
                placeholder="0.00"
              />
            </div>
            <div className="flex items-end pb-1">
              <label className="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={activo}
                  onChange={(e) => setActivo(e.target.checked)}
                  className="size-4"
                />
                Activo
              </label>
            </div>
          </div>

          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label>Artículos (líneas)</Label>
              <Button type="button" variant="outline" size="sm" onClick={addLinea}>
                <Plus className="mr-1 size-3.5" />
                Añadir artículo
              </Button>
            </div>
            {lineas.length === 0 ? (
              <p className="text-xs text-muted-foreground">
                Sin artículos. Añade al menos una línea con descripción.
              </p>
            ) : (
              <div className="space-y-2">
                {lineas.map((linea) => (
                  <div key={linea.key} className="grid grid-cols-[1fr_1fr_auto_auto_auto] gap-2 items-center">
                    <Select
                      value={linea.articulo_codigo || 'sin-articulo'}
                      onValueChange={(v) =>
                        v === 'sin-articulo'
                          ? updateLinea(linea.key, 'articulo_codigo', '')
                          : seleccionarArticulo(linea.key, v, catalog)
                      }
                    >
                      <SelectTrigger aria-label="Artículo del catálogo">
                        <SelectValue placeholder="Artículo del catálogo" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="sin-articulo">— Sin código —</SelectItem>
                        {(catalog ?? []).map((articulo) => (
                          <SelectItem
                            key={articulo.articulo_glory_codigo}
                            value={articulo.articulo_glory_codigo}
                          >
                            {articulo.articulo_bdp_nombre} ({articulo.articulo_glory_codigo})
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <Input
                      value={linea.descripcion}
                      onChange={(e) => updateLinea(linea.key, 'descripcion', e.target.value)}
                      placeholder="Descripción"
                      aria-label="Descripción de la línea"
                    />
                    <Input
                      className="w-20"
                      inputMode="decimal"
                      value={linea.cantidad}
                      onChange={(e) => updateLinea(linea.key, 'cantidad', e.target.value)}
                      placeholder="Cant."
                      aria-label="Cantidad"
                    />
                    <Input
                      className="w-24"
                      inputMode="decimal"
                      value={linea.precio_unitario}
                      onChange={(e) => updateLinea(linea.key, 'precio_unitario', e.target.value)}
                      placeholder="Precio"
                      aria-label="Precio unitario"
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => removeLinea(linea.key)}
                      aria-label="Eliminar línea"
                    >
                      <Trash2 className="size-4 text-destructive" />
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </div>

          {error && <p className="text-sm text-destructive">{error}</p>}
        </form>

        <DialogFooter>
          <Button type="button" variant="outline" onClick={onClose}>
            Cancelar
          </Button>
          <Button type="submit" form="menu-local-form" disabled={isSubmitting}>
            {isEdit ? 'Guardar cambios' : 'Crear menú/pack'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
