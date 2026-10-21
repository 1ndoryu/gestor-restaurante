/* [065A-2] Configuracion de BDP/WebLink REST API.
 * Mantiene credenciales fuera de respuestas publicas y ofrece diagnostico
 * Health + Login + GetVersion para la sesion remota con el PC del restaurante.
 * [147A-F5.6] Secciones de mapeo: tender, order_type, customer_code, poll_interval.
 * [197A-3] Distingue integración, lecturas y permiso puntual de escritura.
 * [237A-3] Desenterrado: catálogo, mapeos técnicos y polling ahora son visibles
 *          directamente. Selector de modo autorización integrado. */

import { useState } from 'react';
import { Shield } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import axios from '@/api/axios-instance';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { toast } from 'sonner';
import type { EstadoConfiguracion } from '../hooks/useConfiguracion';
import type { BdpDiagnosticoResponse } from '../api/generated/gestionRestauranteAPI.schemas';
import { BotonModoAutomatico, ControlesTarjetaModo } from './config-bdp/ControlesModoBdp';
import ConexionBdpCard from './config-bdp/ConexionBdpCard';
import FuncionesBdpCard from './config-bdp/FuncionesBdpCard';
import OperacionesNoApiCard from './config-bdp/OperacionesNoApiCard';
import ParametrosEscrituraBdpCard from './config-bdp/ParametrosEscrituraBdpCard';
import type { BdpSyncDryRunResponse, CambiarCampo } from './config-bdp/tipos';

interface BdpUiState {
  diagnostico: BdpDiagnosticoResponse | null;
  dryRun: BdpSyncDryRunResponse | null;
  diagnosticando: boolean;
  probandoSync: boolean;
}

interface ConfigBdpProps {
  config: EstadoConfiguracion;
  cambiarCampo: CambiarCampo;
  guardar?: () => void;
  guardando?: boolean;
  mensaje?: string;
}

