/* [247A-12] Modal de conciliación de albaranes BDP (Fase 3).
 * Permite vincular el albarán con un gasto existente o crear uno nuevo. */

import { useState } from 'react';
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
import type { BdpPurchaseNote, BdpPurchaseNoteReconcileRequest } from '@/api/bdp';

interface BdpComprasReconcileModalProps {
  open: boolean;
  note: BdpPurchaseNote | null;
  onClose: () => void;
  onSubmit: (req: BdpPurchaseNoteReconcileRequest) => void;
}

export function BdpComprasReconcileModal({
  open,
  note,
  onClose,
  onSubmit,
}: BdpComprasReconcileModalProps) {
  const [mode, setMode] = useState<'new' | 'existing'>('new');
  const [gastoId, setGastoId] = useState('');
  const [categoriaId, setCategoriaId] = useState('');

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const req: BdpPurchaseNoteReconcileRequest =
      mode === 'existing'
        ? { gasto_existente_id: gastoId || undefined }
        : { categoria_id: categoriaId || undefined };
    onSubmit(req);
  }

  if (!note) return null;

  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && onClose()}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Conciliar albarán</DialogTitle>
          <DialogDescription>
            Serie {note.serie} — Número {note.numero}
            <br />
            Proveedor: {note.nombre_proveedor || note.codigo_proveedor || '—'}
            <br />
            Total: {note.total || '—'}
          </DialogDescription>
        </DialogHeader>

        <form id="reconcile-form" onSubmit={handleSubmit} className="space-y-4">
          <div className="flex gap-4">
            <Button
              type="button"
              variant={mode === 'new' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setMode('new')}
            >
              Crear gasto nuevo
            </Button>
            <Button
              type="button"
              variant={mode === 'existing' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setMode('existing')}
            >
              Vincular gasto existente
            </Button>
          </div>

          {mode === 'new' ? (
            <div className="space-y-2">
              <Label htmlFor="categoria-id">ID de categoría (opcional)</Label>
              <Input
                id="categoria-id"
                value={categoriaId}
                onChange={(e) => setCategoriaId(e.target.value)}
                placeholder="uuid de la categoría de gasto"
              />
            </div>
          ) : (
            <div className="space-y-2">
              <Label htmlFor="gasto-id">ID del gasto existente</Label>
              <Input
                id="gasto-id"
                value={gastoId}
                onChange={(e) => setGastoId(e.target.value)}
                placeholder="uuid del gasto"
                required
              />
            </div>
          )}
        </form>

        <DialogFooter>
          <Button type="button" variant="outline" onClick={onClose}>
            Cancelar
          </Button>
          <Button type="submit" form="reconcile-form">
            Conciliar
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
