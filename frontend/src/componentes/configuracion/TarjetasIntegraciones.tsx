/* [D33] Tarjetas de integraciones: Haddock y reseñas/WhatsApp.
 * Sin estado propio: todo llega por props desde Configuracion. */

import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import type { EstadoConfiguracion } from '../../hooks/useConfiguracion';
import type { CambiarCampo } from '../config-bdp/tipos';
import { AyudaTarjeta } from './AyudaTarjeta';

interface TarjetasIntegracionesProps {
  config: EstadoConfiguracion;
  cambiarCampo: CambiarCampo;
}

export function TarjetasIntegraciones({ config, cambiarCampo }: TarjetasIntegracionesProps) {
  return (
    <>
      {/* [034A-3] URL de Haddock para vista detallada
        * [064A-5] Token API y toggle de sincronización con Haddock POS API */}
      <Card>
        <CardHeader>
          <CardTitle>Enlace a Haddock</CardTitle>
          <CardDescription>Sincronizar al día las ventas con Haddock por su POS API</CardDescription>
          <CardAction>
            <AyudaTarjeta id="haddock" titulo="Enlace a Haddock" />
          </CardAction>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <div className="flex flex-col gap-2">
            <Label htmlFor="url-haddock">URL de Haddock</Label>
            <Input
              id="url-haddock"
              type="url"
              value={config.url_haddock}
              onChange={(e) => cambiarCampo('url_haddock', e.target.value)}
              placeholder="https://app.haddock.com/tu-restaurante"
            />
            <p className="text-xs text-muted-foreground">
              Si tu restaurante usa <a href="https://haddock.com" target="_blank" rel="noopener noreferrer" className="underline">Haddock</a> para gestión financiera detallada, pega aquí la URL de tu cuenta.
            </p>
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="haddock-api-token">Token API de Haddock</Label>
            <Input
              id="haddock-api-token"
              type="password"
              value={config.haddock_api_token}
              onChange={(e) => cambiarCampo('haddock_api_token', e.target.value)}
              placeholder="Token Base64..."
            />
            <p className="text-xs text-muted-foreground">
              Se obtiene en Haddock → Configuración → Integraciones → POS API → Conectar.
            </p>
          </div>
          <div className="flex items-center justify-between">
            <Label htmlFor="haddock-sync">Sincronizar ventas automáticamente</Label>
            <Switch
              id="haddock-sync"
              checked={config.haddock_sync_enabled}
              onCheckedChange={(checked) => cambiarCampo('haddock_sync_enabled', checked)}
            />
          </div>
        </CardContent>
      </Card>

      {/* [134A-4] Reseñas y CTA WhatsApp — configuración para review gating y botones CTA */}
      <Card>
        <CardHeader>
          <CardTitle>Pedir reseñas y enlazar WhatsApp</CardTitle>
          <CardDescription>A dónde va quien puntúa bien, y qué botones llevan los mensajes de WhatsApp</CardDescription>
          <CardAction>
            <AyudaTarjeta id="resenas" titulo="Pedir reseñas y enlazar WhatsApp" />
          </CardAction>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <div className="flex flex-col gap-2">
            <Label htmlFor="google-review-url">URL de Google Reviews</Label>
            <Input
              id="google-review-url"
              type="url"
              value={config.google_review_url}
              onChange={(e) => cambiarCampo('google_review_url', e.target.value)}
              placeholder="https://g.page/r/tu-restaurante/review"
            />
            <p className="text-xs text-muted-foreground">
              Los clientes que puntúen 4-5 estrellas serán redirigidos a esta URL para dejar reseña en Google.
            </p>
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="telefono-restaurante">Teléfono del restaurante</Label>
            <Input
              id="telefono-restaurante"
              type="tel"
              value={config.telefono_restaurante}
              onChange={(e) => cambiarCampo('telefono_restaurante', e.target.value)}
              placeholder="+34 612 345 678"
            />
            <p className="text-xs text-muted-foreground">
              Se mostrará en los mensajes de WhatsApp como enlace clickeable para llamar.
            </p>
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="url-reservas">URL de reservas</Label>
            <Input
              id="url-reservas"
              type="url"
              value={config.url_reservas}
              onChange={(e) => cambiarCampo('url_reservas', e.target.value)}
              placeholder="https://tu-restaurante.com/reservar"
            />
            <p className="text-xs text-muted-foreground">
              Se usará como botón "Reserva ya" en los mensajes de WhatsApp.
            </p>
          </div>
        </CardContent>
      </Card>
    </>
  );
}
