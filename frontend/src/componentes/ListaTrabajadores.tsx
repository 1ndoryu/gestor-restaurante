/* [134A-2] Página de gestión de trabajadores con permisos por sección.
 * CRUD + panel de permisos con checkboxes por sección.
 * El propietario asigna/quita acceso a cada sección del sistema. */

import { useState, type FormEvent } from 'react';
import { useTrabajadores } from '../hooks/useTrabajadores';
import { Button } from '@/components/ui/button';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Badge } from '@/components/ui/badge';
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
import { Trash2, Pencil, Shield } from 'lucide-react';
import type { TrabajadorResponse } from '../api/generated/gestionRestauranteAPI.schemas';

interface CamposForm {
  nombre: string;
  email: string;
  password: string;
  cargo: string;
  permisos: string[];
}

const camposVacios: CamposForm = {
  nombre: '',
  email: '',
  password: '',
  cargo: '',
  permisos: [],
};

function FormularioTrabajador({
  secciones,
  trabajador,
  onSubmit,
  cargando,
}: {
  secciones: string[];
  trabajador?: TrabajadorResponse | null;
  onSubmit: (campos: CamposForm) => void;
  cargando: boolean;
}) {
  const [campos, setCampos] = useState<CamposForm>(
    trabajador
      ? {
          nombre: trabajador.nombre,
          email: trabajador.email,
          password: '',
          cargo: trabajador.cargo,
          permisos: trabajador.permisos.filter((p) => p.permitido).map((p) => p.seccion),
        }
      : camposVacios
  );

  const togglePermiso = (seccion: string) => {
    setCampos((prev) => ({
      ...prev,
      permisos: prev.permisos.includes(seccion)
        ? prev.permisos.filter((s) => s !== seccion)
        : [...prev.permisos, seccion],
    }));
  };

  const enviar = (e: FormEvent) => {
    e.preventDefault();
    onSubmit(campos);
  };

  return (
    <form onSubmit={enviar} className="flex flex-col gap-4">
      <div className="grid grid-cols-2 gap-4">
        <div className="flex flex-col gap-1.5">
          <Label>Nombre</Label>
          <Input
            value={campos.nombre}
            onChange={(e) => setCampos((p) => ({ ...p, nombre: e.target.value }))}
            required
          />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label>Email</Label>
          <Input
            type="email"
            value={campos.email}
            onChange={(e) => setCampos((p) => ({ ...p, email: e.target.value }))}
            required
          />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label>{trabajador ? 'Nueva contraseña (dejar vacío para no cambiar)' : 'Contraseña'}</Label>
          <Input
            type="password"
            value={campos.password}
            onChange={(e) => setCampos((p) => ({ ...p, password: e.target.value }))}
            required={!trabajador}
          />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label>Cargo</Label>
          <Input
            value={campos.cargo}
            onChange={(e) => setCampos((p) => ({ ...p, cargo: e.target.value }))}
            placeholder="Ej: Camarero, Cocinero, Gerente..."
          />
        </div>
      </div>

      <div className="flex flex-col gap-2">
        <Label className="flex items-center gap-2">
          <Shield className="size-4" /> Permisos por sección
        </Label>
        <div className="grid grid-cols-2 gap-2 rounded-md border p-3">
          {secciones.map((seccion) => (
            <div key={seccion} className="flex items-center gap-2">
              <Switch
                checked={campos.permisos.includes(seccion)}
                onCheckedChange={() => togglePermiso(seccion)}
              />
              <span className="text-sm capitalize">{seccion.replace(/_/g, ' ')}</span>
            </div>
          ))}
          {secciones.length === 0 && (
            <p className="text-sm text-muted-foreground col-span-2">Cargando secciones...</p>
          )}
        </div>
      </div>

      <Button type="submit" disabled={cargando} className="self-end">
        {cargando ? 'Guardando...' : trabajador ? 'Guardar cambios' : 'Crear trabajador'}
      </Button>
    </form>
  );
}

