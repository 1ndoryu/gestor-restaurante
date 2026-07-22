/* [134A-3] Página de gestión de reglas de inactividad de clientes.
 * Permite crear, editar, activar/desactivar y eliminar reglas que
 * disparan mensajes automáticos cuando un cliente lleva N días sin actividad. */

import { useState, type FormEvent } from 'react';
import { useInactividad } from '../hooks/useInactividad';
import { Button } from '@/components/ui/button';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Badge } from '@/components/ui/badge';
import { Textarea } from '@/components/ui/textarea';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
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
import { Trash2, Pencil, Plus } from 'lucide-react';
import type { ReglaInactividad } from '../api/generated/gestionRestauranteAPI.schemas';

interface CamposForm {
  nombre: string;
  dias_inactividad: number;
  canal: string;
  mensaje_plantilla: string;
  activa: boolean;
}

const camposVacios: CamposForm = {
  nombre: '',
  dias_inactividad: 30,
  canal: 'whatsapp',
  mensaje_plantilla: '',
  activa: true,
};

const canalesDisponibles = ['whatsapp', 'email', 'sms'];

function FormularioRegla({
  regla,
  onSubmit,
  cargando,
}: {
  regla?: ReglaInactividad | null;
  onSubmit: (campos: CamposForm) => void;
  cargando: boolean;
}) {
  const [campos, setCampos] = useState<CamposForm>(
    regla
      ? {
          nombre: regla.nombre,
          dias_inactividad: regla.dias_inactividad,
          canal: regla.canal,
          mensaje_plantilla: regla.mensaje_plantilla,
          activa: regla.activa,
        }
      : camposVacios,
  );

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    if (!campos.nombre.trim() || !campos.mensaje_plantilla.trim()) return;
    onSubmit(campos);
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="nombre">Nombre de la regla</Label>
        <Input
          id="nombre"
          value={campos.nombre}
          onChange={(e) => setCampos({ ...campos, nombre: e.target.value })}
          placeholder="Ej: Clientes inactivos 30 días"
        />
      </div>

      <div className="flex flex-col gap-1.5">
        <Label htmlFor="dias">Días de inactividad</Label>
        <Input
          id="dias"
          type="number"
          min={1}
          value={campos.dias_inactividad}
          onChange={(e) =>
            setCampos({ ...campos, dias_inactividad: parseInt(e.target.value, 10) || 1 })
          }
        />
      </div>

      <div className="flex flex-col gap-1.5">
        <Label htmlFor="canal">Canal de envío</Label>
        <Select
          value={campos.canal}
          onValueChange={(v) => setCampos({ ...campos, canal: v })}
        >
          <SelectTrigger id="canal">
            <SelectValue placeholder="Seleccionar canal" />
          </SelectTrigger>
          <SelectContent>
            {canalesDisponibles.map((c) => (
              <SelectItem key={c} value={c}>
                {c.charAt(0).toUpperCase() + c.slice(1)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div className="flex flex-col gap-1.5">
        <Label htmlFor="plantilla">Mensaje plantilla</Label>
        <Textarea
          id="plantilla"
          value={campos.mensaje_plantilla}
          onChange={(e) => setCampos({ ...campos, mensaje_plantilla: e.target.value })}
          placeholder="Hola {nombre}, te echamos de menos..."
          rows={4}
        />
      </div>

      <div className="flex items-center gap-2">
        <Switch
          id="activa"
          checked={campos.activa}
          onCheckedChange={(v) => setCampos({ ...campos, activa: v })}
        />
        <Label htmlFor="activa">Activa</Label>
      </div>

      <Button type="submit" disabled={cargando} className="w-full">
        {regla ? 'Guardar cambios' : 'Crear regla'}
      </Button>
    </form>
  );
}

export default function Inactividad() {
  const {
    reglas,
    isLoading,
    modalCrear,
    setModalCrear,
    reglaEditar,
    setReglaEditar,
    crearRegla,
    actualizarRegla,
    eliminarRegla,
    creando,
    actualizando,
  } = useInactividad();

  const [confirmarEliminar, setConfirmarEliminar] = useState<string | null>(null);

  if (isLoading) {
    return <p className="p-4 text-muted-foreground">Cargando reglas de inactividad...</p>;
  }

  return (
    <div className="space-y-4 p-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Inactividad de clientes</h1>
        <Button onClick={() => setModalCrear(true)}>
          <Plus className="mr-2 h-4 w-4" />
          Nueva regla
        </Button>
      </div>

      <div className="rounded-md border">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Nombre</TableHead>
            <TableHead>Días</TableHead>
            <TableHead>Canal</TableHead>
            <TableHead>Estado</TableHead>
            <TableHead className="text-right">Acciones</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {reglas.length === 0 && (
            <TableRow>
              <TableCell colSpan={5} className="text-center text-muted-foreground">
                No hay reglas de inactividad configuradas
              </TableCell>
            </TableRow>
          )}
          {reglas.map((r) => (
            <TableRow key={r.id}>
              <TableCell className="font-medium">{r.nombre}</TableCell>
              <TableCell>{r.dias_inactividad}</TableCell>
              <TableCell>
                <Badge variant="outline">
                  {r.canal.charAt(0).toUpperCase() + r.canal.slice(1)}
                </Badge>
              </TableCell>
              <TableCell>
                <Badge variant={r.activa ? 'default' : 'secondary'}>
                  {r.activa ? 'Activa' : 'Inactiva'}
                </Badge>
              </TableCell>
              <TableCell className="text-right">
                <div className="flex justify-end gap-1">
                  <TooltipButton
                    variant="ghost"
                    size="icon"
                    onClick={() => setReglaEditar(r)}
                    tooltip="Editar regla de inactividad"
                  >
                    <Pencil className="h-4 w-4" />
                  </TooltipButton>
                  <TooltipButton
                    variant="ghost"
                    size="icon"
                    onClick={() => setConfirmarEliminar(r.id)}
                    tooltip="Eliminar regla de inactividad"
                  >
                    <Trash2 className="h-4 w-4 text-destructive" />
                  </TooltipButton>
                </div>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      </div>

      {/* Modal crear */}
      <Dialog open={modalCrear} onOpenChange={(v: boolean) => !v && setModalCrear(false)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Nueva regla de inactividad</DialogTitle>
          </DialogHeader>
          <FormularioRegla
            onSubmit={(c) =>
              crearRegla({
                nombre: c.nombre,
                dias_inactividad: c.dias_inactividad,
                canal: c.canal,
                mensaje_plantilla: c.mensaje_plantilla,
              })
            }
            cargando={creando}
          />
        </DialogContent>
      </Dialog>

      {/* Modal editar */}
      <Dialog
        open={!!reglaEditar}
        onOpenChange={(v: boolean) => !v && setReglaEditar(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Editar regla</DialogTitle>
          </DialogHeader>
          {reglaEditar && (
            <FormularioRegla
              regla={reglaEditar}
              onSubmit={(c) =>
                actualizarRegla(reglaEditar.id, {
                  nombre: c.nombre,
                  dias_inactividad: c.dias_inactividad,
                  canal: c.canal,
                  mensaje_plantilla: c.mensaje_plantilla,
                  activa: c.activa,
                })
              }
              cargando={actualizando}
            />
          )}
        </DialogContent>
      </Dialog>

      {/* Modal confirmar eliminación */}
      <Dialog
        open={!!confirmarEliminar}
        onOpenChange={(v: boolean) => !v && setConfirmarEliminar(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Confirmar eliminación</DialogTitle>
          </DialogHeader>
          <p>¿Eliminar esta regla de inactividad? Esta acción no se puede deshacer.</p>
          <div className="flex justify-end gap-2 mt-4">
            <Button variant="outline" onClick={() => setConfirmarEliminar(null)}>
              Cancelar
            </Button>
            <Button
              variant="destructive"
              onClick={() => {
                if (confirmarEliminar) {
                  eliminarRegla(confirmarEliminar);
                  setConfirmarEliminar(null);
                }
              }}
            >
              Eliminar
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
