/* Modal de ajuste manual de stock local: entrada o salida (delta) con motivo obligatorio.
 * El stock BDP (snapshot) no se toca; el formulario se limpia al cambiar de artículo. */
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogClose } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import type { BdpArticleMap } from '@/api/generated/gestionRestauranteAPI.schemas';

/* [128A-1/F3] Modal de ajuste manual de stock local. */
export function AjustarStockDialog({
  articulo,
  open,
  onOpenChange,
  onGuardar,
  pending,
}: {
  articulo: BdpArticleMap | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onGuardar: (delta: string, motivo: string) => void;
  pending: boolean;
}) {
  const [delta, setDelta] = useState('');
  const [motivo, setMotivo] = useState('');
  const [ultimoArticuloKey, setUltimoArticuloKey] = useState<string | null>(null);

  const deltaNum = Number(delta);
  const deltaValido = delta.trim() !== '' && Number.isFinite(deltaNum) && deltaNum !== 0;

  /* [128A-1/F3] Reset del formulario cuando cambia el artículo del modal. */
  const articuloKey = articulo?.id ?? 'none';
  if (ultimoArticuloKey !== articuloKey) {
    setDelta('');
    setMotivo('');
    setUltimoArticuloKey(articuloKey);
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Ajustar stock de {articulo?.articulo_glory_codigo ?? ''}</DialogTitle>
          <DialogDescription>
            Entrada (delta positivo) o salida (delta negativo) de stock local.
            El stock BDP (snapshot) no se modifica.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-2">
          <div className="grid gap-2">
            <Label htmlFor="ajuste-delta">Cantidad (delta)</Label>
            <Input
              id="ajuste-delta"
              type="number"
              step="any"
              placeholder="Ej: 5 o -3"
              value={delta}
              onChange={(e) => setDelta(e.target.value)}
            />
            {!deltaValido && delta.trim() !== '' && (
              <p className="text-xs text-destructive">El delta debe ser un número distinto de cero.</p>
            )}
          </div>
          <div className="grid gap-2">
            <Label htmlFor="ajuste-motivo">Motivo</Label>
            <Textarea
              id="ajuste-motivo"
              placeholder="Ej: entrada de mercancía, merma, conteo..."
              value={motivo}
              onChange={(e) => setMotivo(e.target.value)}
              maxLength={255}
            />
          </div>
        </div>
        <DialogFooter>
          <DialogClose asChild>
            <Button variant="outline">Cancelar</Button>
          </DialogClose>
          <Button
            onClick={() => onGuardar(delta, motivo)}
            disabled={pending || !deltaValido || motivo.trim() === ''}
          >
            Guardar
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
