/* Diálogo de confirmación del conteo: resume lo contado y pide observaciones antes de
 * ajustar el stock. Sin estado propio: todo llega por props desde BdpInventario. */
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Loader2, Save } from 'lucide-react';

interface DialogoConfirmarConteoProps {
  abierto: boolean;
  onAbiertoChange: (abierto: boolean) => void;
  numContados: number;
  conDiferencia: number;
  observaciones: string;
  onObservacionesChange: (valor: string) => void;
  modoEfectivoBdp: boolean;
  guardando: boolean;
  onConfirmar: () => void;
}

export function DialogoConfirmarConteo({
  abierto,
  onAbiertoChange,
  numContados,
  conDiferencia,
  observaciones,
  onObservacionesChange,
  modoEfectivoBdp,
  guardando,
  onConfirmar,
}: DialogoConfirmarConteoProps) {
  return (
    <Dialog open={abierto} onOpenChange={onAbiertoChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Guardar conteo</DialogTitle>
          <DialogDescription>
            Se ajustará el stock local con las diferencias contadas. Esta acción queda registrada con motivo «conteo».
          </DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-3 text-sm">
          <p>
            <span className="font-medium">{numContados}</span> artículo(s) contados ·{' '}
            <span className="font-medium">{conDiferencia}</span> con diferencia
          </p>
          <div className="flex flex-col gap-1">
            <Label htmlFor="inventario-observaciones" className="text-xs">Observaciones (opcional)</Label>
            <Input
              id="inventario-observaciones"
              value={observaciones}
              onChange={(e) => onObservacionesChange(e.target.value)}
              placeholder="Ej: recuento semanal de almacén"
              maxLength={500}
            />
          </div>
          <p className="text-xs text-muted-foreground">
            {modoEfectivoBdp
              ? 'El conteo se guarda, ajusta el stock local y encola el envío al terminal para los artículos con código BDP.'
              : 'Modo independiente: el conteo se guarda y ajusta el stock local; no se envía a BDP.'}
          </p>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onAbiertoChange(false)} disabled={guardando}>
            Cancelar
          </Button>
          <Button onClick={onConfirmar} disabled={guardando || numContados === 0}>
            {guardando ? <Loader2 className="size-4 animate-spin mr-1" /> : <Save className="size-4 mr-1" />}
            Confirmar y guardar
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
