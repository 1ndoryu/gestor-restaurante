/* [263A-16] Configuración — reescrita con shadcn Card + Switch + Input.
 * Campos obligatorios al reservar, IVA por defecto, nombre del restaurante.
 * [283A-8] Sección de API key de Groq para digitalización de documentos.
 * [283A-23] Pestañas: General + Integraciones Marketing.
 * [283A-27] Pestaña Chatbot con gestión de API Keys.
 * [283A-39] Card de datos de prueba: botón seed y botón reset. */

import { useState, useEffect, useRef } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { useConfiguracion } from '../hooks/useConfiguracion';
import { useBdpDemoMode } from '../hooks/useBdpDemoMode';
import IntegracionesMarketing from './IntegracionesMarketing';
import ConfigChatbot from './ConfigChatbot';
import ConfigBdp from './ConfigBdp';
import PanelBdpBackup from './PanelBdpBackup';
import { TarjetasNegocio } from './configuracion/TarjetasNegocio';
import { TarjetasOperativas } from './configuracion/TarjetasOperativas';
import { TarjetasIntegraciones } from './configuracion/TarjetasIntegraciones';
import { TarjetaTurnos } from './configuracion/TarjetaTurnos';
import axios from '@/api/axios-instance';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { toast } from 'sonner';

function Configuracion() {
  const { config, cambiarCampo, guardar, mensaje, cargando, guardando } = useConfiguracion();
  const { demoMode, setDemoMode } = useBdpDemoMode();
  const [operandoSeed, setOperandoSeed] = useState(false);
  const location = useLocation();
  const navigate = useNavigate();
  const bdpPanelRef = useRef<HTMLDivElement>(null);
  const [activeTab, setActiveTab] = useState<string>('general');

  /* [C2-2] Navegación desde el navbar: si venimos con state bdpArming o
   * bdpSection='bdp', activamos la pestaña BDP y hacemos scroll al panel. */
  useEffect(() => {
    const state = location.state as { bdpArming?: boolean; bdpSection?: string } | null;
    if (state?.bdpArming || state?.bdpSection === 'bdp') {
      setActiveTab('bdp');
      window.setTimeout(() => {
        bdpPanelRef.current?.scrollIntoView({ behavior: 'smooth', block: 'start' });
      }, 100);
      /* Limpiar state para no reactivar al refrescar */
      navigate('/configuracion', { replace: true, state: null });
    }
  }, [location.state, navigate]);

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
    <Tabs value={activeTab} onValueChange={setActiveTab} className="max-w-5xl">
      <TabsList>
        <TabsTrigger value="general">General</TabsTrigger>
        <TabsTrigger value="integraciones">Integraciones</TabsTrigger>
        <TabsTrigger value="chatbot">Chatbot</TabsTrigger>
        <TabsTrigger value="bdp">BDP</TabsTrigger>
      </TabsList>

      <TabsContent value="general" className="flex flex-col gap-6 mt-4">
        <TarjetasNegocio config={config} cambiarCampo={cambiarCampo} />
        <TarjetasOperativas config={config} cambiarCampo={cambiarCampo} operandoSeed={operandoSeed} ejecutarOperacion={ejecutarOperacion} />
        <TarjetasIntegraciones config={config} cambiarCampo={cambiarCampo} />
        <TarjetaTurnos config={config} cambiarCampo={cambiarCampo} />

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
        {/* [159A-3] Único punto de activación del modo demo BDP. Las páginas
          * BDP (Stock, Compras, Explorador, Historial) ya no llevan toggle en
          * sus toolbars: el badge ámbar rompía el layout y sobrecargaba la UI. */}
        <Card>
          <CardHeader>
            <CardTitle>Modo demo BDP</CardTitle>
            <CardDescription>Muestra datos de ejemplo en las páginas BDP (Stock, Compras, Explorador e Historial) sin tocar datos reales</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex items-center justify-between">
              <Label htmlFor="bdp-demo">Activar datos de ejemplo</Label>
              <Switch
                id="bdp-demo"
                checked={demoMode}
                onCheckedChange={setDemoMode}
              />
            </div>
          </CardContent>
        </Card>
        <ConfigBdp config={config} cambiarCampo={cambiarCampo} guardar={guardar} guardando={guardando} mensaje={mensaje} />
        <div ref={bdpPanelRef}>
          <PanelBdpBackup config={config} />
        </div>
      </TabsContent>
    </Tabs>
  );
}

export default Configuracion;
