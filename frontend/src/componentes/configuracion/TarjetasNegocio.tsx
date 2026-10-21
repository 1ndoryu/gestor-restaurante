/* [D33] Tarjetas del grupo «Mi restaurante»: nombre, reservas e impuestos. La clave de Groq
 * pasó a «A qué se conecta»: es una conexión a un servicio externo.
 * Sin estado propio: todo llega por props desde Configuracion. */

import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import type { EstadoConfiguracion } from '../../hooks/useConfiguracion';
import type { CambiarCampo } from '../config-bdp/tipos';

interface TarjetasNegocioProps {
  config: EstadoConfiguracion;
  cambiarCampo: CambiarCampo;
}

export function TarjetasNegocio({ config, cambiarCampo }: TarjetasNegocioProps) {
  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>El nombre que sale en los tickets y en la web</CardTitle>
          <CardDescription>Cómo aparece el restaurante para quien lo busca y para quien recibe una factura</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex flex-col gap-2">
            <Label htmlFor="nombre">Nombre del restaurante</Label>
            <Input
              id="nombre"
              value={config.nombre_restaurante}
              onChange={(e) => cambiarCampo('nombre_restaurante', e.target.value)}
              placeholder="Ej: Restaurante La Gloria"
            />
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Qué datos se piden al reservar</CardTitle>
          <CardDescription>Lo que el cliente no puede dejar en blanco al reservar una mesa</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          {/* [303A-16] Fecha y hora siempre obligatorios — switches deshabilitados para
           * que el dueño vea la lista completa. No son configurables porque una reserva
           * sin fecha/hora no es válida a nivel de modelo. */}
          <div className="flex items-center justify-between opacity-70">
            <Label>Fecha</Label>
            <Switch checked disabled />
          </div>
          <div className="flex items-center justify-between opacity-70">
            <Label>Hora</Label>
            <Switch checked disabled />
          </div>
          <div className="flex items-center justify-between">
            <Label htmlFor="nombre-obligatorio">Nombre</Label>
            <Switch
              id="nombre-obligatorio"
              checked={config.reserva_nombre_obligatorio}
              onCheckedChange={(checked) => cambiarCampo('reserva_nombre_obligatorio', checked)}
            />
          </div>
          <div className="flex items-center justify-between">
            <Label htmlFor="apellidos-obligatorio">Apellidos</Label>
            <Switch
              id="apellidos-obligatorio"
              checked={config.reserva_apellidos_obligatorio}
              onCheckedChange={(checked) => cambiarCampo('reserva_apellidos_obligatorio', checked)}
            />
          </div>
          <div className="flex items-center justify-between">
            <Label htmlFor="email-obligatorio">Email</Label>
            <Switch
              id="email-obligatorio"
              checked={config.reserva_email_obligatorio}
              onCheckedChange={(checked) => cambiarCampo('reserva_email_obligatorio', checked)}
            />
          </div>
          <div className="flex items-center justify-between">
            <Label htmlFor="telefono-obligatorio">Teléfono</Label>
            <Switch
              id="telefono-obligatorio"
              checked={config.reserva_telefono_obligatorio}
              onCheckedChange={(checked) => cambiarCampo('reserva_telefono_obligatorio', checked)}
            />
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>El IVA que se aplica por defecto</CardTitle>
          <CardDescription>El impuesto que llevan los artículos nuevos si nadie lo cambia</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex flex-col gap-2">
            <Label htmlFor="iva">IVA por defecto (%)</Label>
            <Input
              id="iva"
              type="number"
              min={0}
              max={100}
              step={0.01}
              value={config.iva_por_defecto}
              onChange={(e) => cambiarCampo('iva_por_defecto', Number(e.target.value))}
              className="max-w-32"
            />
          </div>
        </CardContent>
      </Card>

    </>
  );
}
