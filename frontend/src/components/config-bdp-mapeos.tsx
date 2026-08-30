/* [147A-F5.6] Sección de mapeos técnicos BDP extraída de ConfigBdp (límite 300).
 * Tender map, order type map, customer code, poll interval.
 * [237A-3] Props soloArticulos / soloMapeosTecnicos para renderizar
 *          selectivamente según la sección de ConfigBdp que lo invoca.
 * [208A-2/C1] Tras la auditoría (D1/D6) el CRUD de artículos se movió a la
 *          página Catálogo: este componente queda solo con mapeos técnicos
 *          (sin BdpArticleMapTable), porque Configuración ya no tiene CRUD. */

import { useState } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Switch } from '@/components/ui/switch';
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
      {mapeoError && (
        <p className="text-xs text-destructive">{mapeoError}</p>
      )}
      <div className="grid gap-4 md:grid-cols-2">
        <div className="flex flex-col gap-2">
          <Label htmlFor="bdp-tender-map">Formas de pago de la Aplicación Web → códigos BDP</Label>
          <Textarea
            id="bdp-tender-map"
            className="font-mono text-xs"
            rows={4}
            value={config.bdp_tender_map}
            onChange={(e) => handleJsonChange('bdp_tender_map', e.target.value)}
            placeholder='{"efectivo": 1, "tarjeta": 2}'
          />
          <p className="text-xs text-muted-foreground">Relaciona cada nombre de método de pago en la Aplicación Web con el identificador numérico en BDP.</p>
        </div>
        <div className="flex flex-col gap-2">
          <Label htmlFor="bdp-order-type-map">Canales de la Aplicación Web → tipos de pedido BDP</Label>
          <Textarea
            id="bdp-order-type-map"
            className="font-mono text-xs"
            rows={4}
            value={config.bdp_order_type_map}
            onChange={(e) => handleJsonChange('bdp_order_type_map', e.target.value)}
            placeholder='{"comedor": 1, "barra": 0, "delivery": 2}'
          />
          <p className="text-xs text-muted-foreground">0=Barra, 1=Mesa, 2=Domicilio. Debe coincidir con la operación real del restaurante.</p>
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
            placeholder="Venta Aplicación Web"
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
        <div className="flex items-center justify-between gap-4 rounded-md border p-3">
          <div>
            <Label htmlFor="bdp-auto-sync-customers">Exigir cliente BDP confirmado</Label>
            <p className="text-xs text-muted-foreground">Si está activo, una venta con cliente sin código BDP se bloquea; nunca genera códigos automáticamente.</p>
          </div>
          <Switch
            id="bdp-auto-sync-customers"
            checked={config.bdp_auto_sync_customers}
            onCheckedChange={(checked: boolean) => cambiarCampo('bdp_auto_sync_customers', checked)}
          />
        </div>
      </div>
    </div>
  );
}

export default ConfigBdpMapeos;
