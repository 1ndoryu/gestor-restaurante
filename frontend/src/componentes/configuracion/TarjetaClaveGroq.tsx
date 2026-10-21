/* [D33] Clave de Groq: vive en «A qué se conecta» porque es una conexión a un
 * servicio externo, no un ajuste del restaurante. Con ella la app lee el texto
 * de facturas y albaranes subidos como foto. */

import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import type { EstadoConfiguracion } from '../../hooks/useConfiguracion';
import type { CambiarCampo } from '../config-bdp/tipos';

interface TarjetaClaveGroqProps {
  config: EstadoConfiguracion;
  cambiarCampo: CambiarCampo;
}

export function TarjetaClaveGroq({ config, cambiarCampo }: TarjetaClaveGroqProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Leer facturas y albaranes con IA</CardTitle>
        <CardDescription>
          Con una clave de Groq, la app reconoce sola el texto de los documentos que subes.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="flex flex-col gap-2">
          <Label htmlFor="groq-api-key">Clave de Groq</Label>
          <Input
            id="groq-api-key"
            type="password"
            value={config.groq_api_key}
            onChange={(e) => cambiarCampo('groq_api_key', e.target.value)}
            placeholder="gsk_..."
          />
          <p className="text-xs text-muted-foreground">
            Se obtiene gratis en{' '}
            <a
              href="https://console.groq.com/keys"
              target="_blank"
              rel="noopener noreferrer"
              className="underline"
            >
              console.groq.com/keys
            </a>
            . Sin clave, los documentos se suben igual pero hay que teclearlos.
          </p>
        </div>
      </CardContent>
    </Card>
  );
}
