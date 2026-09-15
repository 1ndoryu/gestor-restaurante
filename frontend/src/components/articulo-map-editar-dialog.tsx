/* [159A-1] Diálogo de edición de artículo del Catálogo (decisión visual 2026-09-15:
 * toda alta/modificación va en modal, como "Nueva Venta", nunca inline).
 * Sustituye la fila de edición expandida de BdpArticleMapTable: edita los
 * campos locales (código BDP, descripción, precio, IVA) vía PATCH parcial. */

import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogClose,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

export interface ArticuloMapEdicion {
  id: string;
  articulo_bdp_codigo: string;
  descripcion: string;
  precio_tarifa1: string;
  iva_pct: string;
}

const edicionVacia: ArticuloMapEdicion = {
  id: '',
  articulo_bdp_codigo: '',
  descripcion: '',
  precio_tarifa1: '',
  iva_pct: '',
};

interface ArticuloMapEditarDialogProps {
  edicion: ArticuloMapEdicion | null;
  isPending: boolean;
  onOpenChange: (open: boolean) => void;
  onGuardar: (edicion: ArticuloMapEdicion) => void;
}

function ArticuloMapEditarDialog({ edicion, isPending, onOpenChange, onGuardar }: ArticuloMapEditarDialogProps) {
  const [form, setForm] = useState<ArticuloMapEdicion>(edicionVacia);

  useEffect(() => {
    setForm(edicion ?? edicionVacia);
  }, [edicion]);

  return (
    <Dialog open={edicion !== null} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Editar artículo</DialogTitle>
          <DialogDescription>
            Modifica los datos locales del artículo. No cambia nada en el catálogo BDP.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-2">
          <div className="grid gap-2">
            <Label htmlFor="editar-articulo-bdp">Código BDP</Label>
            <Input
              id="editar-articulo-bdp"
              className="font-mono text-xs"
              value={form.articulo_bdp_codigo}
              onChange={(e) => setForm((p) => ({ ...p, articulo_bdp_codigo: e.target.value }))}
              placeholder="Vacío = artículo local"
            />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="editar-articulo-descripcion">Descripción</Label>
            <Input
              id="editar-articulo-descripcion"
              className="text-xs"
              value={form.descripcion}
              onChange={(e) => setForm((p) => ({ ...p, descripcion: e.target.value }))}
            />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div className="grid gap-2">
              <Label htmlFor="editar-articulo-precio">Precio (€)</Label>
              <Input
                id="editar-articulo-precio"
                className="text-xs"
                type="number"
                min="0"
                step="0.01"
                value={form.precio_tarifa1}
                onChange={(e) => setForm((p) => ({ ...p, precio_tarifa1: e.target.value }))}
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="editar-articulo-iva">IVA (%)</Label>
              <Input
                id="editar-articulo-iva"
                className="text-xs"
                type="number"
                min="0"
                step="0.01"
                value={form.iva_pct}
                onChange={(e) => setForm((p) => ({ ...p, iva_pct: e.target.value }))}
              />
            </div>
          </div>
        </div>
        <DialogFooter>
          <DialogClose asChild>
            <Button variant="outline">Cancelar</Button>
          </DialogClose>
          <Button onClick={() => onGuardar(form)} disabled={isPending}>
            Guardar cambios
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export default ArticuloMapEditarDialog;
