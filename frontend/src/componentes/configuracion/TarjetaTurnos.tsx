/* Tarjeta de horarios de turnos de la pestaña General. Sin estado propio:
 * todo llega por props desde Configuracion. */

import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import type { EstadoConfiguracion } from '../../hooks/useConfiguracion';
import type { CambiarCampo } from '../config-bdp/tipos';
import { AyudaTarjeta } from './AyudaTarjeta';

interface TarjetaTurnosProps {
  config: EstadoConfiguracion;
  cambiarCampo: CambiarCampo;
}

export function TarjetaTurnos({ config, cambiarCampo }: TarjetaTurnosProps) {
  return (
    <>
      {/* [014A-4] Turnos configurables */}
      <Card>
        <CardHeader>
          <CardTitle>Horarios de turnos</CardTitle>
          <CardDescription>Define los rangos horarios para desayuno, comida y cena</CardDescription>
          <CardAction>
            <AyudaTarjeta id="turnos" titulo="Horarios de turnos" />
          </CardAction>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          {/* Una columna hasta 2xl: en tres, cada hora quedaba en ~77 px y recortaba «12:00 a. m.» (locales de 12 h). */}
          <div className="grid grid-cols-1 gap-4 2xl:grid-cols-3">
            <div className="flex flex-col gap-2">
              <Label className="font-semibold">Desayuno</Label>
              <div className="flex items-center gap-1">
                <Input
                  type="time"
                  value={config.hora_desayuno_inicio.slice(0, 5)}
                  onChange={(e) => cambiarCampo('hora_desayuno_inicio', e.target.value + ':00')}
                  className="w-36"
                />
                <span className="text-muted-foreground">-</span>
                <Input
                  type="time"
                  value={config.hora_desayuno_fin.slice(0, 5)}
                  onChange={(e) => cambiarCampo('hora_desayuno_fin', e.target.value + ':00')}
                  className="w-36"
                />
              </div>
            </div>
            <div className="flex flex-col gap-2">
              <Label className="font-semibold">Comida</Label>
              <div className="flex items-center gap-1">
                <Input
                  type="time"
                  value={config.hora_comida_inicio.slice(0, 5)}
                  onChange={(e) => cambiarCampo('hora_comida_inicio', e.target.value + ':00')}
                  className="w-36"
                />
                <span className="text-muted-foreground">-</span>
                <Input
                  type="time"
                  value={config.hora_comida_fin.slice(0, 5)}
                  onChange={(e) => cambiarCampo('hora_comida_fin', e.target.value + ':00')}
                  className="w-36"
                />
              </div>
            </div>
            <div className="flex flex-col gap-2">
              <Label className="font-semibold">Cena</Label>
              <div className="flex items-center gap-1">
                <Input
                  type="time"
                  value={config.hora_cena_inicio.slice(0, 5)}
                  onChange={(e) => cambiarCampo('hora_cena_inicio', e.target.value + ':00')}
                  className="w-36"
                />
                <span className="text-muted-foreground">-</span>
                <Input
                  type="time"
                  value={config.hora_cena_fin.slice(0, 5)}
                  onChange={(e) => cambiarCampo('hora_cena_fin', e.target.value + ':00')}
                  className="w-36"
                />
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </>
  );
}
