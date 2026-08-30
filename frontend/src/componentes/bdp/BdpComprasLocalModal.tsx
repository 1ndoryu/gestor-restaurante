/* [128A-1/F5] Modal de alta/edición de albaranes de compra locales.
 * Solo aplica a `origen='local'`; los importados de BDP no se editan. */

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
import type {
  ActualizarBdpPurchaseNoteRequest,
  BdpPurchaseNote,
  CrearBdpPurchaseNoteRequest,
} from '@/api/bdp';
import { useBdpComprasLocalForm } from '@/hooks/useBdpComprasLocalForm';

interface BdpComprasLocalModalProps {
  open: boolean;
  /** null = crear; con valor = editar ese albarán. */
  note: BdpPurchaseNote | null;
  isSubmitting: boolean;
  onClose: () => void;
  onSubmit: (req: CrearBdpPurchaseNoteRequest | ActualizarBdpPurchaseNoteRequest) => void;
}

export function BdpComprasLocalModal({
  open,
  note,
  isSubmitting,
  onClose,
  onSubmit,
}: BdpComprasLocalModalProps) {
  const isEdit = note !== null;
  const {
    serie,
    setSerie,
    numero,
    setNumero,
    fecha,
    setFecha,
    nombreProveedor,
    setNombreProveedor,
    codigoProveedor,
    setCodigoProveedor,
    total,
    setTotal,
    lineas,
    addLinea,
    updateLinea,
    removeLinea,
    error,
    handleSubmit,
  } = useBdpComprasLocalForm(open, isEdit, note);

  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && onClose()}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{isEdit ? 'Editar albarán local' : 'Nuevo albarán local'}</DialogTitle>
          <DialogDescription>
            {isEdit
              ? `Serie ${note.serie} — Número ${note.numero}. Los albaranes locales se guardan sin conexión a BDP.`
              : 'Crea un albarán de compra en modo independiente. No se envía nada a BDP.'}
          </DialogDescription>
        </DialogHeader>

        <form id="albaran-local-form" onSubmit={(e) => handleSubmit(e, onSubmit)} className="space-y-4">
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-2">
              <Label htmlFor="albaran-serie">Serie</Label>
              <Input
                id="albaran-serie"
                value={serie}
                onChange={(e) => setSerie(e.target.value)}
                placeholder="L"
                disabled={isEdit}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="albaran-numero">Número (opcional)</Label>
              <Input
                id="albaran-numero"
                value={numero}
                onChange={(e) => setNumero(e.target.value)}
                placeholder="Siguiente secuencial"
              />
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="albaran-fecha">Fecha</Label>
            <Input
              id="albaran-fecha"
              type="date"
              value={fecha}
              onChange={(e) => setFecha(e.target.value)}
            />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-2">
              <Label htmlFor="albaran-nombre">Proveedor (nombre)</Label>
              <Input
                id="albaran-nombre"
                value={nombreProveedor}
                onChange={(e) => setNombreProveedor(e.target.value)}
                placeholder="Nombre del proveedor"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="albaran-codigo">Proveedor (código)</Label>
              <Input
                id="albaran-codigo"
                value={codigoProveedor}
                onChange={(e) => setCodigoProveedor(e.target.value)}
                placeholder="Código opcional"
              />
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="albaran-total">Total (si no indicas líneas)</Label>
            <Input
              id="albaran-total"
              inputMode="decimal"
              value={total}
              onChange={(e) => setTotal(e.target.value)}
              placeholder="0.00"
            />
          </div>

          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label>Líneas (IVA por línea)</Label>
              <Button type="button" variant="outline" size="sm" onClick={addLinea}>
                <Plus className="mr-1 size-3.5" />
                Añadir línea
              </Button>
            </div>
            {lineas.length === 0 ? (
              <p className="text-xs text-muted-foreground">
                Sin líneas. El total se toma del campo anterior.
              </p>
            ) : (
              <div className="space-y-2">
                {lineas.map((linea) => (
                  <div key={linea.key} className="grid grid-cols-[1fr_auto_auto_auto_auto] gap-2 items-center">
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
                    <Input
                      className="w-20"
                      inputMode="decimal"
                      value={linea.iva_pct}
                      onChange={(e) => updateLinea(linea.key, 'iva_pct', e.target.value)}
                      placeholder="IVA %"
                      aria-label="Porcentaje de IVA"
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
          <Button type="submit" form="albaran-local-form" disabled={isSubmitting}>
            {isEdit ? 'Guardar cambios' : 'Crear albarán'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
