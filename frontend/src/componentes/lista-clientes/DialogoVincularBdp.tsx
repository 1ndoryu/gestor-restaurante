/* Diálogo para vincular un cliente local con un código BDP reservado. El servidor
 * verifica que el código no pertenezca a otra identidad (siempre Overwrite=false). */

import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter, DialogDescription } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { Cliente } from '@/api/generated/gestionRestauranteAPI.schemas';

interface DialogoVincularBdpProps {
  cliente: Cliente | null;
  onCerrar: () => void;
  codigo: string;
  onCodigoChange: (valor: string) => void;
  confirmacion: string;
  onConfirmacionChange: (valor: string) => void;
  sincronizando: boolean;
  onVincular: () => void;
}

export function DialogoVincularBdp({
  cliente,
  onCerrar,
  codigo,
  onCodigoChange,
  confirmacion,
  onConfirmacionChange,
  sincronizando,
  onVincular,
}: DialogoVincularBdpProps) {
  return (
    <Dialog open={!!cliente} onOpenChange={(open) => { if (!open) onCerrar(); }}>
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
            <Input id="codigo-bdp-cliente" type="number" min={1} value={codigo} onChange={(e) => onCodigoChange(e.target.value)} />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="confirmar-bdp-cliente">Escribe CREAR CLIENTE {cliente?.nombre} {cliente?.apellidos} {codigo || '<código>'}</Label>
            <Input id="confirmar-bdp-cliente" value={confirmacion} onChange={(e) => onConfirmacionChange(e.target.value)} />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onCerrar()}>Cancelar</Button>
          <Button disabled={!codigo || confirmacion !== `CREAR CLIENTE ${cliente?.nombre} ${cliente?.apellidos} ${codigo}` || sincronizando} onClick={onVincular}>
            {sincronizando ? 'Verificando…' : 'Verificar y vincular'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
