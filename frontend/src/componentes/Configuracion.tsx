/* [263A-16] Configuración — reescrita con shadcn Card + Switch + Input.
 * Campos obligatorios al reservar, IVA por defecto, nombre del restaurante.
 * [283A-8] Sección de API key de Groq para digitalización de documentos.
 * [283A-23] Pestañas: General + Integraciones Marketing.
 * [283A-27] Pestaña Chatbot con gestión de API Keys.
 * [283A-39] Card de datos de prueba: botón seed y botón reset. */

import { useState } from 'react';
import { useConfiguracion } from '../hooks/useConfiguracion';
import IntegracionesMarketing from './IntegracionesMarketing';
import ConfigChatbot from './ConfigChatbot';
import ConfigBdp from './ConfigBdp';
import PanelBdpBackup from './PanelBdpBackup';
import axios from '@/api/axios-instance';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { toast } from 'sonner';

function Configuracion() {
  const { config, cambiarCampo, guardar, mensaje, cargando, guardando } = useConfiguracion();
  const [operandoSeed, setOperandoSeed] = useState(false);

  /* [303A-2] Migrado de raw fetch a axios para usar interceptors JWT/401 */
  async function ejecutarOperacion(endpoint: string, descripcion: string) {
    setOperandoSeed(true);
    try {
      const resp = await axios.post(`/api/${endpoint}`);
      toast.success(descripcion, { description: resp.data.mensaje });
    } catch (err: unknown) {
      /* [044A-3] ErrorResponse usa 'message', no 'mensaje' */
      const msg = (err as { response?: { data?: { message?: string } } })?.response?.data?.message ?? 'No se pudo conectar con el servidor';
      toast.error('Error', { description: msg });
    } finally {
      setOperandoSeed(false);
    }
  }

  if (cargando) return <p className="text-sm text-muted-foreground">Cargando configuración...</p>;

  return (
    <Tabs defaultValue="general" className="max-w-5xl">
      <TabsList>
        <TabsTrigger value="general">General</TabsTrigger>
        <TabsTrigger value="integraciones">Integraciones</TabsTrigger>
        <TabsTrigger value="chatbot">Chatbot</TabsTrigger>
        <TabsTrigger value="bdp">BDP</TabsTrigger>
      </TabsList>

      <TabsContent value="general" className="flex flex-col gap-6 mt-4">
      <Card>
        <CardHeader>
          <CardTitle>Datos del restaurante</CardTitle>
          <CardDescription>Información general del establecimiento</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex flex-col gap-2">
            <Label htmlFor="nombre">Nombre del restaurante</Label>
            <Input
              id="nombre"
              value={config.nombre_restaurante}
              onChange={(e) => cambiarCampo('nombre_restaurante', e.target.value)}
              placeholder="Ej: Restaurante La Gloria"
            />
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Campos obligatorios al reservar</CardTitle>
          <CardDescription>Define qué datos se requieren para crear una reserva</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          {/* [303A-16] Fecha y hora siempre obligatorios — switches deshabilitados para
           * que el dueño vea la lista completa. No son configurables porque una reserva
           * sin fecha/hora no es válida a nivel de modelo. */}
          <div className="flex items-center justify-between opacity-70">
            <Label>Fecha</Label>
            <Switch checked disabled />
          </div>
          <div className="flex items-center justify-between opacity-70">
            <Label>Hora</Label>
            <Switch checked disabled />
          </div>
          <div className="flex items-center justify-between">
            <Label htmlFor="nombre-obligatorio">Nombre</Label>
            <Switch
              id="nombre-obligatorio"
              checked={config.reserva_nombre_obligatorio}
              onCheckedChange={(checked) => cambiarCampo('reserva_nombre_obligatorio', checked)}
            />
          </div>
          <div className="flex items-center justify-between">
            <Label htmlFor="apellidos-obligatorio">Apellidos</Label>
            <Switch
              id="apellidos-obligatorio"
              checked={config.reserva_apellidos_obligatorio}
              onCheckedChange={(checked) => cambiarCampo('reserva_apellidos_obligatorio', checked)}
            />
          </div>
          <div className="flex items-center justify-between">
            <Label htmlFor="email-obligatorio">Email</Label>
            <Switch
              id="email-obligatorio"
              checked={config.reserva_email_obligatorio}
              onCheckedChange={(checked) => cambiarCampo('reserva_email_obligatorio', checked)}
            />
          </div>
          <div className="flex items-center justify-between">
            <Label htmlFor="telefono-obligatorio">Teléfono</Label>
            <Switch
              id="telefono-obligatorio"
              checked={config.reserva_telefono_obligatorio}
              onCheckedChange={(checked) => cambiarCampo('reserva_telefono_obligatorio', checked)}
            />
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Impuestos</CardTitle>
          <CardDescription>Configuración fiscal por defecto</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex flex-col gap-2">
            <Label htmlFor="iva">IVA por defecto (%)</Label>
            <Input
              id="iva"
              type="number"
              min={0}
              max={100}
              step={0.01}
              value={config.iva_por_defecto}
              onChange={(e) => cambiarCampo('iva_por_defecto', Number(e.target.value))}
              className="max-w-32"
            />
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Digitalización con IA</CardTitle>
          <CardDescription>Configura tu API key de Groq para digitalizar facturas, albaranes y tickets automáticamente</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex flex-col gap-2">
            <Label htmlFor="groq-api-key">API Key de Groq</Label>
            <Input
              id="groq-api-key"
              type="password"
              value={config.groq_api_key}
              onChange={(e) => cambiarCampo('groq_api_key', e.target.value)}
              placeholder="gsk_..."
            />
            <p className="text-xs text-muted-foreground">
              Obtén tu API key en <a href="https://console.groq.com/keys" target="_blank" rel="noopener noreferrer" className="underline">console.groq.com/keys</a>. Es gratuita.
            </p>
          </div>
        </CardContent>
      </Card>

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

      {/* [034A-3] URL de Haddock para vista detallada
        * [064A-5] Token API y toggle de sincronización con Haddock POS API */}
      <Card>
        <CardHeader>
          <CardTitle>Plataforma Haddock</CardTitle>
          <CardDescription>Enlace a Haddock y sincronización automática de ventas vía POS API</CardDescription>
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
          <CardTitle>Reseñas y WhatsApp</CardTitle>
          <CardDescription>Configura el review gating para Google y los botones CTA en mensajes de WhatsApp</CardDescription>
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

      {/* [014A-4] Turnos configurables */}
      <Card>
        <CardHeader>
          <CardTitle>Horarios de turnos</CardTitle>
          <CardDescription>Define los rangos horarios para desayuno, comida y cena</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <div className="grid grid-cols-3 gap-4">
            <div className="flex flex-col gap-2">
              <Label className="font-semibold">Desayuno</Label>
              <div className="flex items-center gap-1">
                <Input
                  type="time"
                  value={config.hora_desayuno_inicio.slice(0, 5)}
                  onChange={(e) => cambiarCampo('hora_desayuno_inicio', e.target.value + ':00')}
                  className="max-w-28"
                />
                <span className="text-muted-foreground">-</span>
                <Input
                  type="time"
                  value={config.hora_desayuno_fin.slice(0, 5)}
                  onChange={(e) => cambiarCampo('hora_desayuno_fin', e.target.value + ':00')}
                  className="max-w-28"
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
                  className="max-w-28"
                />
                <span className="text-muted-foreground">-</span>
                <Input
                  type="time"
                  value={config.hora_comida_fin.slice(0, 5)}
                  onChange={(e) => cambiarCampo('hora_comida_fin', e.target.value + ':00')}
                  className="max-w-28"
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
                  className="max-w-28"
                />
                <span className="text-muted-foreground">-</span>
                <Input
                  type="time"
                  value={config.hora_cena_fin.slice(0, 5)}
                  onChange={(e) => cambiarCampo('hora_cena_fin', e.target.value + ':00')}
                  className="max-w-28"
                />
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      <div className="flex items-center gap-4">
        <Button onClick={guardar} disabled={guardando}>
          {guardando ? 'Guardando...' : 'Guardar configuración'}
        </Button>
        {mensaje && (
          <span className={`text-sm ${mensaje.includes('Error') ? 'text-destructive' : 'text-green-600'}`}>
            {mensaje}
          </span>
        )}
      </div>
      </TabsContent>

      <TabsContent value="integraciones" className="mt-4">
        <IntegracionesMarketing />
      </TabsContent>

      <TabsContent value="chatbot" className="mt-4">
        <ConfigChatbot />
      </TabsContent>

      <TabsContent value="bdp" className="mt-4 flex flex-col gap-6">
        <ConfigBdp config={config} cambiarCampo={cambiarCampo} guardar={guardar} guardando={guardando} mensaje={mensaje} />
        <PanelBdpBackup config={config} />
      </TabsContent>
    </Tabs>
  );
}

export default Configuracion;
