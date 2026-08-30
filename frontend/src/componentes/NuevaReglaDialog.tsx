/* [263A-25] Diálogo para crear una regla de recordatorio + helpers de canal.
 * Extraído de Recordatorios.tsx para reducir el archivo (protocolo limite-lineas).
 * El estado del formulario vive en useNuevaReglaForm (usestate-excesivo). */

import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Label } from '@/components/ui/label';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Plus } from 'lucide-react';
import { useNuevaReglaForm, type DatosCrearRegla } from './useNuevaReglaForm';

const ETIQUETAS_CANAL: Record<string, string> = {
  sms: 'SMS',
  email: 'Email',
  whatsapp: 'WhatsApp',
};

export function badgeCanal(canal: string) {
  switch (canal) {
    case 'email':
      return <Badge variant="default">{ETIQUETAS_CANAL[canal] || canal}</Badge>;
    case 'whatsapp':
      return <Badge className="bg-green-600 hover:bg-green-700 text-white">{ETIQUETAS_CANAL[canal] || canal}</Badge>;
    default:
      return <Badge variant="secondary">{ETIQUETAS_CANAL[canal] || canal}</Badge>;
  }
}

/* [014A-3] Soporta tipo "antes" y "despues" */
export function formatHoras(horas: number | null | undefined, tipo?: string | null) {
  const h = horas ?? 0;
  const sufijo = tipo === 'despues' ? 'después' : 'antes';
  if (h >= 24) {
    const dias = Math.floor(h / 24);
    const rest = h % 24;
    return rest > 0 ? `${dias}d ${rest}h ${sufijo}` : `${dias}d ${sufijo}`;
  }
  return `${h}h ${sufijo}`;
}

/* [014A-3] Soporta tipo "antes" y "despues". [014A-5] WhatsApp removido. */
export default function NuevaReglaDialog({ onCrear }: { onCrear: (data: DatosCrearRegla) => Promise<unknown> }) {
  const {
    open, setOpen,
    nombre, setNombre,
    tipo, setTipo,
    horas, setHoras,
    canal, setCanal,
    mensaje, setMensaje,
    enviando, handleCrear,
  } = useNuevaReglaForm(onCrear);

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button><Plus className="mr-1 size-4" /> Nueva Regla</Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Nueva Regla de Recordatorio</DialogTitle>
          <DialogDescription>
            Define cuándo y cómo enviar recordatorios automáticos
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4 py-2">
          <div className="space-y-2">
            <Label htmlFor="nombre">Nombre de la regla *</Label>
            <Input
              id="nombre"
              value={nombre}
              onChange={e => setNombre(e.target.value)}
              placeholder="Ej: Recordatorio 24h antes"
            />
          </div>
          <div className="grid grid-cols-3 gap-3">
            <div className="space-y-2">
              <Label>Tipo</Label>
              <Select value={tipo} onValueChange={(v) => setTipo(v as 'antes' | 'despues')}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="antes">Antes de reserva</SelectItem>
                  <SelectItem value="despues">Después de reserva</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label>{tipo === 'antes' ? 'Horas antes' : 'Horas después'}</Label>
              <Input
                type="number"
                min={1}
                value={horas}
                onChange={e => setHoras(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label>Canal</Label>
              <Select value={canal} onValueChange={setCanal}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="sms">SMS</SelectItem>
                  <SelectItem value="email">Email</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
          <div className="space-y-2">
            <Label htmlFor="mensaje">Mensaje (opcional)</Label>
            <Textarea
              id="mensaje"
              value={mensaje}
              onChange={e => setMensaje(e.target.value)}
              placeholder="Hola {nombre}, te recordamos tu reserva para el {fecha} a las {hora}..."
              rows={3}
            />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => setOpen(false)}>Cancelar</Button>
          <Button onClick={handleCrear} disabled={!nombre.trim() || enviando}>
            {enviando ? 'Creando...' : 'Crear Regla'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}