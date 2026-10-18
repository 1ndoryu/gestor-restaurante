/* Diálogo de importación de salones y mesas desde BDP. Primero consulta el impacto;
 * al aplicar solo crea lo que falte y no elimina nada. */
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { PreviewSyncMesasBdp } from './useSincronizarMesasBdp';

interface DialogoSincronizarMesasBdpProps {
  abierto: boolean;
  onAbiertoChange: (abierto: boolean) => void;
  preview: PreviewSyncMesasBdp | null;
  confirmacion: string;
  onConfirmacionChange: (valor: string) => void;
  sincronizando: boolean;
  onPrevisualizar: () => void;
  onAplicar: () => void;
}

export function DialogoSincronizarMesasBdp({
  abierto,
  onAbiertoChange,
  preview,
  confirmacion,
  onConfirmacionChange,
  sincronizando,
  onPrevisualizar,
  onAplicar,
}: DialogoSincronizarMesasBdpProps) {
  return (
    <Dialog open={abierto} onOpenChange={onAbiertoChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader><DialogTitle>Importar salones y mesas desde BDP</DialogTitle><DialogDescription>Primero consulta el impacto. BDP solo se lee; al aplicar se crean únicamente zonas/mesas faltantes en la Aplicación Web y no se elimina nada.</DialogDescription></DialogHeader>
        {preview && <div className="rounded-md border p-3 text-sm">Se crearían {preview.zonas_creadas} zonas y {preview.mesas_creadas} mesas a partir de {preview.salones_bdp} salones BDP.</div>}
        {preview && <div><Label htmlFor="confirmar-sync-mesas">Escribe IMPORTAR MESAS BDP</Label><Input id="confirmar-sync-mesas" value={confirmacion} onChange={(e) => onConfirmacionChange(e.target.value)} /></div>}
        <DialogFooter>
          <Button variant="outline" onClick={() => onAbiertoChange(false)}>Cancelar</Button>
          <Button variant="secondary" disabled={sincronizando} onClick={onPrevisualizar}>Previsualizar sin cambios</Button>
          {preview && <Button disabled={sincronizando || confirmacion !== 'IMPORTAR MESAS BDP'} onClick={onAplicar}>Aplicar en la Aplicación Web</Button>}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
