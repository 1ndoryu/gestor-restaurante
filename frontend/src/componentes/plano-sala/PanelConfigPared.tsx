/* [134A-8] Panel de configuración de paredes — simplificado.\n * Solo largo (ancho canónico) editable. Grosor fijo (10). */

import { useState, type FormEvent } from 'react';
import { Button } from '@/components/ui/button';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Copy } from 'lucide-react';
import type { ParedSala, ActualizarParedRequest } from '../../api/generated/gestionRestauranteAPI.schemas';

const GROSOR = 10;

interface Props {
  pared: ParedSala;
  onGuardar: (id: string, req: ActualizarParedRequest) => void;
  onEliminar: (id: string) => void;
  onDuplicar: (pared: ParedSala) => void;
  onCerrar: () => void;
}

export default function PanelConfigPared({ pared, onGuardar, onEliminar, onDuplicar, onCerrar }: Props) {
  const [largo, setLargo] = useState(pared.ancho);
  const [rotacion, setRotacion] = useState(pared.rotacion);
  const [posX, setPosX] = useState(pared.pos_x);
  const [posY, setPosY] = useState(pared.pos_y);

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    onGuardar(pared.id, { ancho: largo, alto: GROSOR, rotacion, pos_x: posX, pos_y: posY });
  };

  return (
    <div className="rounded-lg border bg-card p-4 flex flex-col gap-3 sticky top-4">
      <h3 className="font-semibold">Configurar pared</h3>
      <form onSubmit={handleSubmit} className="flex flex-col gap-3">
        <div>
          <Label htmlFor="pared-largo">Largo (px)</Label>
          <Input id="pared-largo" type="number" min={20} value={largo} onChange={e => setLargo(+e.target.value)} />
        </div>
        <div className="grid grid-cols-2 gap-2">
          <div>
            <Label htmlFor="pared-x">Pos X</Label>
            <Input id="pared-x" type="number" min={0} value={posX} onChange={e => setPosX(+e.target.value)} />
          </div>
          <div>
            <Label htmlFor="pared-y">Pos Y</Label>
            <Input id="pared-y" type="number" min={0} value={posY} onChange={e => setPosY(+e.target.value)} />
          </div>
        </div>

        <div>
          <Label htmlFor="pared-rotacion">Rotación (°)</Label>
          <Input id="pared-rotacion" type="number" min={0} max={360} value={rotacion} onChange={e => setRotacion(+e.target.value)} />
        </div>
        <div className="flex gap-2 pt-2">
          <Button type="submit" size="sm">Guardar</Button>
          <TooltipButton type="button" size="sm" variant="outline" onClick={() => onDuplicar(pared)} tooltip="Duplicar esta pared">
            <Copy className="size-4" />
          </TooltipButton>
          <Button type="button" size="sm" variant="destructive" onClick={() => onEliminar(pared.id)}>Eliminar</Button>
          <Button type="button" size="sm" variant="ghost" onClick={onCerrar}>Cerrar</Button>
        </div>
      </form>
    </div>
  );
}
