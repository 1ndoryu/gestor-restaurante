/* [147A-F5.6] Sección de mapeos BDP extraída de ConfigBdp (límite 300).
 * Tender map, order type map, customer code, poll interval. */

import { useState } from 'react';
import { Settings } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import type { EstadoConfiguracion } from '../hooks/useConfiguracion';

interface ConfigBdpMapeosProps {
  config: EstadoConfiguracion;
  cambiarCampo: <K extends keyof EstadoConfiguracion>(campo: K, valor: EstadoConfiguracion[K]) => void;
}

function ConfigBdpMapeos({ config, cambiarCampo }: ConfigBdpMapeosProps) {
  const [mapeoError, setMapeoError] = useState<string | null>(null);

  const handleJsonChange = (campo: 'bdp_tender_map' | 'bdp_order_type_map', value: string) => {
    try {
      JSON.parse(value || '{}');
      setMapeoError(null);
    } catch {
      setMapeoError(`JSON inválido en ${campo === 'bdp_tender_map' ? 'mapeo de formas de pago' : 'mapeo de canales'}`);
    }
    cambiarCampo(campo, value);
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-2 pt-2">
        <Settings className="size-4 text-muted-foreground" />
        <span className="text-sm font-medium">Mapeos BDP</span>
      </div>
      {mapeoError && (
        <p className="text-xs text-destructive">{mapeoError}</p>
      )}
      <div className="grid gap-4 md:grid-cols-2">
        <div className="flex flex-col gap-2">
          <Label htmlFor="bdp-tender-map">Mapeo formas de pago → Tender BDP (JSON)</Label>
          <Textarea
            id="bdp-tender-map"
            className="font-mono text-xs"
            rows={4}
            value={config.bdp_tender_map}
            onChange={(e) => handleJsonChange('bdp_tender_map', e.target.value)}
            placeholder='{"efectivo": "EF", "tarjeta": "TC"}'
          />
          <p className="text-xs text-muted-foreground">Formato: {"{\"metodo_pago_glory\": \"CODIGO_TENDER_BDP\"}"}</p>
        </div>
        <div className="flex flex-col gap-2">
          <Label htmlFor="bdp-order-type-map">Mapeo canales → OrderType BDP (JSON)</Label>
          <Textarea
            id="bdp-order-type-map"
            className="font-mono text-xs"
            rows={4}
            value={config.bdp_order_type_map}
            onChange={(e) => handleJsonChange('bdp_order_type_map', e.target.value)}
            placeholder='{"sala": 1, "barra": 0, "domicilio": 2}'
          />
          <p className="text-xs text-muted-foreground">Formato: {"{\"canal_glory\": orderTypeInt}"} (0=Barra, 1=Mesa, 2=Domicilio)</p>
        </div>
      </div>
      <div className="grid gap-4 md:grid-cols-2">
        <div className="flex flex-col gap-2">
          <Label htmlFor="bdp-default-customer">Código cliente BDP por defecto</Label>
          <Input
            id="bdp-default-customer"
            value={config.bdp_default_customer_code}
            onChange={(e) => cambiarCampo('bdp_default_customer_code', e.target.value)}
            placeholder="Consumidor final"
          />
          <p className="text-xs text-muted-foreground">Se usa cuando la venta no tiene cliente asociado</p>
        </div>
        <div className="flex flex-col gap-2">
          <Label htmlFor="bdp-poll-interval">Intervalo de polling BDP (segundos)</Label>
          <Input
            id="bdp-poll-interval"
            type="number"
            min={10}
            max={600}
            value={config.bdp_poll_interval_secs}
            onChange={(e) => cambiarCampo('bdp_poll_interval_secs', Number(e.target.value))}
          />
          <p className="text-xs text-muted-foreground">Frecuencia de consulta de estado de órdenes BDP (10-600 s)</p>
        </div>
      </div>
    </div>
  );
}

export default ConfigBdpMapeos;
