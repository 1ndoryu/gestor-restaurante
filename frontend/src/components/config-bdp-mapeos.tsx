/* [147A-F5.6] Sección de mapeos técnicos BDP extraída de ConfigBdp (límite 300).
 * Tender map, order type map, customer code, poll interval.
 * [237A-3] Props soloArticulos / soloMapeosTecnicos para renderizar
 *          selectivamente según la sección de ConfigBdp que lo invoca.
 * [208A-2/C1] Tras la auditoría (D1/D6) el CRUD de artículos se movió a la
 *          página Catálogo: este componente queda solo con mapeos técnicos
 *          (sin BdpArticleMapTable), porque Configuración ya no tiene CRUD. */

import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { MapaJsonCrudo, MapaParejas } from '../componentes/configuracion/MapaParejas';
import type { EstadoConfiguracion } from '../hooks/useConfiguracion';

interface ConfigBdpMapeosProps {
  config: EstadoConfiguracion;
  cambiarCampo: <K extends keyof EstadoConfiguracion>(campo: K, valor: EstadoConfiguracion[K]) => void;
}

function ConfigBdpMapeos({ config, cambiarCampo }: ConfigBdpMapeosProps) {

  return (
    <div className="flex flex-col gap-4">
      {/* [D33] Los mapas se editan como parejas, no como JSON: un carácter mal
          puesto antes dejaba el ajuste inservible y el aviso era una línea roja. */}
      <div className="grid gap-4 md:grid-cols-2">
        <MapaParejas
          id="bdp-tender-map"
          etiqueta="Forma de pago de la web → código BDP"
          ayuda="Cada método de pago de la app con su identificador numérico en el TPV."
          columnaClave="Efectivo, tarjeta, bizum…"
          columnaValor="Código"
          valor={config.bdp_tender_map}
          onCambiar={(json) => cambiarCampo('bdp_tender_map', json)}
        />
        <MapaParejas
          id="bdp-order-type-map"
          etiqueta="Canal de pedido de la web → tipo de pedido BDP"
          ayuda="0 = barra, 1 = mesa, 2 = domicilio, según la operación real del restaurante."
          columnaClave="Comedor, barra, delivery…"
          columnaValor="Tipo"
          valor={config.bdp_order_type_map}
          onCambiar={(json) => cambiarCampo('bdp_order_type_map', json)}
        />
      </div>
      <MapaJsonCrudo
        id="bdp-tender-map-json"
        etiqueta="Editar los mapas como JSON (para copiar de otro restaurante)"
        valor={config.bdp_tender_map}
        onCambiar={(json) => cambiarCampo('bdp_tender_map', json)}
      />
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