function ConfigBdp({ config, cambiarCampo, guardar, guardando, mensaje }: ConfigBdpProps) {
  const [estadoBdp, setEstadoBdp] = useState<BdpUiState>({
    diagnostico: null,
    dryRun: null,
    diagnosticando: false,
    probandoSync: false,
  });
  const { diagnostico, dryRun, diagnosticando, probandoSync } = estadoBdp;

  async function diagnosticar() {
    setEstadoBdp((actual) => ({ ...actual, diagnosticando: true }));
    try {
      const resp = await axios.get<BdpDiagnosticoResponse>('/api/configuracion/bdp/diagnostico');
      setEstadoBdp((actual) => ({ ...actual, diagnostico: resp.data }));
      if (resp.data.health_ok && resp.data.login_ok) {
        toast.success('BDP conectado', { description: resp.data.mensaje });
      } else {
        toast.warning('BDP pendiente', { description: resp.data.mensaje });
      }
    } catch (err: unknown) {
      const msg = (err as { response?: { data?: { message?: string } } })?.response?.data?.message ?? 'No se pudo diagnosticar BDP';
      toast.error('Error BDP', { description: msg });
    } finally {
      setEstadoBdp((actual) => ({ ...actual, diagnosticando: false }));
    }
  }

  async function probarSincronizacion() {
    setEstadoBdp((actual) => ({ ...actual, probandoSync: true }));
    try {
      const resp = await axios.get<BdpSyncDryRunResponse>('/api/configuracion/bdp/sync-dry-run');
      setEstadoBdp((actual) => ({ ...actual, dryRun: resp.data }));
      if (resp.data.listo_para_sincronizar) {
        toast.success('Sincronización validada', { description: resp.data.mensaje });
      } else {
        toast.warning('Sincronización pendiente', { description: resp.data.mensaje });
      }
    } catch (err: unknown) {
      const msg = (err as { response?: { data?: { message?: string } } })?.response?.data?.message ?? 'No se pudo probar la sincronización BDP';
      toast.error('Error BDP', { description: msg });
    } finally {
      setEstadoBdp((actual) => ({ ...actual, probandoSync: false }));
    }
  }

  return (
    <>
    <ConexionBdpCard
      config={config}
      cambiarCampo={cambiarCampo}
      diagnostico={diagnostico}
      dryRun={dryRun}
      diagnosticando={diagnosticando}
      probandoSync={probandoSync}
      onDiagnosticar={diagnosticar}
      onProbarSincronizacion={probarSincronizacion}
    />

    {/* [237A-3] Información sobre modo de autorización — el selector real vive en PanelBdpBackup */}
    <Card className="mt-4">
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <Shield className="size-4" />
          Modo de operaciones BDP
        </CardTitle>
        <CardDescription>
          Controla cómo la Aplicación Web interactúa con BDP para operaciones de escritura (crear comandas, pagar, facturar, crear clientes).
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="grid gap-3 md:grid-cols-3">
          <div className={`rounded-md border p-3 transition-colors ${(config.bdp_sync_mode || 'read_only') === 'read_only' ? 'border-primary bg-primary/5' : ''}`}>
            <div className="flex items-center gap-2 mb-1">
              {(config.bdp_sync_mode || 'read_only') === 'read_only' && (
                <Badge variant="default">Activo</Badge>
              )}
              <p className="text-sm font-medium">Solo lectura (BDP → Aplicación Web)</p>
            </div>
            <p className="text-xs text-muted-foreground">
              Permite consultas e importaciones. No se puede crear ni modificar nada en BDP. Es el modo seguro por defecto.
            </p>
            <ControlesTarjetaModo config={config} modo="read_only" etiquetaActivar="Volver a Solo lectura" />
          </div>
          <div className={`rounded-md border p-3 transition-colors ${config.bdp_sync_mode === 'unidirectional' ? 'border-amber-500 bg-amber-50 dark:bg-amber-950/20' : ''}`}>
            <div className="flex items-center gap-2 mb-1">
              {config.bdp_sync_mode === 'unidirectional' && (
                <Badge variant="default" className="bg-amber-600">Activo</Badge>
              )}
              <p className="text-sm font-medium">Autorización manual (Aplicación Web → BDP)</p>
            </div>
            <p className="text-xs text-muted-foreground">
              Para cada operación de escritura se requiere confirmación textual y un arming temporal. Después vuelve automáticamente a Solo lectura.
            </p>
            <ControlesTarjetaModo config={config} modo="unidirectional" etiquetaActivar="Activar autorización manual" />
          </div>
          <BotonModoAutomatico config={config} />
        </div>
        <div className="rounded-md border border-dashed p-3">
          <p className="text-sm text-muted-foreground">
            Cada tarjeta permite cambiar directamente a su modo: el flujo incluye la confirmación de destino, alcance, motivo y duración requeridos para cada escritura. El selector <strong>"Permiso de operación"</strong> del panel de <strong>Seguridad, respaldos e historial BDP</strong> (más abajo) ofrece el mismo cambio con idénticas garantías.
          </p>
        </div>
      </CardContent>
    </Card>

    {/* [10AA-3] Mapeo de IVA, modalidad de envío y códigos por defecto de escritura. */}
    <ParametrosEscrituraBdpCard config={config} cambiarCampo={cambiarCampo} />

    <FuncionesBdpCard config={config} cambiarCampo={cambiarCampo} />

    {/* [D32] Responsable y procedimiento de lo que BDP no expone por API. */}
    <OperacionesNoApiCard />

    {/* [167A-1] Botón guardar propio de la pestaña BDP */}
    {guardar && (
      <div className="flex items-center gap-4 mt-4">
        <Button onClick={guardar} disabled={guardando}>
          {guardando ? 'Guardando...' : 'Guardar conexión BDP'}
        </Button>
        {mensaje && (
          <span className={`text-sm ${mensaje.includes('Error') ? 'text-destructive' : 'text-green-600'}`}>
            {mensaje}
          </span>
        )}
      </div>
    )}
    </>
  );
}

export default ConfigBdp;
