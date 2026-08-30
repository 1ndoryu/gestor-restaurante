/* [128A-1/F7] Modal de alta/edición de menús/packs locales (D2, §4.10).
 * Solo aplica a la agrupación local; no se envía nada a BDP.
 * Las líneas referencian artículos del catálogo local (`useBdpArticleMaps`). */

import { useState } from 'react';
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
import type {
  ActualizarBdpMenuLocalRequest,
  BdpMenuLocalConLineas,
  BdpMenuLocalLineaRequest,
  BdpMenuLocalTipo,
  CrearBdpMenuLocalRequest,
} from '@/api/bdp';

interface LineaForm {
  key: number;
  articulo_codigo: string;
  descripcion: string;
  cantidad: string;
  precio_unitario: string;
}

interface BdpMenuLocalModalProps {
  open: boolean;
  /** null = crear; con valor = editar ese menú/pack. */
  menu: BdpMenuLocalConLineas | null;
  isSubmitting: boolean;
  onClose: () => void;
  onSubmit: (req: CrearBdpMenuLocalRequest | ActualizarBdpMenuLocalRequest) => void;
}

function lineasDesdeMenu(menu: BdpMenuLocalConLineas): LineaForm[] {
  return menu.lineas.map((linea, index) => ({
    key: index,
    articulo_codigo: linea.articulo_codigo ?? '',
    descripcion: linea.descripcion,
    cantidad: String(linea.cantidad),
    precio_unitario: String(linea.precio_unitario),
  }));
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

  const [tipo, setTipo] = useState<BdpMenuLocalTipo>('menu');
  const [nombre, setNombre] = useState('');
  const [descripcion, setDescripcion] = useState('');
  const [precio, setPrecio] = useState('');
  const [activo, setActivo] = useState(true);
  const [lineas, setLineas] = useState<LineaForm[]>([]);
  const [nextLineaKey, setNextLineaKey] = useState(1);
  const [error, setError] = useState('');

  /* Sincroniza el formulario cada vez que se abre el modal. */
  const [lastOpenedFor, setLastOpenedFor] = useState<string | null>(null);
  const openedKey = isEdit ? menu.id : 'nuevo';
  if (open && lastOpenedFor !== openedKey) {
    setLastOpenedFor(openedKey);
    setError('');
    if (menu) {
      setTipo(menu.tipo);
      setNombre(menu.nombre);
      setDescripcion(menu.descripcion ?? '');
      setPrecio(menu.precio);
      setActivo(menu.activo);
      const iniciales = lineasDesdeMenu(menu);
      setLineas(iniciales);
      setNextLineaKey(iniciales.length + 1);
    } else {
      setTipo('menu');
      setNombre('');
      setDescripcion('');
      setPrecio('');
      setActivo(true);
      setLineas([]);
      setNextLineaKey(1);
    }
  }

  function addLinea() {
    setLineas((prev) => [
      ...prev,
      { key: nextLineaKey, articulo_codigo: '', descripcion: '', cantidad: '1', precio_unitario: '' },
    ]);
    setNextLineaKey((k) => k + 1);
  }

  function updateLinea(key: number, campo: keyof LineaForm, valor: string) {
    setLineas((prev) => prev.map((l) => (l.key === key ? { ...l, [campo]: valor } : l)));
  }

  function seleccionarArticulo(key: number, articuloCodigo: string) {
    const articulo = catalog?.find((a) => a.articulo_glory_codigo === articuloCodigo);
    setLineas((prev) =>
      prev.map((l) =>
        l.key === key
          ? {
              ...l,
              articulo_codigo: articuloCodigo,
              descripcion: articulo?.articulo_bdp_nombre ?? l.descripcion,
            }
          : l,
      ),
    );
  }

  function removeLinea(key: number) {
    setLineas((prev) => prev.filter((l) => l.key !== key));
  }

  /* Normaliza decimales con coma (formato español) a punto antes de enviar;
   * el backend espera `Decimal` con punto (serde). [208A-2/F7] */
  function normalizarDecimal(valor: string): string {
    const limpio = valor.trim().replace(/\s/g, '');
    return limpio.replace(',', '.');
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const nombreTrim = nombre.trim();
    if (!nombreTrim) {
      setError('Indica el nombre del menú/pack');
      return;
    }
    const lineasValidas: BdpMenuLocalLineaRequest[] = lineas
      .filter((l) => l.descripcion.trim() !== '')
      .map((l) => ({
        articulo_codigo: l.articulo_codigo.trim() || undefined,
        descripcion: l.descripcion.trim(),
        cantidad: l.cantidad !== '' ? normalizarDecimal(l.cantidad) : undefined,
        precio_unitario: l.precio_unitario !== '' ? normalizarDecimal(l.precio_unitario) : undefined,
      }));
    if (lineasValidas.length === 0) {
      setError('Añade al menos una línea con descripción');
      return;
    }

    const base = {
      tipo,
      nombre: nombreTrim,
      descripcion: descripcion.trim() || undefined,
      precio: precio.trim() ? normalizarDecimal(precio) : undefined,
      activo,
      lineas: lineasValidas,
    };
    if (isEdit) {
      onSubmit(base as ActualizarBdpMenuLocalRequest);
    } else {
      onSubmit(base as CrearBdpMenuLocalRequest);
    }
  }

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

        <form id="menu-local-form" onSubmit={handleSubmit} className="space-y-4">
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
                          : seleccionarArticulo(linea.key, v)
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
