/* [147A-F5.6] Sección de mapeos BDP extraída de ConfigBdp (límite 300).
 * Tender map, order type map, customer code, poll interval. */

import { useState } from 'react';
import { Settings } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Switch } from '@/components/ui/switch';
import BdpArticleMapTable from '@/components/bdp-article-map-table';
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
        <span className="text-sm font-medium">Correspondencias propias de este BDP</span>
      </div>
      {mapeoError && (
        <p className="text-xs text-destructive">{mapeoError}</p>
      )}
      <div className="grid gap-4 md:grid-cols-2">
        <div className="flex flex-col gap-2">
          <Label htmlFor="bdp-tender-map">Formas de pago de Glory → códigos BDP</Label>
          <Textarea
            id="bdp-tender-map"
            className="font-mono text-xs"
            rows={4}
            value={config.bdp_tender_map}
            onChange={(e) => handleJsonChange('bdp_tender_map', e.target.value)}
            placeholder='{"efectivo": 1, "tarjeta": 2}'
          />
          <p className="text-xs text-muted-foreground">Configuración técnica: relaciona cada nombre usado en Glory con el identificador numérico de la forma de pago en BDP.</p>
        </div>
        <div className="flex flex-col gap-2">
          <Label htmlFor="bdp-order-type-map">Canales de Glory → tipos de pedido BDP</Label>
          <Textarea
            id="bdp-order-type-map"
            className="font-mono text-xs"
            rows={4}
            value={config.bdp_order_type_map}
            onChange={(e) => handleJsonChange('bdp_order_type_map', e.target.value)}
            placeholder='{"comedor": 1, "barra": 0, "delivery": 2}'
          />
          <p className="text-xs text-muted-foreground">Configuración técnica: 0=Barra, 1=Mesa y 2=Domicilio. Debe coincidir con la operación real del restaurante.</p>
        </div>
      </div>
      <div className="grid gap-4 md:grid-cols-2">
        <div className="flex flex-col gap-2">
          <Label htmlFor="bdp-default-article-code">Artículo BDP usado si no hay equivalencia</Label>
          <Input
            id="bdp-default-article-code"
            inputMode="numeric"
            value={config.bdp_default_article_code}
            onChange={(e) => cambiarCampo('bdp_default_article_code', e.target.value)}
            placeholder="1001"
          />
          <p className="text-xs text-muted-foreground">Debe ser un código numérico existente en el perfil BDP.</p>
        </div>
        <div className="flex flex-col gap-2">
          <Label htmlFor="bdp-default-article-name">Nombre visible de ese artículo</Label>
          <Input
            id="bdp-default-article-name"
            value={config.bdp_default_article_name}
            onChange={(e) => cambiarCampo('bdp_default_article_name', e.target.value)}
            placeholder="Venta Glory"
          />
        </div>
        <div className="flex flex-col gap-2">
          <Label htmlFor="bdp-default-customer">Código cliente BDP por defecto</Label>
          <Input
            id="bdp-default-customer"
            inputMode="numeric"
            value={config.bdp_default_customer_code}
            onChange={(e) => cambiarCampo('bdp_default_customer_code', e.target.value)}
            placeholder="Código numérico, por ejemplo 1"
          />
          <p className="text-xs text-muted-foreground">Código real del cliente genérico en BDP; no es el nombre “Consumidor final”.</p>
        </div>
        <div className="flex flex-col gap-2">
          <Label htmlFor="bdp-poll-interval">Frecuencia para actualizar estados (segundos)</Label>
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
        <div className="flex items-center justify-between gap-4 rounded-md border p-3">
          <div>
            <Label htmlFor="bdp-poll-enabled">Actualizar estados automáticamente</Label>
            <p className="text-xs text-muted-foreground">Solo consulta el estado de comandas; no crea ni modifica registros en BDP.</p>
          </div>
          <Switch
            id="bdp-poll-enabled"
            checked={config.bdp_poll_enabled}
            onCheckedChange={(checked) => cambiarCampo('bdp_poll_enabled', checked)}
          />
        </div>
        <div className="flex items-center justify-between gap-4 rounded-md border p-3">
          <div>
            <Label htmlFor="bdp-auto-sync-customers">Exigir cliente BDP confirmado</Label>
            <p className="text-xs text-muted-foreground">Si está activo, una venta con cliente sin código BDP se bloquea; nunca genera códigos automáticamente.</p>
          </div>
          <Switch
            id="bdp-auto-sync-customers"
            checked={config.bdp_auto_sync_customers}
            onCheckedChange={(checked) => cambiarCampo('bdp_auto_sync_customers', checked)}
          />
        </div>
      </div>

      {/* [147A-F5.6+5.7] Tabla de mapeo artículos Glory → BDP */}
      <BdpArticleMapTable />
    </div>
  );
}

export default ConfigBdpMapeos;
