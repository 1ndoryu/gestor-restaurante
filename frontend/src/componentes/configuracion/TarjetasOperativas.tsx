/* Tarjetas operativas de la pestaña General: datos de prueba y venta automática
 * al completar reservas. La operación de datos de prueba la ejecuta Configuracion. */

import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import type { EstadoConfiguracion } from '../../hooks/useConfiguracion';
import type { CambiarCampo } from '../config-bdp/tipos';

interface TarjetasOperativasProps {
  config: EstadoConfiguracion;
  cambiarCampo: CambiarCampo;
  operandoSeed: boolean;
  ejecutarOperacion: (endpoint: string, descripcion: string) => Promise<void>;
}

export function TarjetasOperativas({ config, cambiarCampo, operandoSeed, ejecutarOperacion }: TarjetasOperativasProps) {
  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>Datos de prueba</CardTitle>
          <CardDescription>Cargar o eliminar los datos de demostración del sistema</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          <div className="flex gap-3 flex-wrap">
            <Button
              variant="outline"
              disabled={operandoSeed}
              onClick={() => ejecutarOperacion('admin/seed', 'Datos de prueba cargados')}
            >
              {operandoSeed ? 'Procesando...' : 'Recargar datos de prueba'}
            </Button>
            <Button
              variant="destructive"
              disabled={operandoSeed}
              onClick={() => ejecutarOperacion('admin/reset', 'Datos eliminados')}
            >
              Eliminar datos de prueba
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">
            "Recargar" restablece todos los datos demo. "Eliminar" borra todos los datos pero mantiene la cuenta.
          </p>
        </CardContent>
      </Card>

      {/* [014A-1] Toggle auto-venta al completar reserva */}
      <Card>
        <CardHeader>
          <CardTitle>Ventas automáticas</CardTitle>
          <CardDescription>Crear una venta automáticamente cuando una reserva se marca como completada</CardDescription>
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
    </>
  );
}
