/* [263A-25] Gestión de recordatorios automáticos de reservas.
 * Permite configurar reglas (horas antes, canal, mensaje) y ver historial.
 * Las reglas se pueden activar/desactivar con un switch. */

import { useState } from 'react';
import { useRecordatorios } from '../hooks/useRecordatorios';
import TablaHistorial from './TablaHistorial';
import { Button } from '@/components/ui/button';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Plus, Trash2, ChevronLeft, ChevronRight } from 'lucide-react';
import type { ReglaRecordatorio } from '../api/generated/gestionRestauranteAPI.schemas';

const ETIQUETAS_CANAL: Record<string, string> = {
  sms: 'SMS',
  email: 'Email',
  whatsapp: 'WhatsApp',
};

function badgeCanal(canal: string) {
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
function formatHoras(horas: number | null | undefined, tipo?: string | null) {
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
function NuevaReglaDialog({ onCrear }: { onCrear: (data: { data: { nombre: string; horas_antes?: number; horas_despues?: number; tipo?: string; canal: string; mensaje_plantilla?: string } }) => Promise<unknown> }) {
  const [open, setOpen] = useState(false);
  const [nombre, setNombre] = useState('');
  const [tipo, setTipo] = useState<'antes' | 'despues'>('antes');
  const [horas, setHoras] = useState('24');
  const [canal, setCanal] = useState('sms');
  const [mensaje, setMensaje] = useState('');
  const [enviando, setEnviando] = useState(false);

  const handleCrear = async () => {
    if (!nombre.trim() || enviando) return;
    setEnviando(true);
    const h = parseInt(horas, 10) || 24;
    try {
      await onCrear({
        data: {
          nombre: nombre.trim(),
          tipo,
          ...(tipo === 'antes' ? { horas_antes: h } : { horas_despues: h }),
          canal,
          mensaje_plantilla: mensaje.trim() || undefined,
        },
      });
      setOpen(false);
      setNombre('');
      setTipo('antes');
      setHoras('24');
      setCanal('sms');
      setMensaje('');
    } finally {
      setEnviando(false);
    }
  };

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

function TablaReglas() {
  const {
    reglas,
    total,
    page,
    totalPages,
    isLoading,
    setPage,
    crearRegla,
    eliminarRegla,
    toggleActiva,
  } = useRecordatorios();

  const handleEliminar = async (id: string) => {
    await eliminarRegla({ id });
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <span className="text-sm text-muted-foreground">{total} regla(s)</span>
        <NuevaReglaDialog onCrear={crearRegla} />
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Reglas de Recordatorio</CardTitle>
          <CardDescription>
            Cada regla define cuándo enviar un recordatorio automático antes de la reserva
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <p className="text-muted-foreground py-8 text-center">Cargando...</p>
          ) : reglas.length === 0 ? (
            <p className="text-muted-foreground py-8 text-center">
              No hay reglas configuradas. Crea tu primera regla de recordatorio.
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Nombre</TableHead>
                  <TableHead>Tiempo</TableHead>
                  <TableHead>Canal</TableHead>
                  <TableHead>Activa</TableHead>
                  <TableHead className="w-16" />
                </TableRow>
              </TableHeader>
              <TableBody>
                {reglas.map((r: ReglaRecordatorio) => (
                  <TableRow key={r.id}>
                    <TableCell className="font-medium max-w-xs">
                      <div className="overflow-hidden">
                        <span>{r.nombre}</span>
                        {r.mensaje_plantilla && (
                          <p className="text-muted-foreground text-xs mt-0.5 truncate">
                            {r.mensaje_plantilla}
                          </p>
                        )}
                      </div>
                    </TableCell>
                    <TableCell>
                      {formatHoras(
                        r.tipo === 'despues' ? r.horas_despues : r.horas_antes,
                        r.tipo,
                      )}
                    </TableCell>
                    <TableCell>{badgeCanal(r.canal)}</TableCell>
                    <TableCell>
                      <Switch
                        checked={r.activa}
                        onCheckedChange={() => toggleActiva(r.id, r.activa)}
                      />
                    </TableCell>
                    <TableCell>
                      <TooltipButton
                        variant="ghost"
                        size="icon"
                        tooltip="Eliminar recordatorio"
                        onClick={() => handleEliminar(r.id)}
                      >
                        <Trash2 className="size-4" />
                      </TooltipButton>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}

          {totalPages > 1 && (
            <div className="flex items-center justify-center gap-2 pt-4">
              <Button variant="outline" size="sm" disabled={page <= 1} onClick={() => setPage(page - 1)}>
                <ChevronLeft className="size-4" />
              </Button>
              <span className="text-sm">{page} / {totalPages}</span>
              <Button variant="outline" size="sm" disabled={page >= totalPages} onClick={() => setPage(page + 1)}>
                <ChevronRight className="size-4" />
              </Button>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

export default function Recordatorios() {
  return (
    <Tabs defaultValue="reglas" className="space-y-4">
      <TabsList>
        <TabsTrigger value="reglas">Reglas</TabsTrigger>
        <TabsTrigger value="historial">Historial</TabsTrigger>
      </TabsList>
      <TabsContent value="reglas">
        <TablaReglas />
      </TabsContent>
      <TabsContent value="historial">
        <TablaHistorial />
      </TabsContent>
    </Tabs>
  );
}
