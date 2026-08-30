/* [263A-16] Panel lateral para editar propiedades de una mesa seleccionada — shadcn */

import { useState } from 'react';
import type { ActualizarMesaRequest, Mesa } from '../../api/generated';
import { Button } from '@/components/ui/button';
import { Bell, Copy } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';

interface PanelConfigMesaProps {
  mesa: Mesa;
  onGuardar: (id: string, data: ActualizarMesaRequest) => void;
  onEliminar: (id: string) => void;
  onDuplicar: (mesa: Mesa) => void;
  onCerrar: () => void;
  /* [198A-1/D10] Botón "Llamar camarero" — solo se pasa con BDP activo. */
  onLlamarCamarero?: () => void;
  llamarCamareroPending?: boolean;
}

function PanelConfigMesa({ mesa, onGuardar, onEliminar, onDuplicar, onCerrar, onLlamarCamarero, llamarCamareroPending = false }: PanelConfigMesaProps) {
  const MIN_LADO_MESA = 30;
  const MAX_LADO_MESA = 400;
  const RATIO_RECTANGULAR = 1.8;

  const [form, setForm] = useState({
    numero: mesa.numero,
    minP: mesa.min_personas,
    maxP: mesa.max_personas,
    forma: mesa.forma,
    ancho: mesa.ancho,
    alto: mesa.alto,
    activa: mesa.activa,
  });

  const clampDimension = (value: number) =>
    Math.min(MAX_LADO_MESA, Math.max(MIN_LADO_MESA, value || MIN_LADO_MESA));

  const normalizeForma = (forma: string, ancho: number, alto: number) => {
    const anchoSeguro = clampDimension(ancho);
    const altoSeguro = clampDimension(alto);

    if (forma === 'rectangular') {
      const altoBase = Math.min(anchoSeguro, altoSeguro);
      return {
        ancho: clampDimension(Math.round(altoBase * RATIO_RECTANGULAR)),
        alto: altoBase,
      };
    }

    if (forma === 'cuadrada' || forma === 'redonda') {
      const lado = Math.min(anchoSeguro, altoSeguro);
      return { ancho: lado, alto: lado };
    }

    return { ancho: anchoSeguro, alto: altoSeguro };
  };

  /* [313A-2] El usuario debe poder bajar px manualmente.
   * Cambiar de forma solo normaliza una vez; editar ancho/alto no puede
   * inflar la mesa por acumulación. */
  const set = (campo: string, valor: number | string | boolean) =>
    setForm((prev) => {
      const next = { ...prev, [campo]: valor };

      if (campo === 'forma') {
        const dimensiones = normalizeForma(String(valor), next.ancho, next.alto);
        next.ancho = dimensiones.ancho;
        next.alto = dimensiones.alto;
      } else if ((campo === 'ancho' || campo === 'alto') && (next.forma === 'cuadrada' || next.forma === 'redonda')) {
        const lado = clampDimension(Number(valor));
        next.ancho = lado;
        next.alto = lado;
      } else if (campo === 'ancho' || campo === 'alto') {
        next[campo as 'ancho' | 'alto'] = clampDimension(Number(valor));
      }
      return next;
    });

  const guardar = () => {
    onGuardar(mesa.id, {
      numero: form.numero,
      min_personas: form.minP,
      max_personas: form.maxP,
      forma: form.forma,
      ancho: form.ancho,
      alto: form.alto,
      activa: form.activa,
    });
  };

  return (
    <div className="rounded-lg border bg-card p-4 flex flex-col gap-3 sticky top-4">
      <h3 className="font-semibold">Mesa {mesa.numero}</h3>
      <div className="flex flex-col gap-2">
        <Label htmlFor="cfg-numero">Número</Label>
        <Input id="cfg-numero" type="number" value={form.numero} onChange={e => set('numero', Number(e.target.value))} />
      </div>
      <div className="grid grid-cols-2 gap-3">
        <div className="flex flex-col gap-2">
          <Label htmlFor="cfg-minP">Mín personas</Label>
          <Input id="cfg-minP" type="number" value={form.minP} onChange={e => set('minP', Number(e.target.value))} />
        </div>
        <div className="flex flex-col gap-2">
          <Label htmlFor="cfg-maxP">Máx personas</Label>
          <Input id="cfg-maxP" type="number" value={form.maxP} onChange={e => set('maxP', Number(e.target.value))} />
        </div>
      </div>
      <div className="flex flex-col gap-2">
        <Label htmlFor="cfg-forma">Forma</Label>
        <Select value={form.forma} onValueChange={v => set('forma', v)}>
          <SelectTrigger id="cfg-forma"><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="cuadrada">Cuadrada</SelectItem>
            <SelectItem value="redonda">Redonda</SelectItem>
            <SelectItem value="rectangular">Rectangular</SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div className="grid grid-cols-2 gap-3">
        <div className="flex flex-col gap-2">
          <Label htmlFor="cfg-ancho">Ancho (px)</Label>
          <Input id="cfg-ancho" type="number" value={form.ancho} onChange={e => set('ancho', Number(e.target.value))} />
        </div>
        <div className="flex flex-col gap-2">
          <Label htmlFor="cfg-alto">Alto (px)</Label>
          <Input id="cfg-alto" type="number" value={form.alto} onChange={e => set('alto', Number(e.target.value))} />
        </div>
      </div>
      <div className="flex items-center gap-2">
        <Switch id="cfg-activa" checked={form.activa} onCheckedChange={checked => set('activa', checked)} />
        <Label htmlFor="cfg-activa">Activa</Label>
      </div>
      {/* [198A-1/D10] CallWaiter: push directo a BDP, oculto en standalone. */}
      {onLlamarCamarero && (
        <Button size="sm" variant="secondary" onClick={onLlamarCamarero} disabled={llamarCamareroPending}>
          <Bell className="size-4 mr-1" /> Llamar camarero
        </Button>
      )}
      <div className="flex gap-2 pt-2">
        <Button size="sm" onClick={guardar}>Guardar</Button>
        <Button size="sm" variant="outline" onClick={() => onDuplicar(mesa)} title="Duplicar mesa">
          <Copy className="size-4" />
        </Button>
        <Button size="sm" variant="destructive" onClick={() => onEliminar(mesa.id)}>Eliminar</Button>
        <Button size="sm" variant="ghost" onClick={onCerrar}>Cerrar</Button>
      </div>
    </div>
  );
}

export default PanelConfigMesa;
