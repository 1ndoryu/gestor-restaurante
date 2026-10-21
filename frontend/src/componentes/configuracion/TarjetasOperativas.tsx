/* [D33] Tarjetas operativas: venta automática al completar reservas. Los datos
 * de prueba salieron de aquí a su propia sección (tocan datos, no ajustes). */

import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import type { EstadoConfiguracion } from '../../hooks/useConfiguracion';
import type { CambiarCampo } from '../config-bdp/tipos';

interface TarjetasOperativasProps {
  config: EstadoConfiguracion;
  cambiarCampo: CambiarCampo;
}

export function TarjetasOperativas({ config, cambiarCampo }: TarjetasOperativasProps) {
  return (
    <>
      {/* [014A-1] Toggle auto-venta al completar reserva */}
      <Card>
        <CardHeader>
          <CardTitle>Una venta al completar cada reserva</CardTitle>
          <CardDescription>Al marcar una reserva como completada se crea la venta sola, sin que nadie la teclee</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-between">
            <Label htmlFor="auto-venta">Activar venta automática</Label>
            <Switch
              id="auto-venta"
              checked={config.auto_venta_reserva}
              onCheckedChange={(checked) => cambiarCampo('auto_venta_reserva', checked)}
            />
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Cómo se anula una venta</CardTitle>
          <CardDescription>«Crédito completo» exige motivo y revierte la venta en el resumen diario; «Solo estado» la marca como anulada sin pedir motivo</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-between gap-4">
            <Label htmlFor="anulacion-modalidad">Modalidad de anulación</Label>
            <Select
              value={config.anulacion_modalidad}
              onValueChange={(v) => cambiarCampo('anulacion_modalidad', v)}
            >
              <SelectTrigger id="anulacion-modalidad" className="w-52">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="credito_completo">Crédito completo</SelectItem>
                <SelectItem value="estado_solo">Solo estado</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </CardContent>
      </Card>
    </>
  );
}
