/* Diálogo de importación de clientes desde BDP. Primero previsualiza; al aplicar
 * modifica solo la Aplicación Web, nunca escribe clientes en BDP. */

import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter, DialogDescription } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { PreviewImportar } from '@/hooks/useIntegracionBdp';

interface DialogoImportarClientesBdpProps {
  abierto: boolean;
  onAbiertoChange: (abierto: boolean) => void;
  preview: PreviewImportar | null;
  confirmacion: string;
  onConfirmacionChange: (valor: string) => void;
  importando: boolean;
  onPrevisualizar: () => void;
  onAplicar: () => void;
}

export function DialogoImportarClientesBdp({
  abierto,
  onAbiertoChange,
  preview,
  confirmacion,
  onConfirmacionChange,
  importando,
  onPrevisualizar,
  onAplicar,
}: DialogoImportarClientesBdpProps) {
  return (
    <Dialog open={abierto} onOpenChange={onAbiertoChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader><DialogTitle>Importar clientes desde BDP</DialogTitle><DialogDescription>Primero previsualiza. Esta operación solo lee BDP y, al aplicar, modifica la Aplicación Web; nunca escribe clientes en BDP.</DialogDescription></DialogHeader>
        {preview && (
          <div className="grid grid-cols-2 gap-2 rounded-md border p-3 text-sm">
            <span>Nuevos: {preview.imported}</span><span>Vínculos: {preview.updated}</span>
            <span>Sin cambios: {preview.unchanged}</span><span>Conflictos: {preview.conflicts}</span>
            <span>Inválidos: {preview.errors}</span><span>Total: {preview.total}</span>
          </div>
        )}
        {preview && (preview.conflicts > 0 || preview.errors > 0) && <p className="text-sm text-destructive">Hay registros no aplicables. Se omitirán; no se sobrescribirá ningún vínculo existente.</p>}
        {preview && <div><Label htmlFor="confirmar-importar-bdp">Escribe IMPORTAR CLIENTES BDP</Label><Input id="confirmar-importar-bdp" value={confirmacion} onChange={(e) => onConfirmacionChange(e.target.value)} /></div>}
        <DialogFooter>
          <Button variant="outline" onClick={() => onAbiertoChange(false)}>Cancelar</Button>
          <Button variant="secondary" disabled={importando} onClick={() => onPrevisualizar()}>{importando ? 'Consultando…' : 'Previsualizar sin cambios'}</Button>
          {preview && <Button disabled={importando || confirmacion !== 'IMPORTAR CLIENTES BDP'} onClick={() => onAplicar()}>Aplicar en la Aplicación Web</Button>}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
