/* Diálogo de fusión de clientes: el usuario elige cuál de los 2 sobrevive y confirma.
 * Sin estado propio: destino y mutación llegan por props desde ListaClientes. */

import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter, DialogDescription } from '@/components/ui/dialog';
import type { Cliente } from '@/api/generated/gestionRestauranteAPI.schemas';

interface DialogoFusionarClientesProps {
  abierto: boolean;
  onCerrar: () => void;
  seleccionados: Cliente[];
  destinoId: string | null;
  fusionando: boolean;
  onElegirDestino: (id: string) => void;
  onConfirmar: () => void;
}

export function DialogoFusionarClientes({
  abierto,
  onCerrar,
  seleccionados,
  destinoId,
  fusionando,
  onElegirDestino,
  onConfirmar,
}: DialogoFusionarClientesProps) {
  return (
    <Dialog open={abierto} onOpenChange={(open) => { if (!open) { onCerrar(); } }}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Fusionar clientes</DialogTitle>
          <DialogDescription>
            Selecciona cuál de los dos clientes debe sobrevivir. El otro será absorbido: sus reservas, etiquetas y campañas se migrarán, y sus campos vacíos se completarán.
          </DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-3">
          {seleccionados.map((c) => (
            <button
              key={c.id}
              type="button"
              onClick={() => onElegirDestino(c.id)}
              className={`flex items-center gap-3 rounded-md border p-3 text-left transition-colors ${
                destinoId === c.id ? 'border-primary bg-primary/10' : 'border-border hover:bg-muted/50'
              }`}
            >
              <div className="flex-1">
                <p className="font-medium">{c.nombre} {c.apellidos}</p>
                <p className="text-sm text-muted-foreground">
                  {[c.telefono, c.email, c.empresa].filter(Boolean).join(' · ') || 'Sin datos adicionales'}
                </p>
              </div>
              {destinoId === c.id && (
                <span className="text-xs font-semibold text-primary">SOBREVIVE</span>
              )}
              {destinoId && destinoId !== c.id && (
                <span className="text-xs text-destructive">SE ELIMINA</span>
              )}
            </button>
          ))}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onCerrar()}>Cancelar</Button>
          <Button
            disabled={!destinoId || fusionando}
            onClick={onConfirmar}
          >
            {fusionando ? 'Fusionando...' : 'Confirmar fusión'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