export default function ListaTrabajadores() {
  const {
    trabajadores,
    secciones,
    isLoading,
    modalCrear,
    setModalCrear,
    trabajadorEditar,
    setTrabajadorEditar,
    crearTrabajador,
    actualizarTrabajador,
    eliminarTrabajador,
    creando,
    actualizando,
  } = useTrabajadores();

  const handleCrear = (campos: CamposForm) => {
    crearTrabajador({
      nombre: campos.nombre,
      email: campos.email,
      password: campos.password,
      cargo: campos.cargo || null,
      permisos: campos.permisos.length > 0 ? campos.permisos : null,
    });
  };

  const handleActualizar = (campos: CamposForm) => {
    if (!trabajadorEditar) return;
    actualizarTrabajador(trabajadorEditar.id, {
      nombre: campos.nombre,
      email: campos.email,
      password: campos.password || null,
      cargo: campos.cargo || null,
      permisos: campos.permisos,
    });
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">
          {trabajadores.length} trabajador{trabajadores.length !== 1 ? 'es' : ''}
        </p>
        <Button onClick={() => setModalCrear(true)}>+ Nuevo Trabajador</Button>
      </div>

      <Dialog open={modalCrear} onOpenChange={setModalCrear}>
        <DialogContent className="sm:max-w-xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Nuevo Trabajador</DialogTitle>
          </DialogHeader>
          <FormularioTrabajador secciones={secciones} onSubmit={handleCrear} cargando={creando} />
        </DialogContent>
      </Dialog>

      <Dialog open={!!trabajadorEditar} onOpenChange={(open: boolean) => { if (!open) setTrabajadorEditar(null); }}>
        <DialogContent className="sm:max-w-xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Editar Trabajador</DialogTitle>
          </DialogHeader>
          {trabajadorEditar && (
            <FormularioTrabajador
              secciones={secciones}
              trabajador={trabajadorEditar}
              onSubmit={handleActualizar}
              cargando={actualizando}
            />
          )}
        </DialogContent>
      </Dialog>

      {isLoading ? (
        <p className="text-sm text-muted-foreground">Cargando...</p>
      ) : trabajadores.length > 0 ? (
        <div className="rounded-md border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Nombre</TableHead>
                <TableHead>Email</TableHead>
                <TableHead>Cargo</TableHead>
                <TableHead>Permisos</TableHead>
                <TableHead>Estado</TableHead>
                <TableHead className="w-20"></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {trabajadores.map((t) => (
                <TableRow key={t.id}>
                  <TableCell className="font-medium">{t.nombre}</TableCell>
                  <TableCell>{t.email}</TableCell>
                  <TableCell>{t.cargo || '—'}</TableCell>
                  <TableCell>
                    <div className="flex flex-wrap gap-1">
                      {t.permisos.filter((p) => p.permitido).map((p) => (
                        <Badge key={p.seccion} variant="secondary" className="text-xs capitalize">
                          {p.seccion.replace(/_/g, ' ')}
                        </Badge>
                      ))}
                      {t.permisos.filter((p) => p.permitido).length === 0 && (
                        <span className="text-xs text-muted-foreground">Sin permisos</span>
                      )}
                    </div>
                  </TableCell>
                  <TableCell>
                    <Badge variant={t.activo ? 'default' : 'destructive'}>
                      {t.activo ? 'Activo' : 'Inactivo'}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <div className="flex gap-1">
                      <TooltipButton variant="ghost" size="icon" onClick={() => setTrabajadorEditar(t)} tooltip="Editar trabajador">
                        <Pencil className="size-4" />
                      </TooltipButton>
                      <TooltipButton
                        variant="ghost"
                        size="icon"
                        onClick={() => eliminarTrabajador(t.id)}
                        tooltip="Eliminar trabajador"
                      >
                        <Trash2 className="size-4" />
                      </TooltipButton>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      ) : (
        <div className="text-center py-12 text-muted-foreground">
          <Shield className="size-12 mx-auto mb-3 opacity-30" />
          <p>No hay trabajadores registrados</p>
          <p className="text-sm mt-1">Crea trabajadores y asigna permisos para que accedan al sistema</p>
        </div>
      )}
    </div>
  );
}
