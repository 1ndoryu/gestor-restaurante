/* Formulario de trabajador (alta y edición) con un interruptor por sección del sistema. */

import { useState, type FormEvent } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Shield } from 'lucide-react';
import type { TrabajadorResponse } from '@/api/generated/gestionRestauranteAPI.schemas';
import { etiquetaSeccion } from './etiquetas';

export interface CamposForm {
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

export function FormularioTrabajador({
  secciones,
  seccionesError,
  trabajador,
  onSubmit,
  cargando,
}: {
  secciones: string[];
  seccionesError?: boolean;
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
              <span className="text-sm capitalize">{etiquetaSeccion(seccion)}</span>
            </div>
          ))}
          {secciones.length === 0 && (
            <p className="text-sm text-muted-foreground col-span-2">
              {seccionesError
                ? 'No se pudieron cargar las secciones: faltan permisos para listarlas.'
                : 'Cargando secciones...'}
            </p>
          )}
        </div>
        {/* [169A-3] Semántica visible: sin marcar nada = defecto de operativa
         * diaria; los cambios se aplican cuando el trabajador vuelve a entrar
         * (su token guarda los permisos). */}
        <p className="text-xs text-muted-foreground">
          Sin marcar nada se concede la operativa diaria (Panel, Ventas, Gastos,
          Reservas, Calendario, Clientes, Canales, No-Shows, Plano de Sala).
          Los cambios se aplican cuando el trabajador vuelve a entrar.
        </p>
      </div>

      <Button type="submit" disabled={cargando} className="self-end">
        {cargando ? 'Guardando...' : trabajador ? 'Guardar cambios' : 'Crear trabajador'}
      </Button>
    </form>
  );
}
