/* [263A-16] Formulario de reserva — reescrito con shadcn Input + Button + Label + Textarea.
 * [024A-5] Soporte dual crear/editar: si se pasa reserva, pre-rellena y usa PUT.
 * [034A-7] Autocomplete de clientes existentes al escribir nombre. */

import { EstadoReserva, Reserva } from '../api/generated';
import useFormularioReserva from '../hooks/useFormularioReserva';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useRef } from 'react';

interface Props {
  onExito?: () => void;
  reserva?: Reserva;
}

function FormularioReserva({ onExito, reserva }: Props) {
  const {
    campos, cambiarCampo, error, manejarEnvio, cargando, mesasDisponibles, esEdicion,
    sugerenciasClientes, autocompletarAbierto, setAutocompletarAbierto, seleccionarCliente,
    marcarInteraccionCliente,
  } = useFormularioReserva(onExito, reserva);
  /* [034A-7] Ref del contenedor para cerrar autocomplete al hacer click fuera */
  const autocompleteRef = useRef<HTMLDivElement>(null);

  return (
    <div>
      {error && (
        <div className="mb-4 rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>
      )}

      <form className="flex flex-col gap-4" onSubmit={manejarEnvio}>
        <div className="grid grid-cols-2 gap-4">
          <div className="flex flex-col gap-2">
            <Label htmlFor="fecha">Fecha</Label>
            <Input id="fecha" type="date" value={campos.fecha} onChange={e => cambiarCampo('fecha', e.target.value)} />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="hora">Hora</Label>
            <Input id="hora" type="time" value={campos.hora} onChange={e => cambiarCampo('hora', e.target.value)} />
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          {/* [034A-7] Nombre con autocomplete de clientes existentes */}
          <div className="flex flex-col gap-2 relative" ref={autocompleteRef}>
            <Label htmlFor="nombreCliente">Nombre</Label>
            <Input
              id="nombreCliente"
              type="text"
              value={campos.nombreCliente}
              onChange={e => { marcarInteraccionCliente(); cambiarCampo('nombreCliente', e.target.value); }}
              onFocus={() => { marcarInteraccionCliente(); if (campos.nombreCliente.trim().length >= 2) setAutocompletarAbierto(true); }}
              onBlur={() => { setTimeout(() => setAutocompletarAbierto(false), 200); }}
              placeholder="Nombre"
              autoComplete="off"
            />
            {autocompletarAbierto && sugerenciasClientes.length > 0 && (
              <div className="absolute top-full left-0 right-0 z-50 mt-1 max-h-48 overflow-y-auto rounded-md border bg-popover shadow-md">
                {sugerenciasClientes.map(c => (
                  <Button
                    key={c.id}
                    variant="ghost"
                    className="w-full justify-start h-auto py-2 px-3 rounded-none text-left"
                    onMouseDown={(e) => { e.preventDefault(); seleccionarCliente(c); }}
                  >
                    <div className="flex flex-col">
                      <span className="font-medium text-sm">{c.nombre} {c.apellidos}</span>
                      <span className="text-xs text-muted-foreground">
                        {c.telefono || 'Sin teléfono'}{c.email ? ` · ${c.email}` : ''}
                      </span>
                    </div>
                  </Button>
                ))}
              </div>
            )}
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="apellidosCliente">Apellidos</Label>
            <Input id="apellidosCliente" type="text" value={campos.apellidosCliente} onChange={e => cambiarCampo('apellidosCliente', e.target.value)} placeholder="Apellidos" />
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="flex flex-col gap-2">
            <Label htmlFor="numPersonas">Personas</Label>
            <Input id="numPersonas" type="number" min="1" value={campos.numPersonas} onChange={e => cambiarCampo('numPersonas', e.target.value)} />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="mesaId">Mesa</Label>
            <Select value={campos.mesaId || '__none__'} onValueChange={v => cambiarCampo('mesaId', v === '__none__' ? '' : v)}>
              <SelectTrigger id="mesaId"><SelectValue placeholder="Sin mesa" /></SelectTrigger>
              <SelectContent>
                <SelectItem value="__none__">Sin mesa</SelectItem>
                {mesasDisponibles.map(m => (
                  <SelectItem key={m.id} value={m.id}>Mesa {m.numero} — {m.zona}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="flex flex-col gap-2">
            <Label htmlFor="telefono">Teléfono</Label>
            <Input id="telefono" type="tel" value={campos.telefono} onChange={e => cambiarCampo('telefono', e.target.value)} placeholder="Opcional" />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="estado">Estado</Label>
            <Select value={campos.estado} onValueChange={v => cambiarCampo('estado', v as EstadoReserva)}>
              <SelectTrigger id="estado"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value={EstadoReserva.pendiente}>Pendiente</SelectItem>
                <SelectItem value={EstadoReserva.confirmada}>Confirmada</SelectItem>
                <SelectItem value={EstadoReserva.lista_espera}>Lista de espera</SelectItem>
                <SelectItem value={EstadoReserva.completada}>Completada</SelectItem>
                <SelectItem value={EstadoReserva.cancelada}>Cancelada</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        <div className="flex flex-col gap-2">
          <Label htmlFor="notas">Notas</Label>
          <Textarea id="notas" rows={3} value={campos.notas} onChange={e => cambiarCampo('notas', e.target.value)} placeholder="Alergias, preferencias, etc." />
        </div>

        <Button type="submit" className="w-full" disabled={cargando}>
          {cargando ? (esEdicion ? 'Guardando...' : 'Creando...') : (esEdicion ? 'Guardar Cambios' : 'Crear Reserva')}
        </Button>
      </form>
    </div>
  );
}

export default FormularioReserva;
