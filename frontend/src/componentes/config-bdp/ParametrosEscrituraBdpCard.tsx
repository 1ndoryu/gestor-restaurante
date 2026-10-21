/* [10AA-3] Parámetros de las escrituras a BDP (D1/D3/M13).
 * Sin el mapeo IVA→TAVCode, el alta de un artículo se rechaza en BDP con
 * «el código de IVA de venta 0 no existe»: el cliente tiene que poder rellenarlo
 * aquí, viendo qué IVA usa cada uno de sus artículos. */

import { AlertTriangle } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { EstadoConfiguracion } from '@/hooks/useConfiguracion';
import type { CambiarCampo } from './tipos';
import { MapaJsonCrudo, MapaParejas } from '../configuracion/MapaParejas';

interface ParametrosEscrituraBdpCardProps {
  config: EstadoConfiguracion;
  cambiarCampo: CambiarCampo;
}

/** Cuenta las parejas válidas del mapa; -1 si el texto no es un JSON de objetos. */
function contarParejas(texto: string): number {
  try {
    const valor: unknown = JSON.parse(texto);
    if (typeof valor !== 'object' || valor === null) return -1;
    const parejas = Object.entries(valor as Record<string, unknown>)
      .filter(([, codigo]) => typeof codigo === 'number');
    return parejas.length;
  } catch {
    return -1;
  }
}

export default function ParametrosEscrituraBdpCard({
  config,
  cambiarCampo,
}: ParametrosEscrituraBdpCardProps) {
  const parejas = contarParejas(config.bdp_tav_map);

  return (
    <Card>
      <CardHeader>
        <CardTitle>Lo que hace falta para crear artículos y mover stock en el TPV</CardTitle>
        <CardDescription>
          Qué IVA corresponde a cada tipo, cuándo se envía la cola y qué códigos se usan por defecto.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        {parejas <= 0 && (
          <div className="flex items-start gap-2 rounded-md border border-amber-500 bg-amber-50 p-3 text-sm dark:bg-amber-950/20">
            <AlertTriangle className="mt-0.5 size-4 text-amber-600" />
            <span>
              Falta el mapeo de IVA: sin él, BDP rechaza el alta de artículos con «el código de IVA
              de venta 0 no existe».
            </span>
          </div>
        )}

        <MapaParejas
          id="bdp-tav-map"
          etiqueta="IVA de la web → código de IVA de venta del BDP"
          ayuda="Ejemplo: 10 → 1, 4 → 3, 21 → 2. Los códigos se ven en el TPV, en la ficha de cualquier artículo ya creado (IVA de venta)."
          columnaClave="IVA %"
          columnaValor="Código TPV"
          valor={config.bdp_tav_map}
          onCambiar={(json) => cambiarCampo('bdp_tav_map', json)}
        />
        <MapaJsonCrudo
          id="bdp-tav-map-json"
          etiqueta="Editar el mapeo de IVA como JSON (para copiarlo de otro restaurante)"
          valor={config.bdp_tav_map}
          onCambiar={(json) => cambiarCampo('bdp_tav_map', json)}
        />

        <div className="grid gap-4 md:grid-cols-2">
          <div className="flex flex-col gap-2">
            <Label htmlFor="bdp-push-modalidad">Cuándo se envía la cola</Label>
            <select
              id="bdp-push-modalidad"
              value={config.push_modalidad}
              onChange={(evento) => cambiarCampo('push_modalidad', evento.target.value)}
              className="h-9 max-w-md rounded-md border border-input bg-background px-3 text-sm"
            >
              <option value="automatico">Automático: cada cambio se envía a BDP</option>
              <option value="manual">Manual: solo al pulsar «Enviar a BDP»</option>
            </select>
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="bdp-rango-inicial">Primer código reservado para artículos nuevos</Label>
            <Input
              id="bdp-rango-inicial"
              type="number"
              value={config.bdp_articulo_rango_inicial}
              onChange={(evento) =>
                cambiarCampo('bdp_articulo_rango_inicial', Number(evento.target.value))
              }
              className="max-w-md"
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="bdp-almacen">Almacén por defecto (ajustes y traspasos)</Label>
            <Input
              id="bdp-almacen"
              type="number"
              value={config.bdp_almacen_default}
              onChange={(evento) => cambiarCampo('bdp_almacen_default', Number(evento.target.value))}
              className="max-w-md"
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="bdp-codreg">Motivo por defecto de regularización</Label>
            <Input
              id="bdp-codreg"
              type="number"
              value={config.bdp_codreg_default}
              onChange={(evento) => cambiarCampo('bdp_codreg_default', Number(evento.target.value))}
              className="max-w-md"
            />
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
